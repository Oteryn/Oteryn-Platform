use super::*;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use sha2::{Digest, Sha256};
use tower::ServiceExt;
type TestResult = Result<(), Box<dyn std::error::Error>>;
#[path = "consents_test_db.rs"]
mod database;
struct Fixture {
    pool: sqlx::PgPool,
    admin: sqlx::PgPool,
    schema: String,
    account: AccountId,
    token: String,
    csrf: String,
}
impl Fixture {
    async fn new() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED consents PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let (pool, admin, schema) = database::database(&url).await?;
        let account = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES ('consent-owner@example.invalid','unused','active','en',now()) RETURNING id").fetch_one(&pool).await?;
        let token = "a".repeat(64);
        let csrf = "c".repeat(64);
        sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at,sudo_until) VALUES ($1,$2,$3,'consent test',now()+interval '1 hour',now()+interval '1 day',now()+interval '5 minutes')")
            .bind(Sha256::digest(token.as_bytes()).to_vec()).bind(account).bind(&csrf).execute(&pool).await?;
        Ok(Some(Self {
            pool,
            admin,
            schema,
            account,
            token,
            csrf,
        }))
    }
    fn config(&self) -> OptionalConfig {
        OptionalConfig {
            enabled: true,
            ..OptionalConfig::default()
        }
    }
    fn app(&self) -> Router {
        oteryn_web::router(
            oteryn_web::AppState {
                db: self.pool.clone(),
            },
            "../../static",
        )
        .layer(axum::Extension(FeatureFlags::from([FEATURE.to_owned()])))
    }
    async fn count(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT count(*) FROM account_consents WHERE account_id=$1")
            .bind(self.account)
            .fetch_one(&self.pool)
            .await
    }
    async fn close(self) -> TestResult {
        database::close(self.pool, self.admin, self.schema).await?;
        Ok(())
    }
    fn post(&self, body: String) -> Result<Request<Body>, axum::http::Error> {
        Request::post("/en/account/consents")
            .header(
                header::COOKIE,
                format!("__Host-oteryn_session={}", self.token),
            )
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(body))
    }
}
#[tokio::test]
async fn consents_default_not_granted() -> TestResult {
    let Some(f) = Fixture::new().await? else {
        return Ok(());
    };
    for kind in [Kind::PlaySession, Kind::PersonalisedAds] {
        assert!(current(&f.pool, f.account, kind).await?.is_none());
    }
    assert_eq!(f.count().await?, 0);
    let granted: bool = sqlx::query_scalar("INSERT INTO account_consents(account_id,document_kind,document_version) VALUES ($1,'play_session','2026-11-11.1') RETURNING granted")
        .bind(f.account).fetch_one(&f.pool).await?;
    assert!(!granted);
    f.close().await
}
#[tokio::test]
async fn consents_grant_and_revoke_append_records() -> TestResult {
    let Some(f) = Fixture::new().await? else {
        return Ok(());
    };
    let config = f.config();
    for kind in [Kind::PlaySession, Kind::PersonalisedAds] {
        for granted in [true, false, true, false] {
            assert_eq!(
                decide(
                    &f.pool,
                    &config,
                    f.account,
                    kind,
                    granted,
                    config.version(kind)
                )
                .await,
                Ok(())
            );
            let state = current(&f.pool, f.account, kind)
                .await?
                .ok_or("missing state")?;
            assert_eq!(state.granted, granted);
            assert_eq!(state.version, config.version(kind));
            assert!(state.decided_at.ends_with('Z'));
        }
    }
    assert_eq!(f.count().await?, 8);
    let valid: bool = sqlx::query_scalar("SELECT bool_and(actor=account_id AND source='web') FROM account_consents WHERE account_id=$1").bind(f.account).fetch_one(&f.pool).await?;
    assert!(valid);
    f.close().await
}
#[tokio::test]
async fn consents_outdated_version_refused() -> TestResult {
    let Some(f) = Fixture::new().await? else {
        return Ok(());
    };
    for kind in [Kind::PlaySession, Kind::PersonalisedAds] {
        assert_eq!(
            decide(&f.pool, &f.config(), f.account, kind, true, "old").await,
            Err(DecisionError::OutdatedVersion)
        );
    }
    assert_eq!(f.count().await?, 0);
    let response = f
        .app()
        .oneshot(f.post(format!(
            "kind=play_session&version=old&granted=true&_csrf={}",
            f.csrf
        ))?)
        .await?;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let body = String::from_utf8(to_bytes(response.into_body(), 1024 * 1024).await?.to_vec())?;
    assert!(body.contains("The consent text has changed"));
    assert_eq!(f.count().await?, 0);
    assert_eq!(
        decide(
            &f.pool,
            &f.config(),
            f.account,
            Kind::PlaySession,
            false,
            "old"
        )
        .await,
        Ok(())
    );
    assert_eq!(f.count().await?, 1);
    f.close().await
}
#[tokio::test]
async fn consents_csrf_required() -> TestResult {
    let Some(f) = Fixture::new().await? else {
        return Ok(());
    };
    for suffix in [
        String::new(),
        "&_csrf=wrong".into(),
        format!("&_csrf={}&_csrf={}", f.csrf, f.csrf),
    ] {
        let response = f
            .app()
            .oneshot(f.post(format!(
                "kind=play_session&granted=true&version=2026-11-11.1{suffix}"
            ))?)
            .await?;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    assert_eq!(f.count().await?, 0);
    f.close().await
}
#[tokio::test]
async fn consents_anonymous_redirected() -> TestResult {
    let Some(f) = Fixture::new().await? else {
        return Ok(());
    };
    for lang in ["pl", "en"] {
        let response = f
            .app()
            .oneshot(Request::get(format!("/{lang}/account/consents")).body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response.headers()[header::LOCATION],
            format!("/{lang}/login")
        );
    }
    assert_eq!(f.count().await?, 0);
    f.close().await
}
#[tokio::test]
async fn consents_page_through_real_router() -> TestResult {
    let Some(f) = Fixture::new().await? else {
        return Ok(());
    };
    for lang in ["pl", "en"] {
        let response = f
            .app()
            .oneshot(
                Request::get(format!("/{lang}/account/consents"))
                    .header(header::COOKIE, format!("__Host-oteryn_session={}", f.token))
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let body = String::from_utf8(to_bytes(response.into_body(), 1024 * 1024).await?.to_vec())?;
        assert!(body.contains(&format!("<html lang=\"{lang}\"")));
        assert!(body.contains("name=\"_csrf\""));
        assert!(body.contains("for=\"play_session\""));
        assert!(body.contains("for=\"personalised_ads\""));
        assert_eq!(body.matches("type=\"checkbox\"").count(), 2);
        assert!(!body.contains("checked"));
        if lang == "en" {
            assert!(body.contains(
                "sole purpose of improving the gaming experience or enforcing the Rules"
            ));
            assert!(body.contains("There are no ads in the game."));
        } else {
            assert!(body.contains("wyłącznie w celu poprawy wrażeń z gry lub egzekwowania Zasad"));
            assert!(body.contains("W grze nie ma reklam."));
        }
    }
    for suffix in ["&granted=true", ""] {
        let response = f
            .app()
            .oneshot(f.post(format!(
                "kind=play_session&version=2026-11-11.1&_csrf={}{}",
                f.csrf, suffix
            ))?)
            .await?;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(response.headers()[header::LOCATION], "/en/account/consents");
    }
    assert_eq!(f.count().await?, 2);
    assert!(
        !current(&f.pool, f.account, Kind::PlaySession)
            .await?
            .ok_or("missing decision")?
            .granted
    );
    f.close().await
}
#[tokio::test]
async fn consents_bad_input_and_disabled_refused() -> TestResult {
    let Some(f) = Fixture::new().await? else {
        return Ok(());
    };
    for input in [
        "kind=invalid&version=v1",
        "kind=play_session&version=",
        "kind=play_session&version=2026-11-11.1&granted=no",
        "kind=play_session&kind=personalised_ads&version=v1",
        "kind=play_session&version=2026-11-11.1&account_id=other",
    ] {
        let response = f
            .app()
            .oneshot(f.post(format!("{input}&_csrf={}", f.csrf))?)
            .await?;
        assert!(matches!(
            response.status(),
            StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY
        ));
    }
    let disabled = oteryn_web::router(oteryn_web::AppState { db: f.pool.clone() }, "../../static")
        .layer(axum::Extension(OptionalConfig::default()));
    let response = disabled
        .oneshot(Request::get("/en/account/consents").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(f.count().await?, 0);
    f.close().await
}
#[tokio::test]
async fn consents_history_immutable_and_account_isolated() -> TestResult {
    let Some(f) = Fixture::new().await? else {
        return Ok(());
    };
    assert_eq!(
        decide(
            &f.pool,
            &f.config(),
            f.account,
            Kind::PlaySession,
            true,
            "2026-11-11.1"
        )
        .await,
        Ok(())
    );
    assert!(
        current(&f.pool, uuid::Uuid::now_v7(), Kind::PlaySession)
            .await?
            .is_none()
    );
    for query in [
        "UPDATE account_consents SET granted=false WHERE document_kind='play_session'",
        "DELETE FROM account_consents WHERE document_kind='play_session'",
    ] {
        assert!(
            sqlx::query(sqlx::AssertSqlSafe(query))
                .execute(&f.pool)
                .await
                .is_err()
        );
    }
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(f.account)
        .execute(&f.pool)
        .await?;
    assert_eq!(
        decide(
            &f.pool,
            &f.config(),
            f.account,
            Kind::PlaySession,
            false,
            "2026-11-11.1"
        )
        .await,
        Err(DecisionError::Refused)
    );
    assert_eq!(f.count().await?, 1);
    f.close().await
}

#[tokio::test]
async fn consents_export_includes_history() -> TestResult {
    let Some(f) = Fixture::new().await? else {
        return Ok(());
    };
    for kind in [Kind::PlaySession, Kind::PersonalisedAds] {
        for granted in [true, false] {
            assert_eq!(
                decide(
                    &f.pool,
                    &f.config(),
                    f.account,
                    kind,
                    granted,
                    "2026-11-11.1"
                )
                .await,
                Ok(())
            );
        }
    }
    let data = crate::export::download(
        &f.pool,
        &crate::export::ExportConfig { enabled: true },
        &f.token,
    )
    .await
    .map_err(|status| status.to_string())?;
    let history = data["consents"].as_array().ok_or("missing history")?;
    assert_eq!(history.len(), 4);
    for kind in ["play_session", "personalised_ads"] {
        let decisions: Vec<_> = history
            .iter()
            .filter(|row| row["document_kind"] == kind)
            .collect();
        assert_eq!(decisions.len(), 2);
        for (row, granted) in decisions.iter().zip([true, false]) {
            assert_eq!(row["document_version"], "2026-11-11.1");
            assert_eq!(row["granted"], granted);
            assert_eq!(row["actor"], f.account.to_string());
            assert_eq!(row["source"], "web");
            assert!(row["accepted_at"].is_string());
            assert!(row.get("ip").is_some());
        }
        assert_ne!(decisions[0]["accepted_at"], decisions[1]["accepted_at"]);
    }
    assert_eq!(data["account"]["id"], f.account.to_string());
    assert_eq!(
        crate::export::download(
            &f.pool,
            &crate::export::ExportConfig { enabled: true },
            "invalid"
        )
        .await,
        Err(StatusCode::UNAUTHORIZED)
    );
    f.close().await
}

#[tokio::test]
async fn consents_legal_acceptance_unchanged() -> TestResult {
    let Some(f) = Fixture::new().await? else {
        return Ok(());
    };
    oteryn_db::test_support::seed_final_signup_editions(&f.pool, DEFAULT_LEGAL_VERSION).await?;
    let config = ConsentConfig::default();
    let mut tx = f.pool.begin().await?;
    let editions = config.check_publication(&mut *tx, "en").await?;
    record(&mut tx, f.account, &editions, None).await?;
    tx.commit().await?;
    let legal: Vec<(String, bool, bool)> = sqlx::query_as("SELECT document_kind,granted,actor=account_id FROM account_consents WHERE account_id=$1 ORDER BY document_kind")
        .bind(f.account).fetch_all(&f.pool).await?;
    assert_eq!(
        legal,
        vec![("privacy".into(), true, true), ("terms".into(), true, true)]
    );
    let mut tx = f.pool.begin().await?;
    let editions = config.check_publication(&mut *tx, "en").await?;
    assert!(record(&mut tx, f.account, &editions, None).await.is_err());
    tx.rollback().await?;
    f.close().await
}

#[tokio::test]
async fn audit02_consent_dispatch_budget_stops_durable_writes() -> TestResult {
    let Some(f) = Fixture::new().await? else {
        return Ok(());
    };
    let app = f.app();
    for n in 0..12 {
        let response = app
            .clone()
            .oneshot(f.post(format!(
                "kind=play_session&version=2026-11-11.1&_csrf={}&granted=true",
                f.csrf
            ))?)
            .await?;
        assert_eq!(
            response.status(),
            if n < 10 {
                StatusCode::SEE_OTHER
            } else {
                StatusCode::TOO_MANY_REQUESTS
            }
        );
    }
    assert_eq!(f.count().await?, 10);
    f.close().await
}

#[test]
fn recognizes_placeholders_in_rendered_legal_text() {
    for body in [
        "NIP 000‑000‑00‑00",
        "NIP (000) 000-00-00",
        "NIP (00\u{034f}0) 000-00-00",
        "Tax ID (NIP) （０００） 000-00-00",
        "Tax ID (NIP) **000**&#8209;000&#x2011;00‑00",
        "Tax ID (NIP) <strong>000</strong>&#8209;000‑00‑00",
        "KRS 0000000000",
        "NIP: ０００–０００·００ ００",
        "ul. Przykładowa **1**",
    ] {
        let text = inspect_legal_edition("final-v1", "Terms", body);
        assert!(text.has_placeholder && text.has_visible_body, "{body}");
    }
    let real = inspect_legal_edition("final-v1", "Terms", "**NIP (123) 456-78-02**");
    assert!(!real.has_placeholder && real.has_visible_body);
    for body in [
        "<!-- NIP 000‑000‑00‑00 -->",
        "<p>&nbsp;&#8203;</p>",
        "<hr>",
        "<p>&#x034f;&#xfe0f;</p>",
        "&#x2800;",
    ] {
        let text = inspect_legal_edition("final-v1", "Terms", body);
        assert!(!text.has_placeholder && !text.has_visible_body, "{body}");
    }
}

#[test]
fn detects_unresolved_template_fields_in_legal_editions() {
    for (title, body) in [
        ("Terms", "Operator: {{operator}}"),
        (
            "Regulamin",
            "Operator: &#123;&#123; operator.name &#125;&#125;",
        ),
        (
            "Terms",
            "Contact information. ![{{operator}}](/missing-image.png)",
        ),
        ("Terms for {{operator}}", "Contact information."),
    ] {
        let edition = inspect_legal_edition("final-v1", title, body);
        assert!(
            edition.has_visible_body && edition.has_placeholder,
            "{title} {body}"
        );
    }
    assert!(!inspect_legal_edition("final-v1", "Terms", "Contact {operator}").has_placeholder);
}

#[test]
fn validates_nip_in_rendered_text_and_image_alternatives() {
    for body in [
        "![Tax ID (NIP) (000) 000-00-00](/missing-image.png)",
        "NIP (000) ![000-00-00](/missing-image.png)",
        "<img src='/missing-image.png' alt='NIP 000&#8209;000-00-00'>",
        "<a href='/contact' title='NIP 000-000-00-00'>Contact</a>",
    ] {
        assert!(
            inspect_legal_edition("final-v1", "Terms", body).has_placeholder,
            "{body}"
        );
    }
    assert!(
        !inspect_legal_edition(
            "final-v1",
            "Terms",
            "![NIP 123-456-78-02](/missing-image.png)"
        )
        .has_visible_body
    );

    for body in [
        "Tax ID (NIP) PL 123-456-78-03",
        "NIP: 123 456 78 90",
        "NIP: 123-456-78-021",
        "NIP: 123-456",
        "Service terms.\n\n![NIP: 123-456-78-03](/missing-image.png)",
        "Warunki świadczenia usług.\n\n<img src='/missing-image.png' alt='NIP: 123&#8209;456-78-03'>",
    ] {
        let text = inspect_legal_edition("final-v1", "Terms", body);
        assert!(
            text.has_visible_body && text.has_invalid_nip && !text.has_placeholder,
            "{body}"
        );
    }
    for body in [
        "NIP: PL (１２３) 456‑78‑02; 2026",
        "NIP: 123 456 78 02\n\n2026",
        "<p>NIP: 123 456 78 02<br>2026</p>",
        "Contact.\n\n![Tax ID (NIP) 123-456-78-02](/missing-image.png)",
        "Dane do kontaktu. Numer NIP można podać w formularzu.",
    ] {
        let text = inspect_legal_edition("final-v1", "Terms", body);
        assert!(
            text.has_visible_body && !text.needs_operator_update(),
            "{body}"
        );
    }
}
