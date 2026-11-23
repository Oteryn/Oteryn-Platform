//! Bounded season publication and release status on the configured staff host.
use super::{repository::publish_definition, web, *};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_identity::mfa::{Mfa, SESSION_GRANT_KEY};
use oteryn_web::{
    csrf::CsrfConfig,
    session::{BrowserSession, SessionLayer},
};
use serde::Deserialize;
use sqlx::{PgConnection, PgPool};

const MAX_FORM_BYTES: usize = 256 * 1024;
const ACCESS: [&str; 2] = ["admin.access", "events.publish"];

#[derive(Clone)]
pub(super) struct Operator {
    mfa: Mfa,
    host: String,
    flags: Flags,
}

impl Operator {
    pub(super) fn configured(pool: PgPool, flags: Flags) -> Option<Self> {
        if !flags.enabled || std::env::var("OTERYN_ADMIN_PANEL").as_deref() != Ok("true") {
            return None;
        }
        let host = std::env::var("OTERYN_ADMIN_HOST").ok()?;
        host.parse::<axum::http::uri::Authority>().ok()?;
        Some(Self {
            mfa: Mfa::configured(pool).ok()?,
            host,
            flags,
        })
    }

    pub(super) fn router(&self, mut csrf: CsrfConfig) -> Router {
        csrf.trusted_origins
            .push(format!("https://{}", self.host.to_ascii_lowercase()));
        let mut sessions = SessionLayer::new(self.mfa.pool());
        sessions.csrf = csrf;
        Router::new()
            .route("/{lang}/pass/operator", get(status).post(publish))
            .layer(axum::middleware::from_fn_with_state(
                sessions,
                oteryn_web::session::middleware,
            ))
            .layer(DefaultBodyLimit::max(MAX_FORM_BYTES))
            .layer(tower_http::limit::RequestBodyLimitLayer::new(
                MAX_FORM_BYTES,
            ))
            .layer(axum::middleware::from_fn(web::response_policy))
            .layer(axum::middleware::from_fn_with_state(
                self.clone(),
                staff_host,
            ))
            .layer(axum::middleware::from_fn(crate::browser::deadline))
            .with_state(self.clone())
    }

    fn matches_host(&self, headers: &HeaderMap) -> bool {
        let mut hosts = headers.get_all(header::HOST).iter();
        hosts
            .next()
            .and_then(|host| host.to_str().ok())
            .is_some_and(|host| host.eq_ignore_ascii_case(&self.host))
            && hosts.next().is_none()
    }

    pub(super) async fn show_link(&self, headers: &HeaderMap, account: Uuid) -> bool {
        if !self.matches_host(headers) {
            return false;
        }
        let pool = self.mfa.pool();
        let Ok(mut connection) = pool.acquire().await else {
            return false;
        };
        permitted(&mut connection, account, false)
            .await
            .unwrap_or(false)
    }

    async fn authorize(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        session: &BrowserSession,
    ) -> Result<Uuid> {
        let account = session.account_id().await.ok_or(Error::Denied)?;
        let session_hash = session.identity_hash().await.ok_or(Error::Denied)?;
        platform::AccountAuthority.account(tx, account).await?;
        let grant: Option<String> = sqlx::query_scalar(
            "SELECT s.data->>$3 FROM web_sessions s JOIN accounts a ON a.id=s.account_id
             WHERE s.id_hash=$1 AND s.account_id=$2 AND s.data ? $3
               AND s.idle_expires_at>clock_timestamp() AND s.absolute_expires_at>clock_timestamp()
               AND s.authorization_generation=a.native_security_generation
             FOR SHARE OF s",
        )
        .bind(session_hash)
        .bind(account)
        .bind(SESSION_GRANT_KEY)
        .fetch_optional(&mut **tx)
        .await?
        .flatten();
        let grant = grant.ok_or(Error::Denied)?;
        if !permitted(tx, account, true).await?
            || !self
                .mfa
                .satisfied_in_transaction(tx, account, &grant)
                .await?
        {
            return Err(Error::Denied);
        }
        Ok(account)
    }
}

async fn permitted(db: &mut PgConnection, account: Uuid, lock: bool) -> Result<bool> {
    let permissions: Vec<String> = if lock {
        sqlx::query_scalar(
            "SELECT p.permission FROM account_roles r JOIN role_permissions p ON p.role_id=r.role_id
             WHERE r.account_id=$1 AND p.permission=ANY($2) FOR SHARE OF r,p",
        )
        .bind(account)
        .bind(&ACCESS[..])
        .fetch_all(db)
        .await?
    } else {
        sqlx::query_scalar(
            "SELECT DISTINCT p.permission FROM account_roles r JOIN role_permissions p ON p.role_id=r.role_id
             WHERE r.account_id=$1 AND p.permission=ANY($2)",
        )
        .bind(account)
        .bind(&ACCESS[..])
        .fetch_all(db)
        .await?
    };
    Ok(ACCESS
        .iter()
        .all(|needed| permissions.iter().any(|p| p == needed)))
}

async fn staff_host(
    State(operator): State<Operator>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    if operator.matches_host(request.headers()) {
        let mut response = next.run(request).await;
        response.headers_mut().insert(
            header::CONTENT_SECURITY_POLICY,
            axum::http::HeaderValue::from_static(
                "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",
            ),
        );
        response
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

#[derive(sqlx::FromRow)]
struct ScheduledSeason {
    id: Uuid,
    starts_at: OffsetDateTime,
    ends_at: OffsetDateTime,
    active: bool,
}
#[derive(Template)]
#[template(
    path = "pass/operator.html",
    config = "../../templates/pass/askama.toml"
)]
struct StatusPage {
    lang: String,
    csrf: String,
    paid: bool,
    seasons: Vec<ScheduledSeason>,
}
impl StatusPage {
    fn t(&self, key: &str) -> String {
        web::text(&self.lang, key)
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DefinitionForm {
    #[serde(rename = "_csrf")]
    _csrf: String,
    definition: String,
}

async fn status(
    Path(lang): Path<String>,
    State(operator): State<Operator>,
    Extension(session): Extension<BrowserSession>,
) -> Response {
    if let Err(status) = web::locale(&lang) {
        return status.into_response();
    }
    let result = async {
        let pool = operator.mfa.pool();
        let mut tx = crate::browser::begin(&pool).await?;
        operator.authorize(&mut tx, &session).await?;
        let seasons = sqlx::query_as(
            "SELECT id,starts_at,ends_at,starts_at<=statement_timestamp() AS active
             FROM pass_seasons WHERE ends_at>statement_timestamp()
             ORDER BY starts_at LIMIT 10",
        )
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok::<_, Error>(StatusPage {
            lang: lang.clone(),
            csrf: session.csrf_token().await?,
            paid: operator.flags.paid,
            seasons,
        })
    }
    .await;
    match result {
        Ok(page) => match page.render() {
            Ok(html) => axum::response::Html(html).into_response(),
            Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        },
        Err(error) => web::refusal(lang, error),
    }
}

async fn publish(
    Path(lang): Path<String>,
    State(operator): State<Operator>,
    Extension(session): Extension<BrowserSession>,
    Form(form): Form<DefinitionForm>,
) -> Response {
    if let Err(status) = web::locale(&lang) {
        return status.into_response();
    }
    match operator.publish(&session, &form.definition).await {
        Ok(()) => Redirect::to(&format!("/{lang}/pass/operator")).into_response(),
        Err(error) => web::refusal(lang, error),
    }
}

impl Operator {
    async fn publish(&self, session: &BrowserSession, definition: &str) -> Result<()> {
        let pool = self.mfa.pool();
        let mut tx = crate::browser::begin(&pool).await?;
        let actor = self.authorize(&mut tx, session).await?;
        let season = parse_definition(definition)?;
        publish_definition(&mut tx, &season).await?;
        sqlx::query(
            "INSERT INTO audit_log(actor,action,target,before,after,request_id)
             VALUES ($1,'pass.season.published',$2,'null'::jsonb,$3,$4)",
        )
        .bind(actor)
        .bind(season.id.to_string())
        .bind(serde_json::json!({
            "challenges": season.challenges.len(),
            "rewards": season.rewards.len(),
        }))
        .bind(Uuid::now_v7())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }
}

fn parse_definition(definition: &str) -> Result<Season> {
    let season: Season = serde_json::from_str(definition).map_err(|_| Error::Invalid)?;
    season.validate()?;
    let first_day = season.starts_at.to_offset(time::UtcOffset::UTC).date();
    let last_day = (season.ends_at - time::Duration::NANOSECOND)
        .to_offset(time::UtcOffset::UTC)
        .date();
    let maximum_solves = 3 * (1 + (last_day - first_day).whole_days());
    if season.challenges.iter().any(|challenge| {
        challenge.source == Source::Portal
            && (challenge.metric != PUZZLES_SOLVED || challenge.target > maximum_solves)
    }) {
        return Err(Error::Invalid);
    }
    Ok(season)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publication_checks_portal_metric_limit_and_definition_shape() -> Result<()> {
        let mut definition = serde_json::json!({
            "id": Uuid::now_v7(),
            "starts_at": "2030-01-01T00:00:00Z",
            "ends_at": "2030-02-12T00:00:00Z",
            "price": 50,
            "challenges": [{
                "id": Uuid::now_v7(), "source": "portal",
                "metric": PUZZLES_SOLVED, "target": 3, "xp": 5,
            }],
            "rewards": [{
                "id": Uuid::now_v7(), "paid": false,
                "xp": 5, "grant_key": "test.frame", "grant_revision": "1",
            }],
        });
        assert_eq!(
            parse_definition(&definition.to_string())?.challenges[0].target,
            3
        );
        definition["challenges"][0]["target"] = 127.into();
        assert!(matches!(
            parse_definition(&definition.to_string()),
            Err(Error::Invalid)
        ));
        definition["challenges"][0]["target"] = 3.into();
        definition["challenges"][0]["metric"] = "arbitrary.metric".into();
        assert!(matches!(
            parse_definition(&definition.to_string()),
            Err(Error::Invalid)
        ));
        definition["challenges"][0]["metric"] = PUZZLES_SOLVED.into();
        definition["account_id"] = Uuid::now_v7().to_string().into();
        assert!(matches!(
            parse_definition(&definition.to_string()),
            Err(Error::Invalid)
        ));
        Ok(())
    }
    #[tokio::test]
    async fn staff_publication_and_schedule_use_current_permission()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        use super::super::tests::Db;
        use axum::{
            body::{Body, to_bytes},
            http::Request,
        };
        use oteryn_web::session::{self, PrivilegeChange, SessionKey};
        use time::{Duration, format_description::well_known::Rfc3339};
        use tower::ServiceExt;

        let db = Db::required().await?;
        let account = db.account;
        let grant = "7b".repeat(32);
        sqlx::query(
            "WITH enrolled AS (
               INSERT INTO account_mfa_totp(account_id,secret,confirmed)
               VALUES ($1,decode('00','hex'),true) RETURNING generation
             )
             INSERT INTO account_mfa_tokens(token_hash,account_id,generation,credential_hash,purpose,expires_at)
             SELECT sha256(convert_to($2,'UTF8')),$1,enrolled.generation,
               sha256(convert_to(a.password_hash,'UTF8')),'grant',now()+interval '1 hour'
             FROM enrolled JOIN accounts a ON a.id=$1",
        ).bind(account).bind(&grant).execute(&db.pool).await?;
        let generation: i64 =
            sqlx::query_scalar("SELECT native_security_generation FROM accounts WHERE id=$1")
                .bind(account)
                .fetch_one(&db.pool)
                .await?;
        let login = Router::new().route(
            "/test-login",
            get(move |Extension(session): Extension<BrowserSession>| {
                let grant = grant.clone();
                async move {
                    let result = async {
                        session::rotate(
                            &session,
                            PrivilegeChange::AuthorizedLogin(account, generation),
                        )
                        .await?;
                        session
                            .set(SessionKey::new(SESSION_GRANT_KEY), &grant)
                            .await?;
                        session.csrf_token().await
                    }
                    .await;
                    result.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
                }
            }),
        );
        let login = login.layer(axum::middleware::from_fn_with_state(
            SessionLayer::new(db.pool.clone()),
            session::middleware,
        ));
        let response = login
            .oneshot(Request::get("/test-login").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let cookie = response.headers()[header::SET_COOKIE]
            .to_str()?
            .split(';')
            .next()
            .ok_or("cookie")?
            .to_owned();
        let csrf = String::from_utf8(to_bytes(response.into_body(), 4096).await?.to_vec())?;

        let operator = Operator {
            mfa: Mfa::new(db.pool.clone(), [7; 32]),
            host: "staff.example.invalid".into(),
            flags: Flags {
                enabled: true,
                ..Flags::default()
            },
        };
        let app = operator.router(CsrfConfig::default());
        let path = "/en/pass/operator";
        let season = Uuid::now_v7();
        let start = OffsetDateTime::now_utc() + Duration::days(45);
        let definition = serde_json::json!({
            "id": season,
            "starts_at": start.format(&Rfc3339)?,
            "ends_at": (start + Duration::days(42)).format(&Rfc3339)?,
            "price": 50,
            "challenges": [{
                "id": Uuid::now_v7(), "source": "portal",
                "metric": PUZZLES_SOLVED, "target": 3, "xp": 5,
            }],
            "rewards": [{
                "id": Uuid::now_v7(), "paid": false,
                "xp": 5, "grant_key": "test.frame", "grant_revision": "1",
            }],
        });
        let post = |definition: &str| -> std::result::Result<_, axum::http::Error> {
            let body = format!(
                "_csrf={csrf}&definition={}",
                definition.replace('+', "%2B").replace('&', "%26")
            );
            Ok(app.clone().oneshot(
                Request::post(path)
                    .header(header::HOST, &operator.host)
                    .header(header::ORIGIN, "https://staff.example.invalid")
                    .header(header::COOKIE, &cookie)
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(body))?,
            ))
        };
        let definition = definition.to_string();
        assert_eq!(post(&definition)?.await?.status(), StatusCode::FORBIDDEN);
        let role: Uuid = sqlx::query_scalar(
            "INSERT INTO roles (id,key) VALUES (uuidv7(),'pass_operator_test') RETURNING id",
        )
        .fetch_one(&db.pool)
        .await?;
        sqlx::query("INSERT INTO account_roles (account_id,role_id) VALUES ($1,$2)")
            .bind(account)
            .bind(role)
            .execute(&db.pool)
            .await?;
        sqlx::query(
            "INSERT INTO role_permissions (role_id,permission) SELECT $1,unnest($2::text[])",
        )
        .bind(role)
        .bind(&ACCESS[..])
        .execute(&db.pool)
        .await?;
        assert_eq!(
            post("{}")?.await?.status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(post(&definition)?.await?.status(), StatusCode::SEE_OTHER);
        assert_eq!(post(&definition)?.await?.status(), StatusCode::CONFLICT);
        let response = app
            .oneshot(
                Request::get(path)
                    .header(header::HOST, &operator.host)
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let html = String::from_utf8(to_bytes(response.into_body(), 64000).await?.to_vec())?;
        assert!(html.contains(&season.to_string()));
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE target=$1 AND actor=$2 AND action='pass.season.published'")
            .bind(season.to_string()).bind(account).fetch_one(&db.pool).await?;
        assert_eq!(count, 1);
        db.close().await
    }
}
