//! Staff browser boundary for the support enforcement command journal.
use crate::{
    control::controls::http::enforcement_step_up,
    rbac::{CheckContext, GamePermissionFlags, Permission},
    web,
};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{DefaultBodyLimit, Query, State, rejection::FormRejection},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signature, Signer, SigningKey};
use oteryn_web::session::BrowserSession;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
use std::{future::Future, sync::Arc};
use uuid::Uuid;

pub use enforcement_step_up::appeal_step_up_current;

const PATH: &str = "/admin/console/enforcement";
const STEP_PATH: &str = "/admin/console/enforcement/step-up";
const MIN_MINUTES: u32 = 1;
const MAX_MINUTES: u32 = 1_440;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Profile {
    Mute,
    Jail,
}
impl Profile {
    pub const ALL: [Self; 2] = [Self::Mute, Self::Jail];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Mute => "MUTE",
            Self::Jail => "JAIL",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Apply,
    Replace,
    Revoke,
}
impl Action {
    fn name(self) -> &'static str {
        match self {
            Self::Apply => "apply",
            Self::Replace => "replace",
            Self::Revoke => "revoke",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    DisruptiveBehavior,
    RuleViolation,
}
impl Reason {
    pub const fn code(self) -> u16 {
        match self {
            Self::DisruptiveBehavior => 7,
            Self::RuleViolation => 8,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum StateLabel {
    None,
    Pending,
    Unconfirmed,
    Applied,
    Refused,
    Revoked,
    Expired,
}

#[derive(Debug)]
pub struct Target {
    pub account: Uuid,
    pub character: Uuid,
    pub name: String,
}

#[derive(Debug)]
pub struct Restriction {
    pub profile: Profile,
    /// Publication position of this profile's last submitted request, or zero.
    pub version: i64,
    pub operation: Option<Uuid>,
    pub state: StateLabel,
    pub current: StateLabel,
    pub expiry_pending: bool,
    pub expiry_blocked: bool,
    pub received_at: Option<String>,
    pub current_received_at: Option<String>,
    pub effective_until: Option<String>,
    pub result_code: Option<String>,
    pub actions: &'static [Action],
}

#[derive(Debug)]
pub struct Decision {
    pub actor: Uuid,
    pub profile: Profile,
    pub action: Action,
    pub operation: Uuid,
    pub version: i64,
    pub minutes: Option<u32>,
    pub reason: Reason,
}

#[derive(Debug)]
pub enum Error {
    BadInput,
    Denied,
    NotFound,
    Conflict,
    StepUp,
    Limited,
    Unavailable,
}

impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}

impl From<crate::rbac::Error> for Error {
    fn from(error: crate::rbac::Error) -> Self {
        match error {
            crate::rbac::Error::Storage(_) => Self::Unavailable,
            _ => Self::Denied,
        }
    }
}

/// Only the owning support crate supplies this adapter at startup.
pub trait Backend: Send + Sync + 'static {
    fn candidate_account(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        character: Uuid,
    ) -> impl Future<Output = Result<Uuid, Error>> + Send;

    fn lock_resources(
        &self,
        connection: &mut sqlx::PgConnection,
    ) -> impl Future<Output = Result<(), Error>> + Send;

    fn target(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        character: Uuid,
    ) -> impl Future<Output = Result<Target, Error>> + Send;

    fn prove_owner(
        &self,
        connection: &mut sqlx::PgConnection,
        target: &Target,
    ) -> impl Future<Output = Result<(), Error>> + Send;

    fn restrictions(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        target: &Target,
    ) -> impl Future<Output = Result<Vec<Restriction>, Error>> + Send;

    fn submit(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        target: &Target,
        input: Decision,
    ) -> impl Future<Output = Result<(), Error>> + Send;
}

struct Browser<B> {
    pool: PgPool,
    backend: B,
    auth: crate::rbac::Authorizer<web::PortalIdentity>,
}

pub fn configured_router(
    pool: PgPool,
    backend: impl Backend,
) -> Result<Router, Box<dyn std::error::Error>> {
    if std::env::var("OTERYN_ADMIN_CONSOLE").as_deref() != Ok("true") {
        return Ok(Router::new());
    }
    router(pool, backend)
}

fn router<B: Backend>(pool: PgPool, backend: B) -> Result<Router, Box<dyn std::error::Error>> {
    let state = Arc::new(Browser {
        backend,
        auth: crate::rbac::Authorizer::new(
            pool.clone(),
            web::PortalIdentity::configured(pool.clone()),
        )
        .with_game_flags(GamePermissionFlags {
            enabled: true,
            deploy: false,
        }),
        pool: pool.clone(),
    });
    let reads = Router::new()
        .route(PATH, get(dossier::<B>))
        .with_state(state.clone());
    let writes = Router::new()
        .route(PATH, post(submit::<B>))
        .route(STEP_PATH, post(step_up::<B>))
        .layer(DefaultBodyLimit::max(4096))
        .with_state(state);
    web::configured_guard(reads.merge(writes), pool, Permission::Access)
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Language {
    #[default]
    Pl,
    En,
}
impl Language {
    fn code(&self) -> &'static str {
        self.text("pl", "en")
    }

    fn text(&self, pl: &'static str, en: &'static str) -> &'static str {
        match self {
            Self::Pl => pl,
            Self::En => en,
        }
    }

    pub(crate) fn state(&self, state: &StateLabel) -> &'static str {
        match state {
            StateLabel::None => self.text("Brak", "None"),
            StateLabel::Pending => self.text("Oczekuje na grę", "Pending in Game"),
            StateLabel::Unconfirmed => self.text("Brak potwierdzonego stanu", "No confirmed state"),
            StateLabel::Applied => self.text("Nałożona w grze", "Applied in Game"),
            StateLabel::Refused => self.text("Gra odmówiła", "Refused by Game"),
            StateLabel::Revoked => self.text("Cofnięta w grze", "Revoked in Game"),
            StateLabel::Expired => self.text("Wygasła w grze", "Expired in Game"),
        }
    }

    fn action(&self, action: &Action) -> &'static str {
        match action {
            Action::Apply => self.text("Nałóż", "Apply"),
            Action::Replace => self.text("Zastąp", "Replace"),
            Action::Revoke => self.text("Cofnij", "Revoke"),
        }
    }

    fn profile(&self, profile: &Profile) -> &'static str {
        match profile {
            Profile::Mute => self.text("Wyciszenie", "Mute"),
            Profile::Jail => self.text("Więzienie", "Jail"),
        }
    }

    fn refusal(&self, error: Error, character: Option<Uuid>) -> Response {
        let (status, message) = match error {
            Error::BadInput => (
                StatusCode::BAD_REQUEST,
                self.text("Nieprawidłowe dane", "Invalid input"),
            ),
            Error::Denied => (
                StatusCode::FORBIDDEN,
                self.text("Brak uprawnień", "Permission denied"),
            ),
            Error::NotFound => (
                StatusCode::NOT_FOUND,
                self.text("Nie znaleziono postaci", "Character not found"),
            ),
            Error::Conflict => (
                StatusCode::CONFLICT,
                self.text(
                    "Stan sankcji się zmienił. Odśwież widok.",
                    "Enforcement state changed. Reload the dossier.",
                ),
            ),
            Error::StepUp => (
                StatusCode::FORBIDDEN,
                self.text(
                    "Potwierdź działanie kodem TOTP",
                    "Confirm with a fresh TOTP code",
                ),
            ),
            Error::Limited => (
                StatusCode::TOO_MANY_REQUESTS,
                self.text(
                    "Zbyt wiele prób. Spróbuj za minutę.",
                    "Too many attempts. Try again in one minute.",
                ),
            ),
            Error::Unavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                self.text(
                    "Sankcje postaci są niedostępne",
                    "Character enforcement is unavailable",
                ),
            ),
        };
        render(
            Dossier {
                lang: self,
                character,
                message: Some(message),
                ..Dossier::empty(self)
            },
            status,
        )
    }
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    #[serde(default)]
    lang: Language,
    character: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FormInput {
    _csrf: String,
    profile: Profile,
    intent: Action,
    operation: Uuid,
    proof: String,
    version: i64,
    minutes: Option<u32>,
    reason: Reason,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StepInput {
    _csrf: String,
    code: String,
}

impl Selection {
    fn character(&self) -> Result<Uuid, Error> {
        self.character
            .as_deref()
            .and_then(|raw| Uuid::parse_str(raw).ok())
            .filter(|id| id.get_version_num() == 7)
            .ok_or(Error::BadInput)
    }
}

fn location(lang: &Language, character: Uuid) -> String {
    format!("{PATH}?lang={}&character={character}", lang.code())
}

fn form_key(actor: CheckContext) -> Result<SigningKey, Error> {
    // Bind the form to the session and the server-held MFA grant. The session
    // cookie alone cannot recreate this key, and session rotation changes it.
    let session = actor.session_proof.ok_or(Error::Denied)?;
    let seed = Sha256::new()
        .chain_update(b"gm-enforcement-form-key-v1")
        .chain_update(session.grant_hash)
        .chain_update(session.session_hash)
        .finalize()
        .into();
    Ok(SigningKey::from_bytes(&seed))
}

#[derive(Debug, Clone, Copy)]
struct Claim {
    account: Uuid,
    character: Uuid,
    profile: Profile,
    intent: Action,
    version: i64,
    operation: Uuid,
}

impl Claim {
    fn bytes(self) -> String {
        format!(
            "gm-enforcement-v1/{}/{}/{}/{}/{}/{}",
            self.account,
            self.character,
            self.profile.name(),
            self.intent.name(),
            self.version,
            self.operation
        )
    }
}

struct ActionForm {
    intent: Action,
    operation: Uuid,
    proof: String,
}

struct Panel {
    restriction: Restriction,
    forms: Vec<ActionForm>,
}

#[derive(Template)]
#[template(path = "index.html", config = "src/console/enforcement/askama.toml")]
struct Dossier<'a> {
    lang: &'a Language,
    character: Option<Uuid>,
    name: Option<String>,
    message: Option<&'static str>,
    step_up: bool,
    csrf: String,
    panels: Vec<Panel>,
}
impl<'a> Dossier<'a> {
    fn empty(lang: &'a Language) -> Self {
        Self {
            lang,
            character: None,
            name: None,
            message: None,
            step_up: false,
            csrf: String::new(),
            panels: Vec::new(),
        }
    }

    fn title(&self) -> &'static str {
        self.lang.text("Sankcje postaci", "Character enforcement")
    }
}

fn render(page: Dossier<'_>, status: StatusCode) -> Response {
    let mut response = oteryn_web::render(&page);
    if response.status() == StatusCode::OK {
        *response.status_mut() = status;
    }
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    response
}

async fn dossier<B: Backend>(
    State(state): State<Arc<Browser<B>>>,
    Extension(actor): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    Query(query): Query<Selection>,
) -> Response {
    let result = async {
        state
            .auth
            .check(actor, Permission::GameSanctionRead)
            .await?;
        let mut page = Dossier::empty(&query.lang);
        if query.character.is_none() {
            return Ok(page);
        }
        let character = query.character()?;
        let edit = state
            .auth
            .check(actor, Permission::GameSanctionEnforce)
            .await
            .is_ok();
        let mut tx = state.pool.begin().await?;
        crate::rbac::mutation_budget(&mut tx).await?;
        let target = state.backend.target(&mut tx, character).await?;
        let restrictions = state.backend.restrictions(&mut tx, &target).await?;
        let actionable = edit && restrictions.iter().any(|row| !row.actions.is_empty());
        let elevated =
            actionable && enforcement_step_up::confirmed(&mut tx, &session, actor.account).await?;
        tx.commit().await?;

        page.character = Some(character);
        page.name = Some(target.name);
        page.step_up = actionable && !elevated;
        if actionable {
            page.csrf = session.csrf_token().await?;
        }
        let key = elevated.then(|| form_key(actor)).transpose()?;
        for restriction in restrictions {
            let mut forms = Vec::new();
            if let Some(key) = &key {
                for &intent in restriction.actions {
                    let claim = Claim {
                        account: target.account,
                        character,
                        profile: restriction.profile,
                        intent,
                        version: restriction.version,
                        operation: Uuid::now_v7(),
                    };
                    forms.push(ActionForm {
                        intent,
                        operation: claim.operation,
                        proof: URL_SAFE_NO_PAD
                            .encode(key.sign(claim.bytes().as_bytes()).to_bytes()),
                    });
                }
            }
            page.panels.push(Panel { restriction, forms });
        }
        Ok(page)
    }
    .await;
    match result {
        Ok(page) => render(page, StatusCode::OK),
        Err(error) => query.lang.refusal(error, query.character().ok()),
    }
}

async fn submit<B: Backend>(
    State(state): State<Arc<Browser<B>>>,
    Extension(actor): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    Query(query): Query<Selection>,
    form: Result<Form<FormInput>, FormRejection>,
) -> Response {
    let result = async {
        state
            .auth
            .check(actor, Permission::GameSanctionEnforce)
            .await?;
        state
            .auth
            .check(actor, Permission::GameSanctionRead)
            .await?;
        let character = query.character()?;
        let Form(form) = form.map_err(|_| Error::BadInput)?;
        match (form.intent, form.minutes) {
            (Action::Revoke, None) => {}
            (Action::Apply | Action::Replace, Some(MIN_MINUTES..=MAX_MINUTES)) => {}
            _ => return Err(Error::BadInput),
        }
        if form.operation.get_version_num() != 7 || form.version < 0 || form.proof.len() != 86 {
            return Err(Error::BadInput);
        }
        let proof = URL_SAFE_NO_PAD
            .decode(&form.proof)
            .map_err(|_| Error::BadInput)?;
        let proof = Signature::from_slice(&proof).map_err(|_| Error::BadInput)?;

        let mut tx = state.pool.begin().await?;
        crate::rbac::mutation_budget(&mut tx).await?;
        let candidate = state.backend.candidate_account(&mut tx, character).await?;
        let resources = state.clone();
        crate::rbac::authorize_mutation_with_resources_domain(
            &mut tx,
            &[candidate],
            move |connection| {
                Box::pin(async move { resources.backend.lock_resources(connection).await })
            },
            &state.auth,
            actor,
            &[
                Permission::Access,
                Permission::GameSanctionRead,
                Permission::GameSanctionEnforce,
            ],
        )
        .await?;
        if !enforcement_step_up::confirmed(&mut tx, &session, actor.account).await? {
            return Err(Error::StepUp);
        }
        let target = state.backend.target(&mut tx, character).await?;
        if target.account != candidate {
            return Err(Error::Conflict);
        }
        state.backend.prove_owner(&mut tx, &target).await?;
        let claim = Claim {
            account: target.account,
            character,
            profile: form.profile,
            intent: form.intent,
            version: form.version,
            operation: form.operation,
        };
        form_key(actor)?
            .verifying_key()
            .verify_strict(claim.bytes().as_bytes(), &proof)
            .map_err(|_| Error::Conflict)?;
        state
            .backend
            .submit(
                &mut tx,
                &target,
                Decision {
                    actor: actor.account,
                    profile: form.profile,
                    action: form.intent,
                    operation: form.operation,
                    version: form.version,
                    minutes: form.minutes,
                    reason: form.reason,
                },
            )
            .await?;
        state.auth.session_in(&mut tx, actor).await?;
        if !enforcement_step_up::confirmed(&mut tx, &session, actor.account).await? {
            return Err(Error::StepUp);
        }
        tx.commit().await?;
        Ok(Redirect::to(&location(&query.lang, character)))
    }
    .await;
    match result {
        Ok(redirect) => redirect.into_response(),
        Err(error) => query.lang.refusal(error, query.character().ok()),
    }
}

async fn step_up<B: Backend>(
    State(state): State<Arc<Browser<B>>>,
    Extension(actor): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    Query(query): Query<Selection>,
    form: Result<Form<StepInput>, FormRejection>,
) -> Response {
    use crate::control::controls::Refusal;
    let result = async {
        state
            .auth
            .check(actor, Permission::GameSanctionEnforce)
            .await?;
        state
            .auth
            .check(actor, Permission::GameSanctionRead)
            .await?;
        let character = query.character()?;
        let Form(form) = form.map_err(|_| Error::BadInput)?;
        enforcement_step_up::verify(&state.pool, &session, &form.code)
            .await
            .map_err(|error| match error {
                Refusal::Denied => Error::Denied,
                Refusal::InvalidInput => Error::BadInput,
                Refusal::RateLimited => Error::Limited,
                _ => Error::Unavailable,
            })?;
        Ok(Redirect::to(&location(&query.lang, character)))
    }
    .await;
    match result {
        Ok(redirect) => redirect.into_response(),
        Err(error) => query.lang.refusal(error, query.character().ok()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rbac::SessionProof;

    #[test]
    fn enforcement_form_key_binds_both_session_and_server_mfa_grant() -> Result<(), Error> {
        let actor = |session_hash, grant_hash| CheckContext {
            account: Uuid::now_v7(),
            require_staff_mfa: true,
            mfa_verified: true,
            session_proof: Some(SessionProof {
                session_hash,
                grant_hash,
            }),
        };
        let cookie_binding = [1; 32];
        let grant = [2; 32];
        let primary = form_key(actor(cookie_binding, grant))?.verifying_key();
        assert_ne!(
            primary,
            form_key(actor(cookie_binding, [3; 32]))?.verifying_key()
        );
        assert_ne!(primary, form_key(actor([4; 32], grant))?.verifying_key());
        Ok(())
    }
}
