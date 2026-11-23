use crate::{
    mail::Origins,
    model::{self, Audience, Error, PAGE_SIZE, Paging},
    staff,
    view::{self, Page, Screen},
};
use axum::{
    Extension, Form, Router,
    extract::{DefaultBodyLimit, Path, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use oteryn_ui::Chrome;
use oteryn_web::session::{BrowserSession, SessionLayer};
use serde::Deserialize;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Web {
    pub pool: PgPool,
    pub mail: Origins,
    reports: oteryn_support::reports::Reports,
}

pub fn configured_router(
    pool: PgPool,
    navigation: &mut oteryn_ui::Registry,
) -> Result<Router, Box<dyn std::error::Error>> {
    navigation.section(
        true,
        "forum",
        "shell-forum",
        "forum",
        oteryn_ui::Group::Community,
        false,
    );
    let origin = std::env::var("OTERYN_PUBLIC_ORIGIN").ok();
    let host = std::env::var("OTERYN_ADMIN_HOST").ok();
    let mail = Origins::configured(origin.as_deref(), host.as_deref());
    let limit = |key, default, max| {
        std::env::var(key)
            .ok()
            .and_then(|value| value.parse::<u8>().ok())
            .filter(|value| (1..=max).contains(value))
            .unwrap_or(default)
    };
    let reports = oteryn_support::reports::Reports::new(
        pool.clone(),
        oteryn_support::reports::Policy {
            enabled: true,
            pending_limit: limit("OTERYN_REPORTS_PENDING_LIMIT", 5, 20),
            unjustified_limit: limit("OTERYN_REPORTS_UNJUSTIFIED_LIMIT", 3, u8::MAX),
        },
    );
    let state = Web {
        pool: pool.clone(),
        mail,
        reports,
    };
    let mut sessions = SessionLayer::new(pool);
    sessions.csrf.trusted_origins = origin.into_iter().collect();
    let public = read_routes("/{lang}/forum")
        .route("/{lang}/forum/profiles/{handle}", get(profile))
        .route("/{lang}/forum/actions/{action}", post(action))
        .route_layer(middleware::from_fn_with_state(
            sessions,
            oteryn_web::session::middleware,
        ))
        .with_state(state.clone());
    Ok(public
        .merge(staff::routes(state, navigation.clone())?)
        .layer(DefaultBodyLimit::max(110 * 1024))
        .layer(middleware::from_fn(private))
        .route(
            "/forum-assets/forum.css",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
                    include_str!("forum.css"),
                )
            }),
        )
        .route(
            "/forum-assets/forum.js",
            get(|| async {
                (
                    [(
                        header::CONTENT_TYPE,
                        "application/javascript; charset=utf-8",
                    )],
                    include_str!("forum.js"),
                )
            }),
        ))
}

pub fn read_routes(base: &str) -> Router<Web> {
    Router::new()
        .route(base, get(index))
        .route(&format!("{base}/boards/{{id}}"), get(board))
        .route(&format!("{base}/threads/{{id}}"), get(thread))
        .route(&format!("{base}/posts/{{id}}"), get(permalink))
        .route(&format!("{base}/preferences"), get(preferences))
}

async fn private(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    response
}

#[derive(Deserialize)]
pub struct Route {
    lang: Option<String>,
    id: Option<Uuid>,
    handle: Option<String>,
    pub action: Option<String>,
}

pub struct Reader {
    pub lang: String,
    pub staff: Option<oteryn_admin::rbac::CheckContext>,
    pub chrome: Chrome,
}

impl Reader {
    pub fn new(
        route: &Route,
        lang: Option<&str>,
        staff: Option<Extension<oteryn_admin::rbac::CheckContext>>,
        chrome: Chrome,
    ) -> Result<Self, StatusCode> {
        let staff = staff.map(|Extension(ctx)| ctx);
        let lang = if staff.is_some() {
            lang.unwrap_or(&chrome.lang)
        } else {
            route.lang.as_deref().ok_or(StatusCode::NOT_FOUND)?
        };
        if !matches!(lang, "en" | "pl") {
            return Err(StatusCode::NOT_FOUND);
        }
        let lang = lang.to_owned();
        Ok(Self {
            lang,
            staff,
            chrome,
        })
    }

    pub async fn audience(
        &self,
        pool: &PgPool,
        session: &BrowserSession,
    ) -> Result<Audience, Error> {
        if let Some(staff) = self.staff {
            return Ok(Audience::Staff(staff.account));
        }
        if let Some(id) = session.account_id().await
            && let Some(hash) = session.identity_hash().await
            && model::live_account(pool, id, &hash).await?
        {
            return Ok(Audience::Player(id));
        }
        Ok(Audience::Guest)
    }

    pub async fn page(
        &self,
        state: &Web,
        session: &BrowserSession,
        audience: Audience,
        screen: Screen,
    ) -> Result<Page, Error> {
        let mut page = Page::new(
            self.chrome.clone(),
            self.lang.clone(),
            audience.staff(),
            screen,
        );
        if let Some(account) = audience.account() {
            page.signed_in = true;
            page.csrf = session.csrf_token().await?;
            let (eligible, handle, email): (bool, Option<String>, bool) = sqlx::query_as(
                "SELECT a.status='active' AND a.email_verified_at IS NOT NULL
                         AND a.created_at<=clock_timestamp()-interval '24 hours',
                        p.public_handle,coalesce(n.email_enabled,false)
                 FROM accounts a LEFT JOIN forum_profiles p ON p.account_id=a.id
                 LEFT JOIN account_notification_prefs n ON n.account_id=a.id AND n.category='forum'
                 WHERE a.id=$1",
            )
            .bind(account)
            .fetch_one(&state.pool)
            .await?;
            page.handle = handle;
            page.email = email;
            page.can_post = eligible && page.handle.is_some();
            page.post_notice = if !eligible {
                "ineligible"
            } else if page.handle.is_none() {
                "handle-required"
            } else {
                ""
            };
            if audience.staff() {
                let grants: Vec<String> = sqlx::query_scalar(
                    "SELECT DISTINCT permission FROM account_roles JOIN role_permissions USING(role_id)
                      WHERE account_id=$1 AND (permission LIKE 'forum.%' OR permission='support.reports.manage')",
                ).bind(account).fetch_all(&state.pool).await?;
                page.privileges = grants
                    .iter()
                    .filter_map(|grant| grant.parse().ok())
                    .collect();
            }
        }
        Ok(page)
    }

    pub fn response(&self, result: Result<Response, Error>) -> Response {
        result.unwrap_or_else(|error| {
            view::failure(&self.chrome, &self.lang, self.staff.is_some(), error)
        })
    }

    pub fn prefix(&self) -> String {
        if self.staff.is_some() {
            "/admin/forum".into()
        } else {
            format!("/{}/forum", self.lang)
        }
    }

    pub fn location(&self, target: &str, params: &[(&str, &str)], anchor: Option<Uuid>) -> String {
        let mut query = url::form_urlencoded::Serializer::new(String::new());
        if self.staff.is_some() {
            query.append_pair("lang", &self.lang);
        }
        query.extend_pairs(params.iter().copied());
        let query = query.finish();
        let query = if query.is_empty() {
            String::new()
        } else {
            format!("?{query}")
        };
        let anchor = anchor.map_or_else(String::new, |id| format!("#post-{id}"));
        format!("{}{target}{query}{anchor}", self.prefix())
    }
}

type Staff = Option<Extension<oteryn_admin::rbac::CheckContext>>;

async fn index(
    State(state): State<Web>,
    Path(route): Path<Route>,
    axum::extract::Query(paging): axum::extract::Query<Paging>,
    Extension(chrome): Extension<Chrome>,
    Extension(session): Extension<BrowserSession>,
    staff: Staff,
) -> Result<Response, StatusCode> {
    let reader = Reader::new(&route, paging.lang.as_deref(), staff, chrome)?;
    Ok(reader.response(
        async {
            let audience = reader.audience(&state.pool, &session).await?;
            let mut page = reader
                .page(&state, &session, audience, Screen::Index)
                .await?;
            page.title = page.tr(if audience.staff() {
                "staff-forum"
            } else {
                "forum"
            });
            page.boards = model::boards(&state.pool, audience).await?;
            Ok(page.render())
        }
        .await,
    ))
}

async fn board(
    State(state): State<Web>,
    Path(route): Path<Route>,
    axum::extract::Query(paging): axum::extract::Query<Paging>,
    Extension(chrome): Extension<Chrome>,
    Extension(session): Extension<BrowserSession>,
    staff: Staff,
) -> Result<Response, StatusCode> {
    let reader = Reader::new(&route, paging.lang.as_deref(), staff, chrome)?;
    Ok(reader.response(
        async {
            let id = route.id.ok_or(Error::NotFound)?;
            let audience = reader.audience(&state.pool, &session).await?;
            let mut page = reader
                .page(&state, &session, audience, Screen::Board)
                .await?;
            let mut tx = state.pool.begin().await?;
            let board = model::board(&mut tx, audience, id).await?;
            let mut topics = model::topics(&mut tx, audience, id, paging.offset()?).await?;
            tx.commit().await?;
            page.next = paging.has_next(topics.len());
            topics.truncate(PAGE_SIZE as usize);
            page.topics = topics;
            page.page = paging.page;
            page.title = board.name(&page.lang).into();
            page.board = Some(board);
            Ok(page.render())
        }
        .await,
    ))
}

async fn thread(
    State(state): State<Web>,
    Path(route): Path<Route>,
    axum::extract::Query(paging): axum::extract::Query<Paging>,
    Extension(chrome): Extension<Chrome>,
    Extension(session): Extension<BrowserSession>,
    staff: Staff,
) -> Result<Response, StatusCode> {
    let reader = Reader::new(&route, paging.lang.as_deref(), staff, chrome)?;
    Ok(reader.response(async {
        let id = route.id.ok_or(Error::NotFound)?;
        let audience = reader.audience(&state.pool, &session).await?;
        let mut page = reader.page(&state, &session, audience, Screen::Thread).await?;
        if audience.staff() {
            page.boards = model::boards(&state.pool, audience).await?;
        }
        let mut tx = state.pool.begin().await?;
        let (thread, board) = model::thread(&mut tx, audience, id, false).await?;
        let mut posts = model::posts(&mut tx, audience, id, paging.offset()?).await?;
        page.next = paging.has_next(posts.len());
        posts.truncate(PAGE_SIZE as usize);
        if let Some(account) = audience.account() {
            page.subscribed = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM forum_subscriptions WHERE thread_id=$1 AND account_id=$2)",
            ).bind(id).bind(account).fetch_one(&mut *tx).await?;
            if !audience.staff() {
                page.seen = posts.last().map(|post| post.id);
                if let Some(report) = paging.report {
                    page.reported = sqlx::query_scalar(
                        "SELECT EXISTS (SELECT 1 FROM player_reports r JOIN forum_posts p ON p.id=r.forum_post_id
                          WHERE r.id=$1 AND r.reporter_account_id=$2 AND p.thread_id=$3)",
                    ).bind(report).bind(account).bind(id).fetch_one(&mut *tx).await?;
                }
            }
            if let Some(quote) = paging.quote {
                page.quote = sqlx::query_as(
                    "SELECT id,left(body,400) FROM forum_posts WHERE id=$1 AND thread_id=$2 AND deleted_at IS NULL",
                ).bind(quote).bind(id).fetch_optional(&mut *tx).await?;
            }
        }
        tx.commit().await?;
        page.mail_available = state.mail.available(&board.visibility);
        page.posts = posts.into_iter().map(Into::into).collect();
        page.page = paging.page;
        page.title = thread.title.clone();
        page.board = Some(board);
        page.thread = Some(thread);
        Ok(page.render())
    }.await))
}

async fn permalink(
    State(state): State<Web>,
    Path(route): Path<Route>,
    axum::extract::Query(paging): axum::extract::Query<Paging>,
    Extension(chrome): Extension<Chrome>,
    Extension(session): Extension<BrowserSession>,
    staff: Staff,
) -> Result<Response, StatusCode> {
    let reader = Reader::new(&route, paging.lang.as_deref(), staff, chrome)?;
    Ok(reader.response(
        async {
            let post = route.id.ok_or(Error::NotFound)?;
            let audience = reader.audience(&state.pool, &session).await?;
            let mut tx = state.pool.begin().await?;
            let id: Uuid = sqlx::query_scalar("SELECT thread_id FROM forum_posts WHERE id=$1")
                .bind(post)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or(Error::NotFound)?;
            model::thread(&mut tx, audience, id, false).await?;
            let page = model::permalink(&mut tx, audience, id, post).await?;
            tx.commit().await?;
            Ok(Redirect::to(&reader.location(
                &format!("/threads/{id}"),
                &[("page", &page.to_string())],
                Some(post),
            ))
            .into_response())
        }
        .await,
    ))
}

async fn preferences(
    State(state): State<Web>,
    Path(route): Path<Route>,
    axum::extract::Query(paging): axum::extract::Query<Paging>,
    Extension(chrome): Extension<Chrome>,
    Extension(session): Extension<BrowserSession>,
    staff: Staff,
) -> Result<Response, StatusCode> {
    let reader = Reader::new(&route, paging.lang.as_deref(), staff, chrome)?;
    Ok(reader.response(
        async {
            let audience = reader.audience(&state.pool, &session).await?;
            audience.account().ok_or(Error::SignIn)?;
            let mut page = reader
                .page(&state, &session, audience, Screen::Preferences)
                .await?;
            page.title = page.tr("preferences");
            Ok(page.render())
        }
        .await,
    ))
}

async fn profile(
    State(state): State<Web>,
    Path(route): Path<Route>,
    Extension(chrome): Extension<Chrome>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    let reader = Reader::new(&route, None, None, chrome)?;
    Ok(reader.response(async {
        let handle = model::handle(route.handle.as_deref().ok_or(Error::NotFound)?)
            .map_err(|_| Error::NotFound)?;
        let audience = reader.audience(&state.pool, &session).await?;
        let mut page = reader.page(&state, &session, audience, Screen::Profile).await?;
        let name: String = sqlx::query_scalar(
            "SELECT p.public_handle FROM forum_profiles p JOIN accounts a ON a.id=p.account_id
              WHERE lower(p.public_handle)=lower($1) AND a.status<>'closed'",
        ).bind(handle).fetch_optional(&state.pool).await?.ok_or(Error::NotFound)?;
        page.profile_posts = sqlx::query_as(
            "SELECT p.id,t.title FROM forum_posts p JOIN forum_threads t ON t.id=p.thread_id
             JOIN forum_boards b ON b.id=t.board_id JOIN forum_profiles n ON n.account_id=p.author_id
             JOIN accounts a ON a.id=p.author_id
             WHERE n.public_handle=$1 AND a.status<>'closed' AND p.deleted_at IS NULL
               AND t.deleted_at IS NULL AND b.visibility='public'
             ORDER BY p.created_at DESC LIMIT 10",
        ).bind(&name).fetch_all(&state.pool).await?;
        page.title = name;
        Ok(page.render())
    }.await))
}

#[derive(Deserialize)]
pub struct Input {
    pub id: Option<Uuid>,
    pub post_id: Option<Uuid>,
    pub quote: Option<Uuid>,
    pub destination: Option<Uuid>,
    pub version: Option<i64>,
    pub enabled: Option<bool>,
    pub page: Option<u32>,
    pub name_en: Option<String>,
    pub name_pl: Option<String>,
    pub position: Option<i32>,
    pub visibility: Option<String>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub handle: Option<String>,
    pub reason: Option<String>,
    pub category: Option<String>,
}

impl Input {
    pub fn id(&self) -> Result<Uuid, Error> {
        self.id.ok_or(Error::Invalid)
    }
    pub fn post(&self) -> Result<Uuid, Error> {
        self.post_id.ok_or(Error::Invalid)
    }
    pub fn version(&self) -> Result<i64, Error> {
        self.version.ok_or(Error::Invalid)
    }
    pub fn enabled(&self) -> Result<bool, Error> {
        self.enabled.ok_or(Error::Invalid)
    }
    pub fn page(&self) -> Result<String, Error> {
        Ok(model::page_number(self.page.unwrap_or(1))?.to_string())
    }
    pub fn body(&self) -> &str {
        self.body.as_deref().unwrap_or_default()
    }
    pub fn title(&self) -> &str {
        self.title.as_deref().unwrap_or_default()
    }
    pub fn handle(&self) -> &str {
        self.handle.as_deref().unwrap_or_default()
    }
}

async fn action(
    State(state): State<Web>,
    Path(route): Path<Route>,
    Extension(chrome): Extension<Chrome>,
    Extension(session): Extension<BrowserSession>,
    headers: HeaderMap,
    Form(input): Form<Input>,
) -> Result<Response, StatusCode> {
    let reader = Reader::new(&route, None, None, chrome)?;
    let action = route.action.as_deref().unwrap_or_default();
    let result = mutate(&state, &reader, &session, action, input).await;
    if action == "read" && result.is_ok() && headers.contains_key("x-forum-read") {
        Ok(StatusCode::NO_CONTENT.into_response())
    } else {
        Ok(reader.response(result))
    }
}

async fn mutate(
    state: &Web,
    reader: &Reader,
    session: &BrowserSession,
    action: &str,
    input: Input,
) -> Result<Response, Error> {
    let account = session.account_id().await.ok_or(Error::SignIn)?;
    let hash = session.identity_hash().await.ok_or(Error::SignIn)?;
    let actor = Audience::Player(account);
    if matches!(
        action,
        "name" | "email" | "new-thread" | "reply" | "edit" | "subscribe" | "report"
    ) {
        if !model::live_account(&state.pool, account, &hash).await? {
            return Err(Error::SignIn);
        }
        model::charge_mutation(&state.pool, account).await?;
    }
    let mut tx = state.pool.begin().await?;
    model::player_account(&mut tx, account, &hash, action == "edit").await?;
    let location = match action {
        "name" => {
            save_handle(&mut tx, account, input.handle()).await?;
            reader.location("/preferences", &[], None)
        }
        "email" => {
            save_email(&mut tx, account, input.enabled()?).await?;
            reader.location("/preferences", &[], None)
        }
        "new-thread" => {
            model::posting_account(&mut tx, account).await?;
            let board = model::board(&mut tx, actor, input.id()?).await?;
            let thread = model::create_thread(&mut tx, board.id, account, input.title()).await?;
            model::add_post(&mut tx, thread, account, input.body(), None).await?;
            reader.location(&format!("/threads/{thread}"), &[], None)
        }
        "reply" | "edit" | "subscribe" | "read" | "report" => {
            let id = input.id()?;
            let (thread, board) = model::thread(&mut tx, actor, id, action == "reply").await?;
            let path = format!("/threads/{id}");
            match action {
                "reply" => {
                    if thread.locked {
                        return Err(Error::Locked);
                    }
                    model::posting_account(&mut tx, account).await?;
                    let (post, created) =
                        model::add_post(&mut tx, id, account, input.body(), input.quote).await?;
                    state
                        .mail
                        .queue(&mut tx, id, account, created, &board.visibility)
                        .await?;
                    reader.location(&format!("/posts/{post}"), &[], None)
                }
                "edit" => {
                    if thread.locked {
                        return Err(Error::Locked);
                    }
                    let post = input.post()?;
                    model::edit_own(&mut tx, id, post, account, input.body(), input.version()?)
                        .await?;
                    reader.location(&format!("/posts/{post}"), &[], None)
                }
                "subscribe" => {
                    let enabled = input.enabled()?;
                    let page = input.page()?;
                    subscription(
                        &mut tx,
                        &state.mail,
                        &board.visibility,
                        account,
                        id,
                        enabled,
                    )
                    .await?;
                    reader.location(&path, &[("page", &page)], None)
                }
                "read" => {
                    let page = input.page()?;
                    let position: i64 = sqlx::query_scalar(
                        "SELECT position FROM forum_posts WHERE id=$1 AND thread_id=$2 AND deleted_at IS NULL",
                    ).bind(input.post()?).bind(id).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
                    model::mark_read(&mut tx, account, id, position).await?;
                    reader.location(&path, &[("page", &page)], None)
                }
                "report" => {
                    let post = input.post()?;
                    let page = model::permalink(&mut tx, actor, id, post)
                        .await?
                        .to_string();
                    let result = state
                        .reports
                        .submit_forum_in(
                            &mut tx,
                            account,
                            post,
                            input.category.unwrap_or_default(),
                            input.reason.unwrap_or_default(),
                        )
                        .await;
                    let report = match result {
                        Ok(report) => report,
                        Err(oteryn_support::reports::Error::Storage(error)) => {
                            return Err(error.into());
                        }
                        Err(error) => {
                            tx.commit().await?;
                            return Err(error.into());
                        }
                    };
                    reader.location(
                        &path,
                        &[("page", &page), ("report", &report.id.to_string())],
                        Some(post),
                    )
                }
                _ => return Err(Error::NotFound),
            }
        }
        _ => return Err(Error::NotFound),
    };
    tx.commit().await?;
    Ok(Redirect::to(&location).into_response())
}

pub async fn save_handle(db: &mut PgConnection, account: Uuid, value: &str) -> Result<(), Error> {
    let value = model::handle(value)?;
    let result = sqlx::query(
        "INSERT INTO forum_profiles(account_id,public_handle) VALUES($1,$2)
         ON CONFLICT(account_id) DO UPDATE SET public_handle=excluded.public_handle",
    )
    .bind(account)
    .bind(value)
    .execute(db)
    .await;
    match result {
        Err(sqlx::Error::Database(error))
            if error.constraint() == Some("forum_profiles_public_handle") =>
        {
            Err(Error::HandleTaken)
        }
        other => {
            other?;
            Ok(())
        }
    }
}

async fn require_verified(db: &mut PgConnection, account: Uuid) -> Result<(), Error> {
    let verified: bool = sqlx::query_scalar(
        "SELECT status='active' AND email_verified_at IS NOT NULL FROM accounts WHERE id=$1",
    )
    .bind(account)
    .fetch_one(db)
    .await?;
    if verified {
        Ok(())
    } else {
        Err(Error::Ineligible)
    }
}

pub async fn save_email(db: &mut PgConnection, account: Uuid, enabled: bool) -> Result<(), Error> {
    if enabled {
        require_verified(db, account).await?;
    }
    sqlx::query(
        "INSERT INTO account_notification_prefs(account_id,category,email_enabled,portal_enabled)
         VALUES($1,'forum',$2,false)
         ON CONFLICT(account_id,category) DO UPDATE SET email_enabled=$2,updated_at=clock_timestamp()
         WHERE account_notification_prefs.email_enabled IS DISTINCT FROM excluded.email_enabled",
    ).bind(account).bind(enabled).execute(db).await?;
    Ok(())
}

pub async fn subscription(
    db: &mut PgConnection,
    mail: &Origins,
    visibility: &str,
    account: Uuid,
    thread: Uuid,
    enabled: bool,
) -> Result<bool, Error> {
    if !enabled {
        return Ok(sqlx::query(
            "DELETE FROM forum_subscriptions WHERE thread_id=$1 AND account_id=$2",
        )
        .bind(thread)
        .bind(account)
        .execute(db)
        .await?
        .rows_affected()
            == 1);
    }
    if !mail.available(visibility) {
        return Err(Error::Forbidden);
    }
    require_verified(db, account).await?;
    let (subscribed, limited): (bool, bool) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM forum_subscriptions WHERE account_id=$1 AND thread_id=$2),
                (SELECT count(*) FROM forum_subscriptions WHERE account_id=$1)>=100",
    )
    .bind(account)
    .bind(thread)
    .fetch_one(&mut *db)
    .await?;
    if subscribed {
        return Ok(false);
    }
    if limited {
        return Err(Error::Limit);
    }
    sqlx::query("INSERT INTO forum_subscriptions(thread_id,account_id) VALUES ($1,$2)")
        .bind(thread)
        .bind(account)
        .execute(&mut *db)
        .await?;
    sqlx::query(
        "INSERT INTO account_notification_prefs(account_id,category,email_enabled,portal_enabled)
         VALUES($1,'forum',true,false) ON CONFLICT(account_id,category) DO NOTHING",
    )
    .bind(account)
    .execute(db)
    .await?;
    Ok(true)
}
