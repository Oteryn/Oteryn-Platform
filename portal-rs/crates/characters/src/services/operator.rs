use super::{Error, PgPool, Uuid, browser};
use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use sqlx::PgConnection;

#[derive(Template)]
#[template(path = "services/operator.html", config = "src/services/askama.toml")]
struct Page<'a> {
    lang: &'a str,
    counts: [(&'static str, i64); 5],
}

impl Page<'_> {
    fn t(&self, key: &str) -> String {
        browser::text(self.lang, key)
    }
}

/// Same permission for the navigation link and the destination.
pub(crate) async fn allowed(db: &mut PgConnection, actor: Uuid) -> Result<bool, sqlx::Error> {
    Ok(sqlx::query_scalar(
        "SELECT a.status='active' AND (
             SELECT count(DISTINCT rp.permission)=2
             FROM account_roles ar JOIN role_permissions rp ON rp.role_id=ar.role_id
             WHERE ar.account_id=a.id AND rp.permission IN ('admin.access', 'game.player.manage')
         ) FROM accounts a WHERE a.id=$1",
    )
    .bind(actor)
    .fetch_optional(db)
    .await?
    .unwrap_or(false))
}

/// Aggregate operational state only. Player identities and receipt bodies stay private.
pub async fn operator_status(
    pool: &PgPool,
    actor: Option<Uuid>,
    grant: Option<String>,
    lang: &str,
) -> Response {
    if !matches!(lang, "en" | "pl") {
        return browser::error("en", Error::BadInput);
    }
    let Some(actor) = actor else {
        return browser::private(Redirect::to(&format!("/{lang}/login")).into_response());
    };
    let result = async {
        let mut tx = pool.begin().await?;
        // MFA, permissions and counts use the same snapshot.
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await?;
        if !allowed(&mut tx, actor).await? {
            return Err(Error::Denied);
        }
        let grant = grant.ok_or(Error::Denied)?;
        let mfa = oteryn_identity::mfa::Mfa::configured(pool.clone()).map_err(|_| Error::Denied)?;
        if !mfa.satisfied_in_transaction(&mut tx, actor, &grant).await? {
            return Err(Error::Denied);
        }
        let (pending, overdue, reopened, conflicts, credits): (i64, i64, i64, i64, i64) =
            sqlx::query_as(
                "SELECT
                   count(*) FILTER (WHERE c.state='PENDING'),
                   count(*) FILTER (WHERE c.state='PENDING' AND c.updated_at<=now()-interval '300 seconds'),
                   count(*) FILTER (WHERE EXISTS (
                       SELECT 1 FROM character_restore_reopens r
                       WHERE r.operation_id=c.id AND r.new_receipt_id IS NULL
                   )),
                   count(*) FILTER (WHERE EXISTS (
                       SELECT 1 FROM character_restore_queue q
                       WHERE q.operation_id=c.id AND q.resolved_at IS NULL
                   )),
                   count(*) FILTER (WHERE o.financial_state='RECOVERY_REQUIRED' AND o.credit_used_by IS NULL)
                 FROM character_service_orders o JOIN character_commands c ON c.id=o.operation_id",
            )
            .fetch_one(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(Page {
            lang,
            counts: [
                ("operator-pending", pending),
                ("operator-overdue", overdue),
                ("operator-reopened", reopened),
                ("operator-review", conflicts),
                ("operator-credits", credits),
            ],
        })
    }
    .await;
    match result {
        Ok(page) => match page.render() {
            Ok(html) => browser::private(Html(html).into_response()),
            Err(_) => browser::private(StatusCode::SERVICE_UNAVAILABLE.into_response()),
        },
        Err(Error::Denied) => browser::private(
            (
                StatusCode::FORBIDDEN,
                browser::text(lang, "operator-denied"),
            )
                .into_response(),
        ),
        Err(error) => browser::error(lang, error),
    }
}
