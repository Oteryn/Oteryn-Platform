use super::{
    Labels, failure,
    store::{
        self, Command, Component, CreateIncident, Error, Incident, Incidents, Maintenance, Message,
        ScheduleMaintenance, Transition,
    },
};
use crate::{
    rbac::{Authorizer, CheckContext, MutationAuthorization, Permission},
    web::{self, PortalIdentity},
};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, header},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use oteryn_web::session::BrowserSession;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

pub(super) fn router(db: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let guard = |router, permission| web::configured_guard(router, db.clone(), permission);
    Ok(guard(
        Router::new()
            .route("/admin/status", get(page))
            .with_state(db.clone()),
        Permission::StatusView,
    )?
    .merge(guard(
        Router::new()
            .route("/admin/status/incidents", post(create))
            .with_state(db.clone()),
        Permission::StatusIncidentCreate,
    )?)
    .merge(guard(
        Router::new()
            .route("/admin/status/incidents/{id}/update", post(transition))
            .with_state((db.clone(), Transition::Update)),
        Permission::StatusIncidentUpdate,
    )?)
    .merge(guard(
        Router::new()
            .route("/admin/status/incidents/{id}/resolve", post(transition))
            .with_state((db.clone(), Transition::Resolve)),
        Permission::StatusIncidentResolve,
    )?)
    .merge(guard(
        Router::new()
            .route("/admin/status/maintenance", post(schedule))
            .with_state(db.clone()),
        Permission::StatusMaintenanceSchedule,
    )?))
}

#[derive(Deserialize)]
struct LanguageQuery {
    lang: Option<String>,
}

impl LanguageQuery {
    fn language(&self, headers: &HeaderMap) -> &'static str {
        match self.lang.as_deref() {
            Some("pl") => "pl",
            Some("en") => "en",
            _ => web::language(headers),
        }
    }
}

#[derive(Template)]
#[template(path = "status/staff.html", config = "src/web_askama.toml")]
struct StatusAdmin {
    lang: &'static str,
    labels: Labels,
    csrf: String,
    worlds: Vec<Component>,
    active: Vec<Incident>,
    maintenance: Vec<Maintenance>,
    can_create: bool,
    can_update: bool,
    can_resolve: bool,
    can_schedule: bool,
}

impl StatusAdmin {
    fn publication_key(&self) -> Uuid {
        Uuid::now_v7()
    }
}

#[derive(Deserialize)]
struct Publication<T> {
    publication_key: Uuid,
    #[serde(flatten)]
    input: T,
}

async fn allowed(
    auth: &Authorizer<PortalIdentity>,
    context: CheckContext,
    permission: Permission,
) -> Result<bool, Error> {
    match auth.check(context, permission).await {
        Ok(()) => Ok(true),
        Err(crate::rbac::Error::Denied) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

async fn load_page(
    db: PgPool,
    context: CheckContext,
    session: BrowserSession,
    lang: &'static str,
) -> Result<StatusAdmin, Error> {
    let auth = Authorizer::new(db.clone(), PortalIdentity::configured(db.clone()));
    let can_create = allowed(&auth, context, Permission::StatusIncidentCreate).await?;
    let can_update = allowed(&auth, context, Permission::StatusIncidentUpdate).await?;
    let can_resolve = allowed(&auth, context, Permission::StatusIncidentResolve).await?;
    let can_schedule = allowed(&auth, context, Permission::StatusMaintenanceSchedule).await?;
    let mut connection = db.acquire().await?;
    let now = store::clock(&mut connection).await?;
    let worlds = store::components(&mut connection, now).await?;
    let active = store::incidents(&mut connection, lang, now, Incidents::Open).await?;
    let maintenance = store::maintenance(&mut connection, lang, now, false).await?;
    drop(connection);
    Ok(StatusAdmin {
        lang,
        labels: Labels(lang),
        csrf: session.csrf_token().await?,
        worlds,
        active,
        maintenance,
        can_create,
        can_update,
        can_resolve,
        can_schedule,
    })
}

async fn page(
    State(db): State<PgPool>,
    Extension(context): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    Query(query): Query<LanguageQuery>,
    headers: HeaderMap,
) -> Response {
    let lang = query.language(&headers);
    match load_page(db, context, session, lang).await {
        Ok(page) => {
            let mut response = oteryn_web::render(&page);
            response.headers_mut().insert(
                header::CONTENT_LANGUAGE,
                header::HeaderValue::from_static(lang),
            );
            response
        }
        Err(error) => failure(error, lang),
    }
}

async fn publish(
    db: PgPool,
    context: CheckContext,
    authorization: MutationAuthorization,
    publication_key: Uuid,
    command: Command,
    lang: &'static str,
) -> Response {
    let anchor = match command {
        Command::Create(_) | Command::Transition(_, Transition::Update, _) => "incident",
        Command::Transition(_, Transition::Resolve, _) => "resolved",
        Command::Schedule(_) => "maintenance",
    };
    match store::mutate(&db, context, &authorization, publication_key, command).await {
        Ok(_) if anchor == "resolved" => {
            Redirect::to(&format!("/admin/status?lang={lang}#active-incidents")).into_response()
        }
        Ok(id) => Redirect::to(&format!("/admin/status?lang={lang}#{anchor}-{id}")).into_response(),
        Err(error) => failure(error, lang),
    }
}

async fn create(
    State(db): State<PgPool>,
    Extension(context): Extension<CheckContext>,
    Extension(authorization): Extension<MutationAuthorization>,
    Query(query): Query<LanguageQuery>,
    headers: HeaderMap,
    Form(form): Form<Publication<CreateIncident>>,
) -> Response {
    publish(
        db,
        context,
        authorization,
        form.publication_key,
        Command::Create(form.input),
        query.language(&headers),
    )
    .await
}

async fn transition(
    State((db, transition)): State<(PgPool, Transition)>,
    Extension(context): Extension<CheckContext>,
    Extension(authorization): Extension<MutationAuthorization>,
    Path(id): Path<Uuid>,
    Query(query): Query<LanguageQuery>,
    headers: HeaderMap,
    Form(form): Form<Publication<Message>>,
) -> Response {
    publish(
        db,
        context,
        authorization,
        form.publication_key,
        Command::Transition(id, transition, form.input),
        query.language(&headers),
    )
    .await
}

async fn schedule(
    State(db): State<PgPool>,
    Extension(context): Extension<CheckContext>,
    Extension(authorization): Extension<MutationAuthorization>,
    Query(query): Query<LanguageQuery>,
    headers: HeaderMap,
    Form(form): Form<Publication<ScheduleMaintenance>>,
) -> Response {
    publish(
        db,
        context,
        authorization,
        form.publication_key,
        Command::Schedule(form.input),
        query.language(&headers),
    )
    .await
}
