//! Exercise the composed binary, rather than isolated module routers.
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
        "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
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
    for path in [
        "/en/news",
        "/en/events",
        "/en/worlds",
        "/en/online",
        "/en/guilds",
        "/en/houses",
        "/en/deaths",
        "/en/characters",
        "/en/rankings/experience",
        "/en/party",
        "/en/puzzles",
        "/en/collection",
        "/en/shop",
        "/en/wiki",
        "/en/rules",
        "/en/server-information",
    ] {
        let response = request(address, path)?;
        assert!(
            !response.starts_with("HTTP/1.1 500"),
            "{path} must not fail internally"
        );
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
    Ok(())
}
