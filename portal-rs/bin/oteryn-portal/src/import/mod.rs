//! Offline cutover importer. Source values and driver errors never reach diagnostics.
mod characters;
mod commerce;
mod content;
mod identity;
mod source;
mod support;
#[cfg(test)]
mod tests;

use serde_json::{Map, Value, json};
use source::{Dataset, Source};
use sqlx::{Connection, PgConnection, Row};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

type Result<T> = std::result::Result<T, &'static str>;
type Record = Map<String, Value>;
const AREAS: [&str; 5] = ["identity", "content", "commerce", "support", "characters"];

#[derive(PartialEq)]
struct Options {
    areas: Vec<&'static str>,
    source_url: String,
    apply: bool,
}
impl Options {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut args = args.into_iter();
        let area = args.next().ok_or("missing import area")?;
        let areas = if area == "all" {
            AREAS.to_vec()
        } else {
            vec![
                AREAS
                    .into_iter()
                    .find(|name| *name == area)
                    .ok_or("invalid import area")?,
            ]
        };
        let mut source_url = None;
        let mut apply = false;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--source-url" if source_url.is_none() => {
                    source_url = Some(args.next().ok_or("missing source URL")?)
                }
                "--apply" if !apply => apply = true,
                _ => return Err("invalid import arguments"),
            }
        }
        let source_url = source_url.ok_or("missing source URL")?;
        if !source_url.starts_with("mysql://") {
            return Err("source must use MySQL");
        }
        Ok(Self {
            areas,
            source_url,
            apply,
        })
    }
}

pub async fn run() -> std::process::ExitCode {
    // Do not format Options, URLs, source rows or database errors, even at debug level.
    match execute().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(code) => {
            eprintln!("import refused: {code}");
            std::process::ExitCode::FAILURE
        }
    }
}
async fn execute() -> Result<()> {
    let options = Options::parse(std::env::args().skip(2))?;
    let tables: BTreeSet<_> = options
        .areas
        .iter()
        .flat_map(|a| tables(a))
        .copied()
        .collect();
    if options.apply
        && options.areas.contains(&"commerce")
        && std::env::var("PORTAL_IMPORT_COMMERCE_ENABLED").as_deref() != Ok("true")
    {
        return Err("commerce import flag disabled");
    }
    if options.apply
        && options.areas.contains(&"characters")
        && std::env::var("PORTAL_IMPORT_CHARACTERS_ENABLED").as_deref() != Ok("true")
    {
        return Err("character import flag disabled");
    }
    let data = source::MysqlSource::open(&options.source_url)
        .await?
        .snapshot(&tables)
        .await?;
    let url = std::env::var("DATABASE_URL").map_err(|_| "destination URL missing")?;
    let mut db = PgConnection::connect_with(
        &url.parse::<sqlx::postgres::PgConnectOptions>()
            .map_err(|_| "invalid destination URL")?
            .disable_statement_logging(),
    )
    .await
    .map_err(|_| "destination connection failed")?;
    let mut refused = false;
    for area in options.areas {
        let report = import_area(&mut db, &data, area, options.apply).await?;
        for (table, count) in report.counts {
            println!("{area} {table}: {count} rows");
        }
        for problem in &report.problems {
            println!("{area} refusal: {problem}");
        }
        println!(
            "{area}: {} ({} mapped rows)",
            if report.problems.is_empty() {
                if options.apply { "applied" } else { "dry-run" }
            } else {
                "refused"
            },
            report.mapped
        );
        refused |= !report.problems.is_empty();
        if refused && options.apply {
            break;
        }
    }
    if refused {
        Err("mapping problems; no changes committed in refused area")
    } else {
        Ok(())
    }
}
use sqlx::ConnectOptions;
fn tables(area: &str) -> &'static [&'static str] {
    match area {
        "identity" => identity::TABLES,
        "content" => content::TABLES,
        "commerce" => commerce::TABLES,
        "support" => support::TABLES,
        "characters" => characters::TABLES,
        _ => &[],
    }
}
fn key(table: &str, row: &Record) -> Result<String> {
    let columns: &[&str] = match table {
        "admin_role_permissions" => &["role_id", "permission_id"],
        "identity_admin_roles" => &["identity_id", "role_id"],
        "identity_recovery_keys" | "wallet_accounts" | "premium_time_authority" => &["identity_id"],
        _ => &["id"],
    };
    columns
        .iter()
        .map(|c| text(row, c))
        .collect::<Result<Vec<_>>>()
        .map(|v| v.join(":"))
}
#[derive(Default)]
struct Context {
    ids: BTreeMap<(String, String), Uuid>,
    existing: BTreeSet<(String, String)>,
    reused_roles: BTreeSet<String>,
    permissions: BTreeMap<String, String>,
}
impl Context {
    fn id(&self, table: &str, key: &str) -> Result<Value> {
        self.ids
            .get(&(table.to_owned(), key.to_owned()))
            .map(|id| json!(id.to_string()))
            .ok_or("unmapped foreign key")
    }
    fn reference(&self, row: &Record, column: &str, table: &str) -> Result<Value> {
        if row.get(column).is_none_or(Value::is_null) {
            return Ok(Value::Null);
        }
        self.id(table, &text(row, column)?)
    }
}
struct Insert {
    table: &'static str,
    values: Record,
}
fn insert(table: &'static str, values: Value) -> Result<Insert> {
    let mut values = values.as_object().cloned().ok_or("invalid mapped record")?;
    for field in ["created_at", "updated_at"] {
        if values.get(field).is_some_and(Value::is_null) {
            values.remove(field);
        }
    }
    Ok(Insert { table, values })
}

fn copy(row: &Record, columns: &[&str], target: &mut Record) {
    for column in columns {
        if let Some(v) = row.get(*column) {
            target.insert((*column).to_owned(), v.clone());
        }
    }
}
fn text(row: &Record, column: &str) -> Result<String> {
    match row.get(column) {
        Some(Value::String(s)) if !s.is_empty() => Ok(s.clone()),
        Some(Value::Number(n)) => Ok(n.to_string()),
        _ => Err("missing or invalid required field"),
    }
}
fn number(row: &Record, column: &str) -> Result<i64> {
    text(row, column)?.parse().map_err(|_| "invalid integer")
}
fn flag(row: &Record, column: &str, default: bool) -> Result<bool> {
    match row.get(column) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Bool(b)) => Ok(*b),
        Some(v) if v == "1" || v == &json!(1) => Ok(true),
        Some(v) if v == "0" || v == &json!(0) => Ok(false),
        _ => Err("invalid boolean"),
    }
}
fn hex(value: &str) -> Result<Vec<u8>> {
    if !value.len().is_multiple_of(2) || !value.is_ascii() {
        return Err("invalid hexadecimal value");
    }
    (0..value.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&value[i..i + 2], 16).map_err(|_| "invalid hexadecimal value"))
        .collect()
}
fn bytes(bytes: &[u8]) -> Value {
    json!(format!(
        "\\x{}",
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    ))
}
fn mapped_id(ctx: &Context, table: &str, row: &Record) -> Result<Value> {
    ctx.id(table, &key(table, row)?)
}
fn standard(
    ctx: &Context,
    source: &str,
    row: &Record,
    target: &'static str,
    fields: &[&str],
) -> Result<Insert> {
    let mut values = Map::new();
    values.insert("id".into(), mapped_id(ctx, source, row)?);
    copy(row, &["created_at", "updated_at"], &mut values);
    copy(row, fields, &mut values);
    Ok(Insert {
        table: target,
        values,
    })
}
#[derive(Default, Debug)]
struct Report {
    counts: BTreeMap<&'static str, usize>,
    problems: Vec<String>,
    mapped: usize,
}

async fn import_area(
    db: &mut PgConnection,
    data: &Dataset,
    area: &str,
    apply: bool,
) -> Result<Report> {
    let mut tx = db
        .begin()
        .await
        .map_err(|_| "destination transaction failed")?;
    sqlx::query(if apply {
        "SET LOCAL TIME ZONE 'UTC'"
    } else {
        "SET TRANSACTION READ ONLY"
    })
    .execute(&mut *tx)
    .await
    .map_err(|_| "transaction setup failed")?;
    sqlx::query("SET LOCAL TIME ZONE 'UTC'")
        .execute(&mut *tx)
        .await
        .map_err(|_| "transaction UTC setup failed")?;
    if apply {
        // Serialize map allocation and every area, including concurrent invocations.
        sqlx::query("SELECT pg_advisory_xact_lock(783428101)")
            .execute(&mut *tx)
            .await
            .map_err(|_| "import lock failed")?;
    }
    let mut report = Report::default();
    let mut ctx = Context::default();
    for row in sqlx::query("SELECT source_table,legacy_id,target_id FROM legacy_import_map")
        .fetch_all(&mut *tx)
        .await
        .map_err(|_| "import map unavailable; migrate destination first")?
    {
        let table: String = row.get(0);
        let key: String = row.get(1);
        let id: Uuid = row.get(2);
        ctx.existing.insert((table.clone(), key.clone()));
        ctx.ids.insert((table, key), id);
    }
    for &table in tables(area) {
        let Some(rows) = data.get(table) else {
            report
                .problems
                .push(format!("{table}: source table missing"));
            continue;
        };
        report.counts.insert(table, rows.len());
        let mut seen = BTreeSet::new();
        for row in rows {
            match key(table, row) {
                Ok(key) if seen.insert(key.clone()) => {
                    let identity = (table.to_owned(), key);
                    if let std::collections::btree_map::Entry::Vacant(entry) =
                        ctx.ids.entry(identity)
                    {
                        let canonical = match table {
                            "identities" => Some("account_id"),
                            "premium_time_entitlements" => Some("id"),
                            "premium_time_entitlement_events" => Some("request_id"),
                            _ => None,
                        };
                        let id = if let Some(field) = canonical {
                            text(row, field)
                                .ok()
                                .and_then(|s| Uuid::parse_str(&s).ok())
                                .filter(|id| !id.is_nil())
                        } else {
                            Some(Uuid::now_v7())
                        };
                        if let Some(id) = id {
                            entry.insert(id);
                        } else {
                            report
                                .problems
                                .push(format!("{table}: invalid canonical identifier"));
                        }
                    }
                }
                _ => report
                    .problems
                    .push(format!("{table}: invalid or duplicate legacy key")),
            }
        }
    }
    if area == "identity" {
        for row in data.get("admin_permissions").into_iter().flatten() {
            if let (Ok(id), Ok(key)) = (text(row, "id"), text(row, "key")) {
                ctx.permissions.insert(id, key);
            }
        }
        for role in data.get("admin_roles").into_iter().flatten() {
            let (Ok(key), Ok(legacy)) = (text(role, "key"), text(role, "id")) else {
                continue;
            };
            if ctx
                .existing
                .contains(&("admin_roles".into(), legacy.clone()))
            {
                continue;
            }
            let target: Option<Uuid> = sqlx::query_scalar(if apply {
                "SELECT id FROM roles WHERE key=$1 FOR UPDATE"
            } else {
                "SELECT id FROM roles WHERE key=$1"
            })
            .bind(key)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| "role lookup failed")?;
            if let Some(target) = target {
                let assigned: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM account_roles WHERE role_id=$1)",
                )
                .bind(target)
                .fetch_one(&mut *tx)
                .await
                .map_err(|_| "role assignment check failed")?;
                if assigned {
                    report.problems.push(
                        "existing role has assignments; refusing to alter permissions".into(),
                    );
                } else {
                    ctx.ids
                        .insert(("admin_roles".into(), legacy.clone()), target);
                    ctx.reused_roles.insert(legacy);
                }
            }
        }
    }
    if area == "commerce" {
        commerce::check(data, &mut report);
    }
    let mut pending = Vec::new();
    for &table in tables(area) {
        for (index, row) in data.get(table).into_iter().flatten().enumerate() {
            let Ok(key) = key(table, row) else {
                continue;
            };
            if ctx.existing.contains(&(table.to_owned(), key.clone())) {
                continue;
            }
            let result = match area {
                "identity" => identity::map(&ctx, table, row),
                "content" => content::map(&ctx, data, table, row),
                "commerce" => commerce::map(&ctx, data, table, row),
                "support" => support::map(&ctx, table, row),
                "characters" => characters::map(&ctx, table, row),
                _ => Err("unknown area"),
            };
            match result {
                Ok(inserts) => pending.push((table, key, inserts)),
                Err(code) => report
                    .problems
                    .push(format!("{table} row {}: {code}", index + 1)),
            }
        }
    }
    report.mapped = pending.len();
    if report.problems.is_empty() {
        if apply {
            for role in &ctx.reused_roles {
                sqlx::query("DELETE FROM role_permissions WHERE role_id=$1")
                    .bind(ctx.ids.get(&("admin_roles".into(), role.clone())))
                    .execute(&mut *tx)
                    .await
                    .map_err(|_| "role permission replacement failed")?;
            }
        }
        for (table, key, inserts) in pending {
            for record in inserts {
                let columns = record.values.keys().cloned().collect::<Vec<_>>().join(",");
                // All identifiers originate in the compiled area mappings, never the source.
                let valid = if apply {
                    let query = format!(
                        "INSERT INTO {} ({columns}) SELECT {columns} FROM jsonb_populate_record(NULL::{},$1)",
                        record.table, record.table
                    );
                    sqlx::query(sqlx::AssertSqlSafe(query))
                        .bind(json!(record.values))
                        .execute(&mut *tx)
                        .await
                        .is_ok()
                } else {
                    validate_record(&mut tx, &record).await.unwrap_or(false)
                };
                if !valid {
                    report.problems.push(format!(
                        "{table}: destination type, constraint or conflict refusal"
                    ));
                    tx.rollback().await.map_err(|_| "rollback failed")?;
                    return Ok(report);
                }
            }
            if apply {
                sqlx::query("INSERT INTO legacy_import_map(source_table,legacy_id,target_id,area) VALUES($1,$2,$3,$4)")
                    .bind(table).bind(&key).bind(ctx.ids.get(&(table.to_owned(),key)).copied().ok_or("missing allocated identifier")?).bind(area)
                    .execute(&mut *tx).await.map_err(|_| "import map write failed")?;
            }
        }
    }
    if apply && report.problems.is_empty() {
        tx.commit().await.map_err(|_| "commit failed")?;
    } else {
        tx.rollback().await.map_err(|_| "rollback failed")?;
    }
    Ok(report)
}

// Evaluate PostgreSQL types, defaults, NOT NULL and CHECK constraints without writes.
// Foreign-key identifiers are checked by the planner; uniqueness is enforced at apply.
async fn validate_record(db: &mut PgConnection, record: &Insert) -> Result<bool> {
    let attributes=sqlx::query("SELECT a.attname,a.attnotnull,pg_get_expr(d.adbin,d.adrelid) AS default_expr FROM pg_attribute a LEFT JOIN pg_attrdef d ON d.adrelid=a.attrelid AND d.adnum=a.attnum WHERE a.attrelid=$1::regclass AND a.attnum>0 AND NOT a.attisdropped AND a.attgenerated='' AND a.attidentity=''")
        .bind(record.table).fetch_all(&mut *db).await.map_err(|_| "destination schema unavailable")?;
    let mut defaults = Vec::new();
    let mut checks = Vec::new();
    for column in attributes {
        let name: String = column.get("attname");
        if column.get::<bool, _>("attnotnull") {
            checks.push(format!("{name} IS NOT NULL"));
        }
        if let Some(expression) = column.get::<Option<String>, _>("default_expr") {
            defaults.push(format!("'{name}',({expression})"));
        }
    }
    let constraints: Vec<String>=sqlx::query_scalar("SELECT pg_get_expr(conbin,conrelid) FROM pg_constraint WHERE conrelid=$1::regclass AND contype='c'")
        .bind(record.table).fetch_all(&mut *db).await.map_err(|_| "destination constraints unavailable")?;
    checks.extend(
        constraints
            .into_iter()
            .map(|expression| format!("COALESCE(({expression}),true)")),
    );
    let predicate = if checks.is_empty() {
        "true".to_owned()
    } else {
        checks.join(" AND ")
    };
    let defaults = defaults.join(",");
    let query = format!(
        "SELECT {predicate} FROM jsonb_populate_record(NULL::{},jsonb_build_object({defaults}) || $1::jsonb)",
        record.table
    );
    sqlx::query_scalar(sqlx::AssertSqlSafe(query))
        .bind(json!(record.values))
        .fetch_one(db)
        .await
        .map_err(|_| "destination record invalid")
}
