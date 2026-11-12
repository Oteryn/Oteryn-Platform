//! Operator role `oteryn-portal bootstrap-intent issue [--input <file>]`
//! (character bootstrap-intent producer contract v3, *Operator command*).
//!
//! Reads one JSON object from `--input <file>` or stdin, issues the intent through
//! `oteryn_characters::bootstrap::issue` and prints the stored canonical intent on
//! stdout; an exact retry prints the same bytes. Errors are one JSON object on
//! stderr and never carry configuration values, the database URL or the intent.
//!
//! Requires `GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_ENABLED=true` (default off) and
//! `GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_TTL_SECONDS` (1..=300, no default).
//!
//! | exit | meaning |
//! |---|---|
//! | 0 | issued, or the stored decision for an exact retry |
//! | 2 | bad input: usage, JSON, a field, or an Identity that is not active |
//! | 3 | conflict: the operation id was issued with another binding |
//! | 4 | unavailable: disabled, configuration, database or authority state |
use oteryn_characters::bootstrap::issue::{self, Context, IssueError, IssueInput};
use oteryn_internal_api::canonical::{self, Value};
use serde_json::json;
use sqlx::PgPool;
use std::{io::Read, path::PathBuf, process::ExitCode};

pub const ENABLED: &str = "GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_ENABLED";
pub const TTL_SECONDS: &str = "GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_TTL_SECONDS";
const MAX_INPUT_BYTES: usize = 16 * 1024;
const USAGE: &str = "usage: oteryn-portal bootstrap-intent issue [--input <file>]";
const FIELDS: [&str; 9] = [
    "identityId",
    "operationId",
    "targetWorldId",
    "requestedName",
    "requestedSex",
    "profileRevision",
    "rulesetRevision",
    "contentRevision",
    "starterTemplateRevision",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    BadInput(String),
    Conflict,
    Unavailable(&'static str),
}
impl CliError {
    pub fn code(&self) -> u8 {
        match self {
            Self::BadInput(_) => 2,
            Self::Conflict => 3,
            Self::Unavailable(_) => 4,
        }
    }
    fn body(&self) -> serde_json::Value {
        match self {
            Self::BadInput(reason) => json!({"error": "bad_input", "reason": reason}),
            Self::Conflict => json!({
                "error": "conflict",
                "reason": "operationId was issued with a different binding",
            }),
            Self::Unavailable(reason) => json!({"error": "unavailable", "reason": reason}),
        }
    }
}
impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.body())
    }
}
impl std::error::Error for CliError {}
impl From<IssueError> for CliError {
    fn from(error: IssueError) -> Self {
        match error {
            IssueError::BadInput(reason) => Self::BadInput(reason),
            IssueError::Conflict => Self::Conflict,
            IssueError::Unavailable(reason) => Self::Unavailable(reason),
            IssueError::Db => Self::Unavailable("database"),
        }
    }
}
fn bad(reason: &str) -> CliError {
    CliError::BadInput(reason.to_owned())
}

/// Where the request JSON comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Stdin,
    File(PathBuf),
}

/// `args` are the words after `bootstrap-intent`.
pub fn parse_args(args: &[String]) -> Result<Source, CliError> {
    match args {
        [action] if action == "issue" => Ok(Source::Stdin),
        [action, flag, path] if action == "issue" && flag == "--input" && !path.is_empty() => {
            Ok(Source::File(PathBuf::from(path)))
        }
        _ => Err(bad(USAGE)),
    }
}

/// The issuer's environment, read once per invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    pub enabled: bool,
    pub ttl_seconds: Option<i64>,
}
impl Settings {
    pub fn from_values(enabled: Option<&str>, ttl: Option<&str>) -> Self {
        Self {
            enabled: enabled == Some("true"),
            ttl_seconds: ttl.and_then(|raw| {
                raw.parse::<i64>()
                    .ok()
                    .filter(|ttl| ttl.to_string() == raw)
                    .filter(|ttl| (1..=issue::MAX_TTL_SECONDS).contains(ttl))
            }),
        }
    }
    fn from_env() -> Self {
        Self::from_values(
            std::env::var(ENABLED).ok().as_deref(),
            std::env::var(TTL_SECONDS).ok().as_deref(),
        )
    }
    /// The feature flag is default off; the TTL has no default (contract).
    fn ttl(&self) -> Result<i64, CliError> {
        if !self.enabled {
            return Err(CliError::Unavailable(
                "bootstrap-intent issuance is disabled",
            ));
        }
        self.ttl_seconds.ok_or(CliError::Unavailable(
            "GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_TTL_SECONDS must be 1..=300",
        ))
    }
}

/// Parses one bounded JSON object with exactly the request fields, all strings.
pub fn parse_input(raw: &[u8], ttl_seconds: i64) -> Result<IssueInput, CliError> {
    let value = canonical::decode(raw, MAX_INPUT_BYTES, 2)
        .map_err(|_| bad("input must be one flat JSON object of at most 16 KiB"))?;
    let Value::Object(members) = &value else {
        return Err(bad("input must be a JSON object"));
    };
    if let Some((key, _)) = members
        .iter()
        .find(|(key, _)| !FIELDS.contains(&key.as_str()))
    {
        return Err(CliError::BadInput(format!("unknown field {key:?}")));
    }
    let text = |key: &str| match value.get(key) {
        Some(Value::String(text)) => Ok(text.clone()),
        Some(_) => Err(CliError::BadInput(format!("{key} must be a string"))),
        None => Err(CliError::BadInput(format!("{key} is required"))),
    };
    Ok(IssueInput {
        identity_id: text("identityId")?,
        operation_id: text("operationId")?,
        target_world_id: text("targetWorldId")?,
        requested_name: text("requestedName")?,
        requested_sex: text("requestedSex")?,
        context: Context {
            profile_revision: text("profileRevision")?,
            ruleset_revision: text("rulesetRevision")?,
            content_revision: text("contentRevision")?,
            starter_template_revision: text("starterTemplateRevision")?,
        },
        ttl_seconds,
    })
}

/// Validates and issues; returns the stored canonical intent bytes.
pub async fn execute(pool: &PgPool, settings: Settings, raw: &[u8]) -> Result<String, CliError> {
    let ttl = settings.ttl()?;
    let input = parse_input(raw, ttl)?;
    Ok(issue::issue(pool, input).await?.intent_json)
}

fn read(source: &Source) -> Result<Vec<u8>, CliError> {
    let mut raw = Vec::new();
    let limit = u64::try_from(MAX_INPUT_BYTES + 1).unwrap_or(u64::MAX);
    let read = match source {
        Source::Stdin => std::io::stdin().lock().take(limit).read_to_end(&mut raw),
        Source::File(path) => {
            std::fs::File::open(path).and_then(|file| file.take(limit).read_to_end(&mut raw))
        }
    };
    read.map_err(|_| bad("input cannot be read"))?;
    Ok(raw)
}

async fn run(args: &[String]) -> Result<String, CliError> {
    let source = parse_args(args)?;
    let settings = Settings::from_env();
    let ttl = settings.ttl()?;
    let raw = read(&source)?;
    // Input errors are reported before any database configuration is read.
    parse_input(&raw, ttl)?;
    let config = oteryn_core::Config::from_env()
        .map_err(|_| CliError::Unavailable("OTERYN_DATABASE_URL is not configured"))?;
    let pool = oteryn_db::lazy_pool(config.database_url())
        .map_err(|_| CliError::Unavailable("database"))?;
    let result = execute(&pool, settings, &raw).await;
    pool.close().await;
    result
}

/// Entry point; stdout carries only the intent, stderr only a JSON error.
pub async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(2).collect();
    match run(&args).await {
        Ok(intent) => {
            println!("{intent}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{}", error.body());
            ExitCode::from(error.code())
        }
    }
}

#[cfg(test)]
#[path = "bootstrap_intent_tests.rs"]
mod tests;
