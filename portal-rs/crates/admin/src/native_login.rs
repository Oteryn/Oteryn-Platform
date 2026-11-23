//! Read-only qualification status for production native login (N4P-5).
use crate::web::{message, messages, panel_language};
use askama::Template;
use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};

const DECISIONS: &[&str] = &[
    "native-login-u3",
    "native-login-d3",
    "native-login-u7",
    "native-login-u8",
    "native-login-u9",
];

const EVIDENCE: &[&str] = &[
    "native-login-route",
    "native-login-ownership",
    "native-login-connect",
];

const SWITCHES: &[(&str, &str)] = &[
    ("native-login-ticket-switch", "PORTAL_GAME_TICKETS_ENABLED"),
    (
        "native-login-admission-switch",
        "PORTAL_NATIVE_ADMISSIONS_ENABLED",
    ),
    (
        "native-login-grant-switch",
        "PORTAL_NATIVE_ADMISSION_GRANTS_ENABLED",
    ),
];

fn switch_state(raw: Option<&str>) -> &'static str {
    match raw {
        Some("true") => "native-login-on",
        None | Some("false") => "native-login-off",
        Some(_) => "native-login-invalid",
    }
}

#[derive(Template)]
#[template(path = "admin/native_login.html", config = "src/web_askama.toml")]
struct Page {
    lang: &'static str,
    title: String,
    back: String,
    blocked: String,
    summary: String,
    decisions_title: String,
    decisions: Vec<String>,
    pending: String,
    evidence_title: String,
    evidence: Vec<String>,
    unavailable: String,
    switches_title: String,
    switches_help: String,
    switches: Vec<(String, String)>,
}

pub(super) async fn page(headers: HeaderMap) -> Response {
    render(panel_language(&headers), |name| {
        std::env::var_os(name).map(|value| value.to_string_lossy().into_owned())
    })
}

fn render(lang: &'static str, lookup: impl Fn(&str) -> Option<String>) -> Response {
    let Some(bundle) = messages(lang) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let t = |key| message(&bundle, key);
    oteryn_web::render(&Page {
        lang,
        title: t("native-login-title"),
        back: t("title"),
        blocked: t("native-login-blocked"),
        summary: t("native-login-summary"),
        decisions_title: t("native-login-decisions"),
        decisions: DECISIONS.iter().map(|key| t(key)).collect(),
        pending: t("native-login-pending"),
        evidence_title: t("native-login-evidence"),
        evidence: EVIDENCE.iter().map(|key| t(key)).collect(),
        unavailable: t("native-login-unavailable"),
        switches_title: t("native-login-switches"),
        switches_help: t("native-login-switches-help"),
        switches: SWITCHES
            .iter()
            .map(|(key, setting)| (t(key), t(switch_state(lookup(setting).as_deref()))))
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn qualification_stays_blocked_with_local_switches_on_or_invalid()
    -> Result<(), Box<dyn std::error::Error>> {
        for (lang, title, unknown, invalid) in [
            (
                "en",
                "Native login readiness",
                "Unavailable",
                "Invalid configuration",
            ),
            (
                "pl",
                "Gotowość logowania do gry",
                "Niedostępne",
                "Nieprawidłowa konfiguracja",
            ),
        ] {
            let response = render(lang, |setting| match setting {
                "PORTAL_GAME_TICKETS_ENABLED" => Some("true".into()),
                "PORTAL_NATIVE_ADMISSION_GRANTS_ENABLED" => Some("<unexpected>".into()),
                _ => None,
            });
            assert_eq!(response.status(), StatusCode::OK);
            let html = String::from_utf8(
                axum::body::to_bytes(response.into_body(), 32_768)
                    .await?
                    .to_vec(),
            )?;
            assert!(html.contains(title));
            assert!(html.contains("data-state=\"blocked\""));
            assert_eq!(
                html.matches("data-state=\"pending\"").count(),
                DECISIONS.len()
            );
            assert_eq!(
                html.matches("data-state=\"unavailable\"").count(),
                EVIDENCE.len()
            );
            assert!(html.contains(unknown));
            assert!(html.contains(invalid));
            assert!(!html.contains("unexpected"));
        }
        assert_eq!(switch_state(None), "native-login-off");
        assert_eq!(switch_state(Some("false")), "native-login-off");
        assert_eq!(switch_state(Some("true")), "native-login-on");
        assert_eq!(switch_state(Some("1")), "native-login-invalid");
        Ok(())
    }
}
