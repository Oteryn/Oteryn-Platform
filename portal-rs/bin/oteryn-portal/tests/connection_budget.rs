//! Exercise the composed binary, rather than isolated module routers.
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    process::{Child, Command, Stdio},
    time::Duration,
};

struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn request(address: std::net::SocketAddr, path: &str) -> std::io::Result<String> {
    let mut socket = TcpStream::connect_timeout(&address, Duration::from_secs(2))?;
    socket.set_read_timeout(Some(Duration::from_secs(5)))?;
    write!(
        socket,
        "GET {path} HTTP/1.1\r\nHost: localhost\r\nCookie: __Host-oteryn_session=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\r\nConnection: close\r\n\r\n"
    )?;
    let mut response = String::new();
    socket.read_to_string(&mut response)?;
    Ok(response)
}
#[tokio::test]
async fn composed_modules_share_one_postgres_connection_budget()
-> Result<(), Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED PostgreSQL binary test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let started = std::time::Instant::now();
    let root = sqlx::PgPool::connect(&url).await?;
    let schema = format!("pool_budget_{}", uuid::Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let search = schema.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .after_connect(move |connection, _| {
            let search = search.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(search)
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    let account = uuid::Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,'composition@example.test','test-only','active','en',now())")
        .bind(account).execute(&pool).await?;
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,user_agent_summary,csrf_token,idle_expires_at,absolute_expires_at) VALUES($1,$2,'composition','bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',now()+interval '1 hour',now()+interval '1 day')")
        .bind(Sha256::digest(b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").to_vec()).bind(account).execute(&pool).await?;
    let separator = if url.contains('?') { "&" } else { "?" };
    let connection_url =
        format!("{url}{separator}options=-c%20search_path%3D{schema}&application_name={schema}");
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?;
    drop(listener);
    let mut command = Command::new(env!("CARGO_BIN_EXE_oteryn-portal"));
    for (key, _) in std::env::vars().filter(|(key, _)| {
        key.starts_with("OTERYN_")
            || key.starts_with("PORTAL_")
            || key.starts_with("GAME_AUTH_")
            || key == "APP_ENV"
            || key == "PANEL_PREMIUM_PAYMENTS"
    }) {
        command.env_remove(key);
    }
    command
        .arg("serve")
        .env("OTERYN_DATABASE_URL", connection_url.as_str())
        .env("OTERYN_BIND", address.to_string())
        .env("OTERYN_ENV", "testing")
        .env("APP_ENV", "testing")
        .env("OTERYN_PUBLIC_ORIGIN", "https://localhost")
        .env("PORTAL_PUBLIC_PROJECTIONS_AUTHORITY", "local-test")
        .env("PORTAL_PUBLIC_PROJECTIONS_SAN", "local-test")
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    for flag in [
        "PORTAL_WORLDS_ENABLED",
        "PORTAL_ONLINE_ENABLED",
        "PORTAL_PUBLIC_RANKINGS_ENABLED",
        "PORTAL_PUBLIC_PROFILES_ENABLED",
        "PORTAL_GUILDS_ENABLED",
        "PORTAL_HOUSES_SERVING_ENABLED",
        "PORTAL_DEATHS_ENABLED",
        "PORTAL_PUBLIC_PROJECTIONS_ENABLED",
        "PORTAL_PARTY_ENABLED",
        "OTERYN_DAILY_PUZZLES_ENABLED",
        "OTERYN_COLLECTION_ENABLED",
        "OTERYN_SHOP_ENABLED",
        "OTERYN_WALLET_ENABLED",
        "PANEL_PREMIUM_PAYMENTS",
        "OTERYN_ACCOUNT_RECOVERY_KEYS",
        "OTERYN_WORLD_ENABLED",
    ] {
        command.env(flag, "true");
    }
    let mut server = Server(command.spawn()?);
    let mut ready = false;
    for _ in 0..100 {
        if request(address, "/healthz").is_ok() {
            ready = true;
            break;
        }
        if server.0.try_wait()?.is_some() {
            let mut diagnostic = String::new();
            if let Some(mut stderr) = server.0.stderr.take() {
                stderr.read_to_string(&mut diagnostic)?;
            }
            return Err(format!("portal exited before readiness: {diagnostic}").into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(ready, "portal must become ready");
    // Lock a relation owned by each module, then observe this binary's
    // request waiting on our lock. Startup and unrelated recovery reads cannot
    // satisfy this per-request proof of module SQL execution.
    for (path, table, status, content) in [
        ("/en/news", "news", "200", "News"),
        ("/en/events", "events", "200", "Events"),
        ("/en/worlds", "world_status", "200", "World status"),
        ("/en/online", "public_projection_state", "503", "Online"),
        (
            "/en/characters/Composition",
            "public_snapshots",
            "503",
            "Character",
        ),
        (
            "/en/rankings/experience",
            "public_snapshots",
            "503",
            "Rankings",
        ),
        ("/en/party", "pd_party_posts", "200", "Looking for a team"),
        ("/en/puzzles", "puzzles_daily", "200", "puzzle"),
        ("/en/shop", "store_active_catalog", "200", "Shop"),
        (
            "/en/collection/019ad000-0000-7000-8000-000000000001",
            "accounts",
            "503",
            "Monster collection",
        ),
        ("/en/wiki", "wiki_guides", "200", "Wiki"),
        ("/en/rules", "pages", "200", "not been published"),
        (
            "/en/server-information",
            "pages",
            "200",
            "not been published",
        ),
    ] {
        let mut blocker = pool.begin().await?;
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *blocker)
            .await?;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "LOCK TABLE {table} IN ACCESS EXCLUSIVE MODE"
        )))
        .execute(&mut *blocker)
        .await?;
        let probe = tokio::task::spawn_blocking(move || request(address, path));
        let observed = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE application_name=$1 AND $2=ANY(pg_blocking_pids(pid)))")
                    .bind(&schema).bind(pid).fetch_one(&root).await?;
                if waiting { return Ok::<_, sqlx::Error>(()); }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }).await;
        blocker.rollback().await?;
        let response = probe.await??;
        assert!(
            matches!(observed, Ok(Ok(()))),
            "{path} did not execute its module database path: {response}"
        );
        assert!(
            response.starts_with(&format!("HTTP/1.1 {status}")),
            "{path}: {response}"
        );
        let main = response
            .split_once("<main")
            .and_then(|(_, body)| body.split_once("</main>"))
            .map(|(body, _)| body)
            .ok_or("module page must contain main")?;
        assert!(
            main.contains(content),
            "{path} missing {content}: {response}"
        );
    }
    // Search/collection landings are static. G-4 guilds/houses/deaths are
    // deliberately refused before SQL while their contracts are CANDIDATE;
    // preserve those gates and never count them as database execution evidence.
    for (path, status, content) in [
        ("/en/characters", "200", "Character"),
        ("/en/collection", "200", "Monster collection"),
        ("/en/guilds", "503", "Guild information is unavailable."),
        ("/en/houses", "503", "House information is unavailable."),
        (
            "/en/deaths",
            "503",
            "Death records are currently unavailable.",
        ),
    ] {
        let response = request(address, path)?;
        assert!(
            response.starts_with(&format!("HTTP/1.1 {status}")),
            "{path}: {response}"
        );
        assert!(response.contains(content), "{path}: {response}");
    }
    let recovery = request(address, "/en/recovery-key")?;
    assert!(recovery.starts_with("HTTP/1.1 200"));
    assert!(recovery.contains("Account recovery key"));
    assert!(recovery.contains("name=\"_csrf\""));
    let mut socket = TcpStream::connect_timeout(&address, Duration::from_secs(2))?;
    socket.set_read_timeout(Some(Duration::from_secs(5)))?;
    write!(
        socket,
        "POST /en/recovery-key HTTP/1.1\r\nHost: localhost\r\nOrigin: https://localhost\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )?;
    let mut refused = String::new();
    socket.read_to_string(&mut refused)?;
    assert!(refused.starts_with("HTTP/1.1 403"));
    let connections: i64 =
        sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE application_name=$1")
            .bind(&schema)
            .fetch_one(&root)
            .await?;
    assert!(connections > 0, "module reads must exercise PostgreSQL");
    assert!(
        connections <= 10,
        "module composition exceeded its one-pool connection budget: {connections}"
    );
    drop(server);
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    root.close().await;
    eprintln!(
        "PG composed_modules_share_one_postgres_connection_budget duration={:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
