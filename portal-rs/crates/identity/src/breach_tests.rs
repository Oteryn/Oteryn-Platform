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
    sqlx::query("INSERT INTO registration_password_ranges(prefix,suffixes,expires_at,complete) VALUES($1,ARRAY[]::text[],now()+interval '1 hour',true) ON CONFLICT(prefix) DO UPDATE SET suffixes=ARRAY[]::text[],expires_at=now()+interval '1 hour',complete=true")
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
