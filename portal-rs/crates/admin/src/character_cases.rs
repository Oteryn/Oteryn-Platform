//! Platform review of support evidence for staff character operations.
use crate::{
    audit::{self, AuditContext, Entry},
    rbac::{self, AccountStateReader, Authorizer, CheckContext, MutationAuthorization, Permission},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{PgConnection, PgPool, Postgres, Transaction, types::Json};
use uuid::Uuid;

pub(crate) mod web;

const SUBMIT: Permission = Permission::ManageSupportTickets;
const REVIEW: Permission = Permission::ReviewCharacterCases;

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("invalid character case")]
    Invalid,
    #[error("character case access denied")]
    Denied,
    #[error("character case not found")]
    Missing,
    #[error("character case state changed")]
    Conflict,
    #[error("character case storage unavailable")]
    Storage(#[from] sqlx::Error),
}

impl From<rbac::Error> for Error {
    fn from(error: rbac::Error) -> Self {
        match error {
            rbac::Error::Storage(error) => Self::Storage(error),
            _ => Self::Denied,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Operation {
    Create,
    Rename,
    Schedule,
    Restore,
    Repair,
}

impl Operation {
    const ALL: [Self; 5] = [
        Self::Create,
        Self::Rename,
        Self::Schedule,
        Self::Restore,
        Self::Repair,
    ];

    fn key(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Rename => "rename",
            Self::Schedule => "schedule",
            Self::Restore => "restore",
            Self::Repair => "repair",
        }
    }

    fn has_character(self) -> bool {
        !matches!(self, Self::Create)
    }

    fn has_name(self) -> bool {
        matches!(self, Self::Create | Self::Rename)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
struct Profile {
    #[serde(skip_serializing_if = "Option::is_none")]
    character: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_revision: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vocation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    schedule: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    destination: Option<Uuid>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Submission {
    #[serde(rename = "_csrf")]
    _csrf: String,
    id: Uuid,
    ticket: Uuid,
    world: Uuid,
    reviewer: Uuid,
    operation: Operation,
    evidence: String,
    character: Option<Uuid>,
    expected_revision: Option<u64>,
    name: Option<String>,
    sex: Option<String>,
    vocation: Option<String>,
    schedule: Option<Uuid>,
    destination: Option<Uuid>,
}

impl Submission {
    fn validate(&self) -> Result<Profile, Error> {
        use oteryn_characters::{
            commands::{Sex, Vocation, services::display_name},
            create_web::canonicalize,
        };

        let create = matches!(self.operation, Operation::Create);
        if [self.id, self.ticket, self.world, self.reviewer]
            .iter()
            .any(Uuid::is_nil)
            || [Some(self.world), self.character, self.schedule]
                .into_iter()
                .flatten()
                .any(|id| id.get_version_num() != 7 || id.get_variant() != uuid::Variant::RFC4122)
            || self.destination.is_some_and(|id| id.is_nil())
            || self.character.is_some() != self.operation.has_character()
            || self.expected_revision.is_some() != self.operation.has_character()
            || self.expected_revision == Some(0)
            || self.name.is_some() != self.operation.has_name()
            || self.sex.is_some() != create
            || self.vocation.is_some() != create
            || self.schedule.is_some() != matches!(self.operation, Operation::Restore)
            || self.destination.is_some() != matches!(self.operation, Operation::Repair)
        {
            return Err(Error::Invalid);
        }
        bounded(&self.evidence, 160)?;
        let name = match self.name.as_deref().map(str::trim) {
            Some(value) if create => Some(canonicalize(value).map_err(|_| Error::Invalid)?),
            Some(value) => {
                display_name(value).map_err(|_| Error::Invalid)?;
                Some(value.to_owned())
            }
            None => None,
        };
        if self
            .sex
            .as_deref()
            .is_some_and(|value| Sex::parse(value).is_err())
            || self
                .vocation
                .as_deref()
                .is_some_and(|value| Vocation::parse(value).is_err())
        {
            return Err(Error::Invalid);
        }
        Ok(Profile {
            character: self.character,
            expected_revision: self.expected_revision,
            name,
            sex: self.sex.clone(),
            vocation: self.vocation.clone(),
            schedule: self.schedule,
            destination: self.destination,
        })
    }
}

fn bounded(value: &str, max: usize) -> Result<&str, Error> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max || value.chars().any(char::is_control) {
        return Err(Error::Invalid);
    }
    Ok(value)
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Decision {
    Accept,
    Return,
}

impl Decision {
    fn key(self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::Return => "return",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Review {
    #[serde(rename = "_csrf")]
    _csrf: String,
    revision: i64,
    decision: Decision,
    note: String,
}

#[derive(Debug, sqlx::FromRow)]
struct Case {
    id: Uuid,
    ticket_id: Uuid,
    account_id: Uuid,
    requester: Uuid,
    reviewer: Uuid,
    world_id: Uuid,
    operation: String,
    intent: Json<Profile>,
    evidence_ref: String,
    revision: i64,
    decision: Option<String>,
    decision_note: Option<String>,
    created_at: time::OffsetDateTime,
    decided_at: Option<time::OffsetDateTime>,
}

impl Case {
    fn state(&self) -> &str {
        self.decision.as_deref().unwrap_or("pending")
    }
}

async fn load(pool: &PgPool, actor: Uuid, id: Uuid) -> Result<Case, Error> {
    sqlx::query_as(
        "SELECT * FROM staff_character_cases WHERE id=$1 AND $2 IN (requester, reviewer)",
    )
    .bind(id)
    .bind(actor)
    .fetch_optional(pool)
    .await?
    .ok_or(Error::Missing)
}

async fn ticket_account(pool: &PgPool, ticket: Uuid) -> Result<Uuid, Error> {
    sqlx::query_scalar("SELECT account_id FROM tickets WHERE id=$1 AND category='character'")
        .bind(ticket)
        .fetch_optional(pool)
        .await?
        .ok_or(Error::Missing)
}

async fn lock_ticket(c: &mut PgConnection, ticket: Uuid, account: Uuid) -> Result<(), Error> {
    let status: Option<String> = sqlx::query_scalar(
        "SELECT status FROM tickets WHERE id=$1 AND account_id=$2 AND category='character' FOR UPDATE",
    )
    .bind(ticket)
    .bind(account)
    .fetch_optional(c)
    .await?;
    match status.as_deref() {
        Some("open" | "waiting_user" | "waiting_staff") => Ok(()),
        Some(_) => Err(Error::Conflict),
        None => Err(Error::Missing),
    }
}

async fn submit<A: AccountStateReader>(
    pool: &PgPool,
    auth: &Authorizer<A>,
    authority: &MutationAuthorization,
    actor: CheckContext,
    form: Submission,
) -> Result<Uuid, Error> {
    let profile = form.validate()?;
    let evidence = bounded(&form.evidence, 160)?;
    let account = ticket_account(pool, form.ticket).await?;
    if form.reviewer == actor.account || [actor.account, form.reviewer].contains(&account) {
        return Err(Error::Denied);
    }
    if profile.destination == Some(account) {
        return Err(Error::Invalid);
    }
    let mut accounts = vec![account, form.reviewer];
    accounts.extend(profile.destination);
    let mut tx = pool.begin().await?;
    rbac::mutation_budget(&mut tx).await?;
    let ticket = form.ticket;
    rbac::authorize_mutation_with_resources_domain(
        &mut tx,
        &accounts,
        move |c| Box::pin(lock_ticket(c, ticket, account)),
        authority,
        actor,
        &[Permission::Access, SUBMIT],
    )
    .await?;
    for permission in [Permission::Access, SUBMIT, REVIEW] {
        auth.check_subject_in(&mut tx, form.reviewer, permission)
            .await?;
    }
    if let Some(destination) = profile.destination
        && (!auth.account_active_in(&mut tx, destination).await
            || [actor.account, form.reviewer].contains(&destination))
    {
        return Err(Error::Denied);
    }

    let inserted = sqlx::query(
        "INSERT INTO staff_character_cases
         (id, ticket_id, account_id, requester, reviewer, world_id, operation, intent, evidence_ref)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) ON CONFLICT (id) DO NOTHING",
    )
    .bind(form.id)
    .bind(ticket)
    .bind(account)
    .bind(actor.account)
    .bind(form.reviewer)
    .bind(form.world)
    .bind(form.operation.key())
    .bind(Json(&profile))
    .bind(evidence)
    .execute(&mut *tx)
    .await?;
    if inserted.rows_affected() == 0 {
        let identical: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM staff_character_cases
             WHERE id=$1 AND requester=$2 AND ticket_id=$3 AND account_id=$4 AND reviewer=$5
             AND world_id=$6 AND operation=$7 AND intent=$8 AND evidence_ref=$9)",
        )
        .bind(form.id)
        .bind(actor.account)
        .bind(ticket)
        .bind(account)
        .bind(form.reviewer)
        .bind(form.world)
        .bind(form.operation.key())
        .bind(Json(&profile))
        .bind(evidence)
        .fetch_one(&mut *tx)
        .await?;
        if !identical {
            return Err(Error::Conflict);
        }
    } else {
        record(
            &mut tx,
            actor.account,
            form.id,
            "staff.character_case.submitted",
            json!(null),
            json!({"ticket": ticket, "reviewer": form.reviewer, "world": form.world,
                   "operation": form.operation.key(), "state": "pending"}),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(form.id)
}

async fn decide<A: AccountStateReader>(
    pool: &PgPool,
    auth: &Authorizer<A>,
    authority: &MutationAuthorization,
    actor: CheckContext,
    id: Uuid,
    form: Review,
) -> Result<(), Error> {
    if id.is_nil() || form.revision <= 0 {
        return Err(Error::Invalid);
    }
    let note = bounded(&form.note, 500)?;
    let case = load(pool, actor.account, id).await?;
    if case.reviewer != actor.account || case.requester == actor.account {
        return Err(Error::Denied);
    }
    let mut tx = pool.begin().await?;
    rbac::mutation_budget(&mut tx).await?;
    let (ticket, account) = (case.ticket_id, case.account_id);
    rbac::authorize_mutation_with_resources_domain(
        &mut tx,
        &[account, case.requester],
        move |c| Box::pin(lock_ticket(c, ticket, account)),
        authority,
        actor,
        &[Permission::Access, SUBMIT, REVIEW],
    )
    .await?;
    for permission in [Permission::Access, SUBMIT] {
        auth.check_subject_in(&mut tx, case.requester, permission)
            .await?;
    }
    let updated = sqlx::query(
        "UPDATE staff_character_cases
         SET decision=$2, decision_note=$3, decided_at=clock_timestamp(), revision=revision+1
         WHERE id=$1 AND reviewer=$4 AND decision IS NULL AND revision=$5",
    )
    .bind(id)
    .bind(form.decision.key())
    .bind(note)
    .bind(actor.account)
    .bind(form.revision)
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(Error::Conflict);
    }
    record(
        &mut tx,
        actor.account,
        id,
        "staff.character_case.reviewed",
        json!({"state": "pending"}),
        json!({"ticket": ticket, "operation": case.operation, "state": form.decision.key()}),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn record(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    id: Uuid,
    action: &str,
    before: Value,
    after: Value,
) -> Result<(), Error> {
    audit::append(
        tx,
        Entry {
            context: AuditContext {
                actor: Some(actor),
                request_id: Uuid::now_v7(),
            },
            action,
            target: &id.to_string(),
            before,
            after,
        },
    )
    .await?;
    Ok(())
}

#[cfg(test)]
#[path = "character_cases_tests.rs"]
mod tests;
