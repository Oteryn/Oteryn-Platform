use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
/// Private PostgreSQL schema per test, dropped on close.
struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn open() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED breach PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = oteryn_db::lazy_pool(&url)?;
        let schema = format!("breach_test_{}", uuid::Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(4)
            .after_connect(move |conn, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1 || ',pg_catalog',false)")
                        .bind(path)
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        Ok(Some(Self {
            pool,
            admin,
            schema,
        }))
    }
    async fn close(self) -> TestResult {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await?;
        self.admin.close().await;
        Ok(())
    }
}
type TestResult = Result<(), Box<dyn std::error::Error>>;

/// What the local range mock answers.
#[derive(Clone)]
pub(crate) enum Reply {
    Ok(String),
    Status(u16),
    Hang,
}
/// Local HTTP range mock on 127.0.0.1 that counts requests and keeps their heads.
pub(crate) struct Mock {
    pub base: String,
    hits: Arc<AtomicUsize>,
    heads: Arc<std::sync::Mutex<Vec<String>>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Mock {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Mock {
    pub async fn start(reply: Reply) -> std::io::Result<Self> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let base = format!("http://{}/range/", listener.local_addr()?);
        let hits = Arc::new(AtomicUsize::new(0));
        let heads = Arc::new(std::sync::Mutex::new(Vec::new()));
        let (counter, log) = (hits.clone(), heads.clone());
        let task = tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let (counter, log, reply) = (counter.clone(), log.clone(), reply.clone());
                tokio::spawn(async move {
                    let mut head = Vec::new();
                    let mut buf = [0_u8; 1024];
                    while !head.windows(4).any(|w| w == b"\r\n\r\n") && head.len() < 16384 {
                        match socket.read(&mut buf).await {
                            Ok(0) | Err(_) => return,
                            Ok(n) => head.extend_from_slice(&buf[..n]),
                        }
                    }
                    if let Ok(mut log) = log.lock() {
                        log.push(String::from_utf8_lossy(&head).to_lowercase());
                    }
                    counter.fetch_add(1, Ordering::SeqCst);
                    let (status, body) = match reply {
                        Reply::Ok(body) => (200, body),
                        Reply::Status(status) => (status, String::new()),
                        Reply::Hang => {
                            std::future::pending::<()>().await;
                            return;
                        }
                    };
                    let response = format!(
                        "HTTP/1.1 {status} X\r\ncontent-type: text/plain\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                    let _ = socket.shutdown().await;
                });
            }
        });
        Ok(Self {
            base,
            hits,
            heads,
            task,
        })
    }
    pub fn hits(&self) -> usize {
        self.hits.load(Ordering::SeqCst)
    }
    pub fn checker(&self, pool: &PgPool) -> Checker {
        Checker::new(pool.clone(), self.base.clone(), Duration::from_secs(3))
    }
    fn heads(&self) -> Vec<String> {
        self.heads.lock().map(|h| h.clone()).unwrap_or_default()
    }
}
/// A padded range body: other suffixes, a count-0 padding line for `password` when
/// clean, and its suffix with a positive count when breached. CRLF as HIBP sends it.
pub(crate) fn range(password: &str, breached: bool) -> String {
    let digest = sha1_hex(password);
    let own = &digest[5..];
    let other = if own.starts_with('0') {
        "F".repeat(35)
    } else {
        "0".repeat(35)
    };
    let mut lines = [
        format!("{other}:12"),
        format!("{}:0", "A".repeat(35)),
        format!("{own}:{}", if breached { 3 } else { 0 }),
    ];
    lines.sort();
    lines.join("\r\n")
}
/// Cache a complete clean range for `password` so the check needs no HTTP.
pub(crate) async fn allow(pool: &PgPool, password: &str) -> Result<(), sqlx::Error> {
    let digest = sha1_hex(password);
    sqlx::query("INSERT INTO registration_password_ranges(prefix,suffixes,expires_at,complete,complete_xact) VALUES($1,ARRAY[]::text[],now()+interval '1 hour',true,pg_current_xact_id()) ON CONFLICT(prefix) DO UPDATE SET suffixes=ARRAY[]::text[],expires_at=now()+interval '1 hour',complete=true,complete_xact=pg_current_xact_id()")
        .bind(&digest[..5])
        .execute(pool)
        .await?;
    Ok(())
}
const PASSWORD: &str = "a locally generated breach test passphrase";

#[test]
fn parser_drops_padding_and_accepts_crlf() {
    let a = "0".repeat(35);
    let b = "ABCDEF0123456789ABCDEF0123456789ABC";
    let body = format!("{a}:0\r\n{b}:7\r\n{}:00\r\n", "F".repeat(35));
    assert_eq!(parse(body.as_bytes()), Ok(vec![b.to_owned()]));
    assert_eq!(
        parse(format!("{a}:1\n{b}:2").as_bytes()),
        Ok(vec![a.clone(), b.to_owned()])
    );
    assert_eq!(parse(format!("{a}:0").as_bytes()), Ok(Vec::new()));
}
#[test]
fn parser_rejects_malformed_empty_and_oversized() {
    let a = "0".repeat(35);
    for body in [
        String::new(),
        "\n".into(),
        format!("{a}:1\n\n{a}:2"),
        a.clone(),
        format!("{a}:"),
        format!("{a}:-1"),
        format!("{a}:1 "),
        format!("{}:1", "a".repeat(35)),
        format!("{}:1", "0".repeat(34)),
        format!("{}:1", "0".repeat(36)),
        format!("{}:1", "G".repeat(35)),
        format!("{a}:1:2"),
        format!("{a}:1\r\r\n"),
    ] {
        assert_eq!(
            parse(body.as_bytes()),
            Err(BreachError::Unavailable),
            "{body:?}"
        );
    }
    assert_eq!(parse(&[0xff, b':', b'1']), Err(BreachError::Unavailable));
    let line = format!("{a}:1\n");
    let oversized = line.repeat(MAX_BODY / line.len() + 1);
    assert!(oversized.len() > MAX_BODY);
    assert_eq!(parse(oversized.as_bytes()), Err(BreachError::Unavailable));
}
#[tokio::test]
async fn fetched_range_decides_when_the_cache_is_unusable() -> TestResult {
    // No database: the cache read and the upsert both fail; the fetched range decides.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(200))
        .connect_lazy("postgres://oteryn@127.0.0.1:1/none")?;
    let hit = Mock::start(Reply::Ok(range(PASSWORD, true))).await?;
    assert_eq!(
        check(&hit.checker(&pool), PASSWORD).await,
        Err(BreachError::Breached)
    );
    let miss = Mock::start(Reply::Ok(range(PASSWORD, false))).await?;
    assert_eq!(check(&miss.checker(&pool), PASSWORD).await, Ok(()));
    let down = Mock::start(Reply::Status(503)).await?;
    assert_eq!(
        check(&down.checker(&pool), PASSWORD).await,
        Err(BreachError::Unavailable)
    );
    let refused = Checker::new(pool.clone(), "http://127.0.0.1:1/range/", TIMEOUT);
    assert_eq!(
        check(&refused, PASSWORD).await,
        Err(BreachError::Unavailable)
    );
    for base in [
        "http://example.test/range/",
        "ftp://127.0.0.1/range/",
        "not a url",
    ] {
        let checker = Checker::new(pool.clone(), base, TIMEOUT);
        assert_eq!(
            check(&checker, PASSWORD).await,
            Err(BreachError::Unavailable)
        );
    }
    assert_eq!(
        format!("{:?}", Checker::new(pool, DEFAULT_URL, TIMEOUT)),
        "Checker { base: \"https://api.pwnedpasswords.com/range/\", timeout: 3s, .. }"
    );
    Ok(())
}

async fn row(pool: &PgPool, password: &str) -> Result<Option<(bool, bool)>, sqlx::Error> {
    sqlx::query_as("SELECT complete, expires_at > now()+interval '29 days' FROM registration_password_ranges WHERE prefix=$1")
        .bind(&sha1_hex(password)[..5])
        .fetch_optional(pool)
        .await
}

#[tokio::test]
async fn hit_and_miss_are_fetched_cached_and_served_from_cache() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let p = &db.pool;
    let hit = Mock::start(Reply::Ok(range(PASSWORD, true))).await?;
    assert_eq!(
        check(&hit.checker(p), PASSWORD).await,
        Err(BreachError::Breached)
    );
    assert_eq!(hit.hits(), 1);
    let prefix = &sha1_hex(PASSWORD)[..5];
    let head = &hit.heads()[0];
    assert!(head.starts_with(&format!(
        "get /range/{} http/1.1\r\n",
        prefix.to_lowercase()
    )));
    assert!(head.contains("\r\nadd-padding: true\r\n"), "{head}");
    assert!(head.contains("\r\nuser-agent: oteryn-portal\r\n"), "{head}");
    assert_eq!(row(p, PASSWORD).await?, Some((true, true)));
    // The cached complete range decides without another request.
    assert_eq!(
        check(&hit.checker(p), PASSWORD).await,
        Err(BreachError::Breached)
    );
    assert_eq!(hit.hits(), 1);
    // The cached range is shared: a mock that would answer differently is not asked.
    let miss = Mock::start(Reply::Ok(range(PASSWORD, false))).await?;
    assert_eq!(
        check(&miss.checker(p), PASSWORD).await,
        Err(BreachError::Breached)
    );
    assert_eq!(miss.hits(), 0);
    sqlx::query("DELETE FROM registration_password_ranges")
        .execute(p)
        .await?;
    assert_eq!(check(&miss.checker(p), PASSWORD).await, Ok(()));
    assert_eq!(check(&miss.checker(p), PASSWORD).await, Ok(()));
    assert_eq!(miss.hits(), 1);
    let stored: Vec<String> =
        sqlx::query_scalar("SELECT suffixes FROM registration_password_ranges WHERE prefix=$1")
            .bind(prefix)
            .fetch_one(p)
            .await?;
    assert_eq!(stored.len(), 1, "padding lines must not be cached");
    db.close().await
}

#[tokio::test]
async fn incomplete_and_expired_rows_are_refetched() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let p = &db.pool;
    let digest = sha1_hex(PASSWORD);
    // An operator import (complete=false) that lists the password is ignored.
    sqlx::query("INSERT INTO registration_password_ranges(prefix,suffixes,expires_at) VALUES($1,ARRAY[$2],now()+interval '1 year')")
        .bind(&digest[..5])
        .bind(&digest[5..])
        .execute(p)
        .await?;
    let miss = Mock::start(Reply::Ok(range(PASSWORD, false))).await?;
    assert_eq!(check(&miss.checker(p), PASSWORD).await, Ok(()));
    assert_eq!(miss.hits(), 1);
    assert_eq!(row(p, PASSWORD).await?, Some((true, true)));
    // An expired complete row is fetched again and replaced.
    sqlx::query("UPDATE registration_password_ranges SET expires_at=now()-interval '1 second'")
        .execute(p)
        .await?;
    let hit = Mock::start(Reply::Ok(range(PASSWORD, true))).await?;
    assert_eq!(
        check(&hit.checker(p), PASSWORD).await,
        Err(BreachError::Breached)
    );
    assert_eq!(hit.hits(), 1);
    assert_eq!(row(p, PASSWORD).await?, Some((true, true)));
    // A malformed complete row is not trusted either.
    sqlx::query("UPDATE registration_password_ranges SET suffixes=ARRAY['bad']")
        .execute(p)
        .await?;
    assert_eq!(check(&miss.checker(p), PASSWORD).await, Ok(()));
    assert_eq!(miss.hits(), 2);
    db.close().await
}

/// SQL emitted by the importer before the `complete` column existed, as an operator
/// could still replay it from an older `--min-count 100` download.
const OLD_IMPORT_SQL: &str = "INSERT INTO registration_password_ranges (prefix, suffixes, expires_at) \
     SELECT prefix, suffixes, now() + interval '730 days' FROM import_ranges \
     ON CONFLICT (prefix) DO UPDATE SET suffixes = EXCLUDED.suffixes, \
     expires_at = EXCLUDED.expires_at, updated_at = now()";

#[tokio::test]
async fn old_thresholded_import_over_a_complete_row_does_not_stay_complete() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let p = &db.pool;
    let digest = sha1_hex(PASSWORD);
    let hit = Mock::start(Reply::Ok(range(PASSWORD, true))).await?;
    assert_eq!(
        check(&hit.checker(p), PASSWORD).await,
        Err(BreachError::Breached)
    );
    assert_eq!(row(p, PASSWORD).await?, Some((true, true)));
    // The thresholded subset omits the breached suffix.
    let mut tx = p.begin().await?;
    sqlx::query("CREATE TEMP TABLE import_ranges (prefix text, suffixes text[]) ON COMMIT DROP")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO import_ranges VALUES($1, ARRAY[]::text[])")
        .bind(&digest[..5])
        .execute(&mut *tx)
        .await?;
    sqlx::query(OLD_IMPORT_SQL).execute(&mut *tx).await?;
    tx.commit().await?;
    let (complete, suffixes): (bool, Vec<String>) = sqlx::query_as(
        "SELECT complete, suffixes FROM registration_password_ranges WHERE prefix=$1",
    )
    .bind(&digest[..5])
    .fetch_one(p)
    .await?;
    assert!(suffixes.is_empty());
    assert!(
        !complete,
        "a subset written by the old importer must not decide"
    );
    // The subset row is not trusted: the breached password is still refused, from HIBP.
    assert_eq!(
        check(&hit.checker(p), PASSWORD).await,
        Err(BreachError::Breached)
    );
    assert_eq!(hit.hits(), 2);
    assert_eq!(row(p, PASSWORD).await?, Some((true, true)));
    // A direct update that does not stamp the transaction cannot keep the row complete.
    sqlx::query("UPDATE registration_password_ranges SET suffixes=ARRAY[]::text[]")
        .execute(p)
        .await?;
    assert_eq!(row(p, PASSWORD).await?.map(|r| r.0), Some(false));
    db.close().await
}

async fn complete_of(pool: &PgPool, prefix: &str) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("SELECT complete FROM registration_password_ranges WHERE prefix=$1")
        .bind(prefix)
        .fetch_one(pool)
        .await
}

#[tokio::test]
async fn unstamped_insert_copy_and_inherited_stamp_are_not_complete() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let p = &db.pool;
    // A fresh INSERT that claims completeness without the stamp.
    sqlx::query("INSERT INTO registration_password_ranges(prefix,suffixes,expires_at,complete) VALUES('AAAA1',ARRAY[]::text[],now()+interval '1 day',true)")
        .execute(p)
        .await?;
    assert!(!complete_of(p, "AAAA1").await?);
    // A direct COPY that claims completeness without the stamp.
    let mut conn = p.acquire().await?;
    let mut copy = conn
        .copy_in_raw(
            "COPY registration_password_ranges(prefix,suffixes,expires_at,complete) FROM STDIN",
        )
        .await?;
    copy.send(b"AAAA2\t{}\t2999-01-01 00:00:00+00\tt\n".as_slice())
        .await?;
    copy.finish().await?;
    drop(conn);
    assert!(!complete_of(p, "AAAA2").await?);
    // A stamped write is complete; the stamp is not stored, so a later unstamped write in
    // the same transaction (and in a savepoint) cannot inherit it.
    let mut tx = p.begin().await?;
    sqlx::query("INSERT INTO registration_password_ranges(prefix,suffixes,expires_at,complete,complete_xact) VALUES('AAAA3',ARRAY[]::text[],now()+interval '1 day',true,pg_current_xact_id())")
        .execute(&mut *tx)
        .await?;
    let stamp: Option<String> = sqlx::query_scalar(
        "SELECT complete_xact::text FROM registration_password_ranges WHERE prefix='AAAA3'",
    )
    .fetch_one(&mut *tx)
    .await?;
    assert_eq!(stamp, None);
    sqlx::query("SAVEPOINT s").execute(&mut *tx).await?;
    sqlx::query(
        "UPDATE registration_password_ranges SET suffixes=ARRAY[]::text[] WHERE prefix='AAAA3'",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query("RELEASE SAVEPOINT s").execute(&mut *tx).await?;
    tx.commit().await?;
    assert!(!complete_of(p, "AAAA3").await?);
    db.close().await
}

/// The importer's upsert (`deploy/oracle/password-ranges.py` `emit_sql`, `--min-count 1`).
const NEW_IMPORT_SQL: &str = "INSERT INTO registration_password_ranges (prefix, suffixes, expires_at, complete, complete_xact) \
     SELECT prefix, suffixes, now() + interval '30 days', true, pg_current_xact_id() FROM import_ranges \
     ON CONFLICT (prefix) DO UPDATE SET suffixes = EXCLUDED.suffixes, \
     expires_at = EXCLUDED.expires_at, complete = EXCLUDED.complete, \
     complete_xact = pg_current_xact_id(), updated_at = now() \
     WHERE EXCLUDED.complete OR NOT registration_password_ranges.complete";

#[tokio::test]
async fn importer_full_then_legacy_subset_in_one_transaction_is_not_complete() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let p = &db.pool;
    let suffix = "0".repeat(35);
    let mut tx = p.begin().await?;
    sqlx::query("CREATE TEMP TABLE import_ranges (prefix text, suffixes text[]) ON COMMIT DROP")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO import_ranges VALUES('BBBB1', ARRAY[$1])")
        .bind(&suffix)
        .execute(&mut *tx)
        .await?;
    sqlx::query(NEW_IMPORT_SQL).execute(&mut *tx).await?;
    let full: bool = sqlx::query_scalar(
        "SELECT complete FROM registration_password_ranges WHERE prefix='BBBB1'",
    )
    .fetch_one(&mut *tx)
    .await?;
    assert!(full, "a stamped full import is complete");
    sqlx::query("UPDATE import_ranges SET suffixes=ARRAY[]::text[]")
        .execute(&mut *tx)
        .await?;
    sqlx::query(OLD_IMPORT_SQL).execute(&mut *tx).await?;
    tx.commit().await?;
    assert!(!complete_of(p, "BBBB1").await?);
    // A full import on its own, and the portal's own store, stay complete.
    let mut tx = p.begin().await?;
    sqlx::query("CREATE TEMP TABLE import_ranges (prefix text, suffixes text[]) ON COMMIT DROP")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO import_ranges VALUES('BBBB1', ARRAY[$1]), ('BBBB2', ARRAY[]::text[])")
        .bind(&suffix)
        .execute(&mut *tx)
        .await?;
    sqlx::query(NEW_IMPORT_SQL).execute(&mut *tx).await?;
    tx.commit().await?;
    assert!(complete_of(p, "BBBB1").await?);
    assert!(complete_of(p, "BBBB2").await?);
    store(p, "BBBB3", &[]).await?;
    store(p, "BBBB3", std::slice::from_ref(&suffix)).await?;
    assert!(complete_of(p, "BBBB3").await?);
    db.close().await
}

#[tokio::test]
async fn outage_timeout_and_malformed_body_are_unavailable() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let p = &db.pool;
    let server_error = Mock::start(Reply::Status(500)).await?;
    let hang = Mock::start(Reply::Hang).await?;
    let malformed = Mock::start(Reply::Ok(format!(
        "{}\r\nnot a range line",
        range(PASSWORD, false)
    )))
    .await?;
    let empty = Mock::start(Reply::Ok(String::new())).await?;
    let short = Checker::new(p.clone(), hang.base.clone(), Duration::from_millis(300));
    let started = std::time::Instant::now();
    assert_eq!(check(&short, PASSWORD).await, Err(BreachError::Unavailable));
    assert!(started.elapsed() < Duration::from_secs(3));
    assert_eq!(hang.hits(), 1);
    for mock in [&server_error, &malformed, &empty] {
        assert_eq!(
            check(&mock.checker(p), PASSWORD).await,
            Err(BreachError::Unavailable)
        );
        assert_eq!(mock.hits(), 1);
    }
    // Failures are never cached as a range.
    assert_eq!(row(p, PASSWORD).await?, None);
    // An incomplete operator row does not rescue an outage.
    let digest = sha1_hex(PASSWORD);
    sqlx::query("INSERT INTO registration_password_ranges(prefix,suffixes,expires_at) VALUES($1,ARRAY[]::text[],now()+interval '1 year')")
        .bind(&digest[..5])
        .execute(p)
        .await?;
    assert_eq!(
        check(&server_error.checker(p), PASSWORD).await,
        Err(BreachError::Unavailable)
    );
    db.close().await
}
