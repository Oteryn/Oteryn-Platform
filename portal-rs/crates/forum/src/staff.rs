use crate::{
    model::{self, Audience, Board, Error, Thread},
    player::{self, Input, Reader, Route, Web},
};
use axum::{
    Extension, Form, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::post,
};
use oteryn_admin::{
    audit::{self, AuditContext, Entry},
    rbac::{self, MutationAuthorization, Permission},
};
use oteryn_ui::Chrome;
use serde_json::{Value, json};
use sqlx::{PgConnection, Postgres, Transaction};
use uuid::Uuid;

pub fn routes(state: Web) -> Result<Router, Box<dyn std::error::Error>> {
    oteryn_admin::web::configured_guard(
        player::read_routes("/admin/forum")
            .route("/admin/forum/actions/{action}", post(action))
            .with_state(state.clone()),
        state.pool,
        Permission::ForumModerator,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    CreateBoard,
    EditBoard,
    CreateThread,
    Reply,
    Name,
    Email,
    Subscribe,
    Lock,
    Pin,
    Move,
    DeleteThread,
    RestoreThread,
    EditThread,
    DeletePost,
    RestorePost,
    EditPost,
}

impl Action {
    fn parse(value: &str) -> Result<Self, Error> {
        Ok(match value {
            "create-board" => Self::CreateBoard,
            "edit-board" => Self::EditBoard,
            "new-thread" => Self::CreateThread,
            "reply" => Self::Reply,
            "name" => Self::Name,
            "email" => Self::Email,
            "subscribe" => Self::Subscribe,
            "lock" => Self::Lock,
            "pin" => Self::Pin,
            "move" => Self::Move,
            "thread-delete" => Self::DeleteThread,
            "thread-restore" => Self::RestoreThread,
            "thread-edit" => Self::EditThread,
            "post-delete" => Self::DeletePost,
            "post-restore" => Self::RestorePost,
            "post-edit" => Self::EditPost,
            _ => return Err(Error::NotFound),
        })
    }

    fn permission(self) -> Permission {
        match self {
            Self::CreateBoard => Permission::ForumBoardCreate,
            Self::EditBoard => Permission::ForumBoardEdit,
            Self::CreateThread => Permission::ForumThreadCreate,
            Self::Reply => Permission::ForumReply,
            Self::Name => Permission::ForumProfile,
            Self::Email | Self::Subscribe => Permission::ForumSubscribe,
            Self::Lock => Permission::ForumLock,
            Self::Pin => Permission::ForumPin,
            Self::Move => Permission::ForumMove,
            Self::DeleteThread => Permission::ForumThreadDelete,
            Self::RestoreThread => Permission::ForumThreadRestore,
            Self::EditThread => Permission::ForumThreadEdit,
            Self::DeletePost => Permission::ForumPostDelete,
            Self::RestorePost => Permission::ForumPostRestore,
            Self::EditPost => Permission::ForumPostEdit,
        }
    }

    fn resource(self, input: &Input) -> Result<Resource, Error> {
        Ok(match self {
            Self::CreateBoard | Self::Name | Self::Email => Resource::None,
            Self::EditBoard => Resource::Board(input.id()?, true),
            Self::CreateThread => Resource::Board(input.id()?, false),
            Self::Move => Resource::Move(input.id()?, input.destination.ok_or(Error::Invalid)?),
            _ => Resource::Thread(input.id()?),
        })
    }
}

#[derive(Clone, Copy)]
enum Resource {
    None,
    Board(Uuid, bool),
    Thread(Uuid),
    Move(Uuid, Uuid),
}

impl Resource {
    async fn lock(self, db: &mut PgConnection, audience: Audience) -> Result<(), Error> {
        match self {
            Self::None => Ok(()),
            Self::Board(id, true) => {
                board_for_update(db, id).await?;
                Ok(())
            }
            Self::Board(id, false) => {
                model::board(db, audience, id).await?;
                Ok(())
            }
            Self::Thread(id) => {
                model::thread(db, audience, id, true).await?;
                Ok(())
            }
            Self::Move(id, destination) => {
                model::thread(db, audience, id, true).await?;
                model::board(db, audience, destination).await?;
                Ok(())
            }
        }
    }
}

async fn action(
    State(state): State<Web>,
    Path(route): Path<Route>,
    Query(paging): Query<model::Paging>,
    Extension(chrome): Extension<Chrome>,
    Extension(context): Extension<rbac::CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    Form(input): Form<Input>,
) -> Result<Response, StatusCode> {
    let reader = Reader::new(
        &route,
        paging.lang.as_deref(),
        Some(Extension(context)),
        chrome,
    )?;
    Ok(reader.response(
        async {
            let action = Action::parse(route.action.as_deref().unwrap_or_default())?;
            let audience = Audience::Staff(context.account);
            let resource = action.resource(&input)?;
            let mut tx = state.pool.begin().await?;
            rbac::authorize_mutation_with_resources_domain(
                &mut tx,
                &[context.account],
                move |db| Box::pin(resource.lock(db, audience)),
                &auth,
                context,
                &[
                    Permission::Access,
                    Permission::ForumModerator,
                    action.permission(),
                ],
            )
            .await?;
            let change = apply(&mut tx, &state, &reader, action, &input, context.account).await?;
            audit::append(
                &mut tx,
                Entry {
                    context: AuditContext {
                        actor: Some(context.account),
                        request_id: Uuid::now_v7(),
                    },
                    action: action.permission().as_str(),
                    target: &change.target.to_string(),
                    before: change.before,
                    after: change.after,
                },
            )
            .await?;
            tx.commit().await?;
            Ok(Redirect::to(&change.location).into_response())
        }
        .await,
    ))
}

struct Change {
    target: Uuid,
    before: Value,
    after: Value,
    location: String,
}

async fn board_for_update(db: &mut PgConnection, id: Uuid) -> Result<Board, Error> {
    sqlx::query_as("SELECT * FROM forum_boards WHERE id=$1 FOR UPDATE")
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or(Error::NotFound)
}

async fn save_board(
    db: &mut PgConnection,
    input: &Input,
    edit: bool,
) -> Result<(Option<Board>, Board), Error> {
    let pl = model::text(input.name_pl.as_deref().unwrap_or_default(), 100, false)?;
    let en = model::text(input.name_en.as_deref().unwrap_or_default(), 100, false)?;
    let order = input
        .position
        .filter(|v| (0..=10_000).contains(v))
        .ok_or(Error::Invalid)?;
    let visibility = input
        .visibility
        .as_deref()
        .filter(|v| matches!(*v, "public" | "signed-in" | "staff"))
        .ok_or(Error::Invalid)?;
    if edit {
        let before = board_for_update(db, input.id()?).await?;
        if before.version != input.version()? {
            return Err(Error::Conflict);
        }
        let board = sqlx::query_as(
            "UPDATE forum_boards SET name_pl=$1,name_en=$2,position=$3,visibility=$4,version=version+1
              WHERE id=$5 RETURNING *",
        ).bind(pl).bind(en).bind(order).bind(visibility).bind(before.id).fetch_one(db).await?;
        Ok((Some(before), board))
    } else {
        let board = sqlx::query_as(
            "INSERT INTO forum_boards(name_pl,name_en,position,visibility)
                         VALUES($1,$2,$3,$4) RETURNING *",
        )
        .bind(pl)
        .bind(en)
        .bind(order)
        .bind(visibility)
        .fetch_one(db)
        .await?;
        Ok((None, board))
    }
}

async fn apply(
    tx: &mut Transaction<'_, Postgres>,
    state: &Web,
    reader: &Reader,
    action: Action,
    input: &Input,
    account: Uuid,
) -> Result<Change, Error> {
    let audience = Audience::Staff(account);
    match action {
        Action::CreateBoard | Action::EditBoard => {
            let (before, board) = save_board(tx, input, action == Action::EditBoard).await?;
            Ok(Change {
                target: board.id,
                before: json!(before),
                after: json!(board),
                location: reader.location(&format!("/boards/{}", board.id), &[], None),
            })
        }
        Action::Name | Action::Email => {
            let (before, after) = if action == Action::Name {
                let before: Option<String> = sqlx::query_scalar(
                    "SELECT public_handle FROM forum_profiles WHERE account_id=$1",
                )
                .bind(account)
                .fetch_optional(&mut **tx)
                .await?;
                player::save_handle(tx, account, input.handle()).await?;
                (
                    json!({"name":before}),
                    json!({"name":input.handle().trim()}),
                )
            } else {
                let before: Option<bool> = sqlx::query_scalar(
                    "SELECT email_enabled FROM account_notification_prefs WHERE account_id=$1 AND category='forum'",
                ).bind(account).fetch_optional(&mut **tx).await?;
                let enabled = input.enabled()?;
                player::save_email(tx, account, enabled).await?;
                (json!({"email":before}), json!({"email":enabled}))
            };
            Ok(Change {
                target: account,
                before,
                after,
                location: reader.location("/preferences", &[], None),
            })
        }
        Action::CreateThread => {
            model::posting_account(tx, account).await?;
            let id = model::create_thread(tx, input.id()?, account, input.title()).await?;
            let (post, _) = model::add_post(tx, id, account, input.body(), None).await?;
            Ok(Change {
                target: id,
                before: Value::Null,
                after: json!({"board":input.id()?, "post":post}),
                location: reader.location(&format!("/threads/{id}"), &[], None),
            })
        }
        Action::Reply | Action::Subscribe => {
            let (thread, board) = model::thread(tx, audience, input.id()?, true).await?;
            if thread.deleted_at.is_some() {
                return Err(Error::NotFound);
            }
            let id = thread.id;
            if action == Action::Reply {
                if thread.locked {
                    return Err(Error::Locked);
                }
                model::posting_account(tx, account).await?;
                let (post, created) =
                    model::add_post(tx, id, account, input.body(), input.quote).await?;
                state
                    .mail
                    .queue(tx, id, account, created, &board.visibility)
                    .await?;
                Ok(Change {
                    target: post,
                    before: Value::Null,
                    after: json!({"thread":id}),
                    location: reader.location(&format!("/posts/{post}"), &[], None),
                })
            } else {
                let enabled = input.enabled()?;
                let changed =
                    player::subscription(tx, &state.mail, &board.visibility, account, id, enabled)
                        .await?;
                if !changed {
                    return Err(Error::Conflict);
                }
                Ok(Change {
                    target: id,
                    before: json!({"subscribed":!enabled}),
                    after: json!({"subscribed":enabled}),
                    location: reader.location(&format!("/threads/{id}"), &[], None),
                })
            }
        }
        Action::DeletePost | Action::RestorePost | Action::EditPost => {
            moderate_post(tx, reader, action, input).await
        }
        Action::Lock
        | Action::Pin
        | Action::Move
        | Action::DeleteThread
        | Action::RestoreThread
        | Action::EditThread => {
            let (thread, _) = model::thread(tx, audience, input.id()?, true).await?;
            moderate_thread(tx, reader, action, input, thread).await
        }
    }
}

fn reason(input: &Input) -> Result<&str, Error> {
    model::text(input.reason.as_deref().unwrap_or_default(), 400, false)
}

async fn moderate_thread(
    db: &mut PgConnection,
    reader: &Reader,
    action: Action,
    input: &Input,
    thread: Thread,
) -> Result<Change, Error> {
    let reason = reason(input)?;
    if thread.version != input.version()? {
        return Err(Error::Conflict);
    }
    let id = thread.id;
    let (board, locked, pinned, removed, title) = match action {
        Action::Lock => (
            thread.board_id,
            !thread.locked,
            thread.pinned,
            thread.deleted_at.is_some(),
            thread.title.as_str(),
        ),
        Action::Pin => (
            thread.board_id,
            thread.locked,
            !thread.pinned,
            thread.deleted_at.is_some(),
            thread.title.as_str(),
        ),
        Action::Move => (
            input.destination.ok_or(Error::Invalid)?,
            thread.locked,
            thread.pinned,
            thread.deleted_at.is_some(),
            thread.title.as_str(),
        ),
        Action::DeleteThread if thread.deleted_at.is_none() => (
            thread.board_id,
            thread.locked,
            thread.pinned,
            true,
            thread.title.as_str(),
        ),
        Action::RestoreThread if thread.deleted_at.is_some() => (
            thread.board_id,
            thread.locked,
            thread.pinned,
            false,
            thread.title.as_str(),
        ),
        Action::EditThread => (
            thread.board_id,
            thread.locked,
            thread.pinned,
            thread.deleted_at.is_some(),
            model::text(input.title(), 160, false)?,
        ),
        _ => return Err(Error::Conflict),
    };
    if (board, locked, pinned, removed, title)
        == (
            thread.board_id,
            thread.locked,
            thread.pinned,
            thread.deleted_at.is_some(),
            thread.title.as_str(),
        )
    {
        return Err(Error::Conflict);
    }
    let changed: Thread = sqlx::query_as(
        "UPDATE forum_threads SET board_id=$2,locked=$3,pinned=$4,
           deleted_at=CASE WHEN $5 THEN coalesce(deleted_at,clock_timestamp()) ELSE NULL END,
           title=$6,version=version+1 WHERE id=$1 RETURNING *",
    )
    .bind(id)
    .bind(board)
    .bind(locked)
    .bind(pinned)
    .bind(removed)
    .bind(title)
    .fetch_one(db)
    .await?;
    Ok(Change {
        target: id,
        before: json!(thread),
        after: json!({"thread":changed,"reason":reason}),
        location: reader.location(&format!("/threads/{id}"), &[], None),
    })
}

async fn moderate_post(
    db: &mut PgConnection,
    reader: &Reader,
    action: Action,
    input: &Input,
) -> Result<Change, Error> {
    let reason = reason(input)?;
    let thread = input.id()?;
    let post = input.post()?;
    let (removed, version): (bool, i64) = sqlx::query_as(
        "SELECT deleted_at IS NOT NULL,version FROM forum_posts WHERE thread_id=$1 AND id=$2 FOR UPDATE",
    ).bind(thread).bind(post).fetch_optional(&mut *db).await?.ok_or(Error::NotFound)?;
    if version != input.version()? {
        return Err(Error::Conflict);
    }
    let (next_removed, body) = match action {
        Action::DeletePost if !removed => (true, None),
        Action::RestorePost if removed => (false, None),
        Action::EditPost => (removed, Some(model::text(input.body(), 8000, true)?)),
        _ => return Err(Error::Conflict),
    };
    sqlx::query(
        "UPDATE forum_posts SET deleted_at=CASE WHEN $2 THEN coalesce(deleted_at,clock_timestamp()) ELSE NULL END,
           body=coalesce($3,body), edited_at=CASE WHEN $3::text IS NOT NULL THEN clock_timestamp() ELSE edited_at END,
           version=version+1 WHERE id=$1",
    ).bind(post).bind(next_removed).bind(body).execute(db).await?;
    Ok(Change {
        target: post,
        before: json!({"removed":removed,"version":version}),
        after: json!({"removed":next_removed,"version":version+1,"reason":reason,"edited":body.is_some()}),
        location: reader.location(&format!("/posts/{post}"), &[], None),
    })
}
