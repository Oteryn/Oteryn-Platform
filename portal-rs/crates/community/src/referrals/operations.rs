use super::{Error, Worker, web::translate};
use askama::Template;
use axum::{
    Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use oteryn_admin::{rbac::Permission, web::configured_guard};
use sqlx::PgPool;

#[derive(Clone)]
struct Operations {
    pool: PgPool,
    deployment_enabled: bool,
    mail_configured: bool,
    premium_enabled: bool,
}

pub(super) fn configured_router(worker: &Worker) -> Result<Router, Box<dyn std::error::Error>> {
    configured_guard(
        routes(worker),
        worker.campaign.pool.clone(),
        Permission::Access,
    )
}

pub(super) fn routes(worker: &Worker) -> Router {
    let state = Operations {
        pool: worker.campaign.pool.clone(),
        deployment_enabled: worker.campaign.flags.enabled,
        mail_configured: worker.sender.is_some(),
        premium_enabled: worker.premium_enabled,
    };
    Router::new()
        .route("/admin/referrals", get(page))
        .with_state(state)
}

#[derive(serde::Deserialize)]
struct Locale {
    lang: Option<String>,
}

#[derive(Template)]
#[template(
    path = "community/referral_operations.html",
    config = "src/guilds_askama.toml"
)]
struct Page<'a> {
    lang: &'a str,
    title: String,
    back: String,
    game_unavailable: String,
    states: Vec<(String, String)>,
}

#[derive(sqlx::FromRow)]
struct Counts {
    campaign_enabled: bool,
    activity_enabled: bool,
    coverage_ready: bool,
    pending_mail: i64,
    paid: i64,
    pending_premium: i64,
    applied_premium: i64,
    game_held: i64,
}

impl Operations {
    async fn status<'a>(&self, lang: &'a str) -> Result<Page<'a>, Error> {
        let mut tx = self.pool.begin().await?;
        oteryn_db::mutation_budget(&mut tx).await?;
        let counts: Counts = sqlx::query_as(
            "SELECT s.enabled AS campaign_enabled, referral_deployment_enabled() AS activity_enabled,
                 COALESCE(c.covered_since <= statement_timestamp()-interval '12 months', false) AS coverage_ready,
                 (SELECT count(*) FROM referral_email_attempts WHERE delivered_at IS NULL) AS pending_mail,
                 (SELECT count(*) FROM referral_campaign_invitations i WHERE first_paid_at IS NOT NULL OR EXISTS(
                     SELECT 1 FROM referral_campaign_rewards r WHERE r.operation_key='buddy-premium:'||i.id)) AS paid,
                 (SELECT count(*) FROM referral_campaign_rewards WHERE kind='premium' AND applied_at IS NULL) AS pending_premium,
                 (SELECT count(*) FROM referral_campaign_rewards WHERE kind='premium' AND applied_at IS NOT NULL) AS applied_premium,
                 (SELECT count(*) FROM outbox WHERE kind='referral.entitlement' AND state='pending' AND run_after='infinity') AS game_held
             FROM referral_campaign_settings s CROSS JOIN portal_referral_coverage c WHERE s.singleton AND c.id",
        ).fetch_one(&mut *tx).await?;
        tx.commit().await?;

        let t = |key| translate(lang, key);
        let enabled = |on| if on { "enabled" } else { "disabled" };
        let coverage = if counts.activity_enabled && counts.coverage_ready {
            "ready"
        } else {
            "unavailable"
        };
        let mut states = Vec::new();
        for (label, value) in [
            ("operations-deployment", enabled(self.deployment_enabled)),
            ("operations-campaign", enabled(counts.campaign_enabled)),
            ("operations-fence", enabled(counts.activity_enabled)),
            ("operations-coverage", coverage),
            ("operations-smtp", enabled(self.mail_configured)),
            ("operations-premium", enabled(self.premium_enabled)),
        ] {
            states.push((t(label)?, t(value)?));
        }
        for (label, count) in [
            ("operations-mail", counts.pending_mail),
            ("operations-paid", counts.paid),
            ("operations-premium-pending", counts.pending_premium),
            ("operations-premium-applied", counts.applied_premium),
            ("operations-held", counts.game_held),
        ] {
            states.push((t(label)?, count.to_string()));
        }
        Ok(Page {
            lang,
            title: t("operations-title")?,
            back: t("operations-back")?,
            game_unavailable: t("game-unavailable")?,
            states,
        })
    }
}

async fn page(
    State(operations): State<Operations>,
    Query(locale): Query<Locale>,
    headers: HeaderMap,
) -> Response {
    let lang = match locale.lang.as_deref() {
        Some("en") => "en",
        Some("pl") => "pl",
        Some(_) => return StatusCode::BAD_REQUEST.into_response(),
        None if headers
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("en")) =>
        {
            "en"
        }
        None => "pl",
    };
    match operations.status(lang).await {
        Ok(page) => oteryn_web::render(&page),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
