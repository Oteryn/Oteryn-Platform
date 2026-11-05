//! Starts the real binary with default flags and checks that it serves.
//! Router assembly happens at startup, so an overlapping route between two
//! modules only shows up as a panic here, not in per-crate tests.
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn get(port: u16, path: &str) -> Option<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
    )
    .ok()?;
    let mut response = String::new();
    stream.read_to_string(&mut response).ok()?;
    Some(response)
}

#[test]
fn serve_starts_and_answers_health_and_legacy_redirects() -> Result<(), Box<dyn std::error::Error>>
{
    let port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();
    let mut server = Server(
        Command::new(env!("CARGO_BIN_EXE_oteryn-portal"))
            .arg("serve")
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .env("OTERYN_ENV", "testing")
            .env("OTERYN_BIND", format!("127.0.0.1:{port}"))
            .env("OTERYN_PUBLIC_ORIGIN", format!("http://127.0.0.1:{port}"))
            // Pools are lazy; startup must not need a reachable database.
            .env(
                "OTERYN_DATABASE_URL",
                "postgres://startup-test@127.0.0.1:1/startup_test",
            )
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?,
    );
    let deadline = Instant::now() + Duration::from_secs(30);
    let health = loop {
        if let Some(status) = server.0.try_wait()? {
            let mut stderr = String::new();
            if let Some(mut pipe) = server.0.stderr.take() {
                pipe.read_to_string(&mut stderr)?;
            }
            return Err(format!("server exited early ({status}): {stderr}").into());
        }
        if let Some(response) = get(port, "/healthz") {
            break response;
        }
        if Instant::now() > deadline {
            return Err("server did not answer /healthz within 30 s".into());
        }
        std::thread::sleep(Duration::from_millis(200));
    };
    assert!(health.starts_with("HTTP/1.1 200"), "{health}");
    let legacy = get(port, "/servers").ok_or("no response for /servers")?;
    assert!(legacy.starts_with("HTTP/1.1 301"), "{legacy}");
    assert!(
        legacy.to_ascii_lowercase().contains("location: /pl/worlds"),
        "{legacy}"
    );
    let unknown = get(port, "/no-such-page").ok_or("no response")?;
    assert!(unknown.starts_with("HTTP/1.1 404"), "{unknown}");
    Ok(())
}
