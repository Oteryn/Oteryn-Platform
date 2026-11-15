use super::*;
use ed25519_dalek::Signer;
use rcgen::{BasicConstraints, CertificateParams, ExtendedKeyUsagePurpose, IsCa, KeyPair};
use std::fmt::Write as _;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

struct Ca(rcgen::Certificate, KeyPair);
impl Ca {
    fn new() -> Result<Self> {
        let key = KeyPair::generate()?;
        let mut params = CertificateParams::new(Vec::<String>::new())?;
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        Ok(Self(params.self_signed(&key)?, key))
    }
    fn issue(
        &self,
        usage: ExtendedKeyUsagePurpose,
    ) -> Result<(CertificateDer<'static>, PrivateKeyDer<'static>)> {
        let key = KeyPair::generate()?;
        let mut params = CertificateParams::new(vec!["localhost".into()])?;
        params.extended_key_usages = vec![usage];
        let cert = params.signed_by(&key, &self.0, &self.1)?;
        Ok((
            cert.der().clone(),
            PrivateKeyDer::try_from(key.serialize_der())?,
        ))
    }
}

async fn call(
    addr: std::net::SocketAddr,
    roots: rustls::RootCertStore,
    client: Option<&(CertificateDer<'static>, PrivateKeyDer<'static>)>,
    path: &str,
) -> Result<Option<String>> {
    let builder = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])?
    .with_root_certificates(roots);
    let mut tls = match client {
        Some((cert, key)) => builder.with_client_auth_cert(vec![cert.clone()], key.clone_key())?,
        None => builder.with_no_client_auth(),
    };
    tls.alpn_protocols = vec![b"http/1.1".to_vec()];
    let connector = tokio_rustls::TlsConnector::from(Arc::new(tls));
    let tcp = tokio::net::TcpStream::connect(addr).await?;
    let Ok(mut stream) = connector.connect("localhost".try_into()?, tcp).await else {
        return Ok(None);
    };
    let mut signature = String::new();
    for byte in ed25519_dalek::SigningKey::from_bytes(&[7; 32])
        .sign(b"{}")
        .to_bytes()
    {
        write!(signature, "{byte:02x}")?;
    }
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nX-Oteryn-Key-Id: tls-test\r\nX-Oteryn-Signature: {signature}\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}"
    );
    if stream.write_all(request.as_bytes()).await.is_err() {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    // TLS close without close_notify is valid refusal; preserve any HTTP bytes.
    let _ = tokio::time::timeout(Duration::from_secs(3), stream.read_to_end(&mut bytes)).await?;
    Ok(if bytes.is_empty() {
        None
    } else {
        Some(String::from_utf8(bytes)?)
    })
}

#[tokio::test]
async fn dedicated_closure_listener_refuses_untrusted_unpinned_and_cross_purpose_clients() -> Result
{
    let started = std::time::Instant::now();
    let db = crate::reset_http::tests::db::Database::open()
        .await?
        .ok_or("PORTAL_TEST_DATABASE_URL required")?;
    let ca = Ca::new()?;
    let foreign = Ca::new()?.issue(ExtendedKeyUsagePurpose::ClientAuth)?;
    let unpinned = ca.issue(ExtendedKeyUsagePurpose::ClientAuth)?;
    let command = ca.issue(ExtendedKeyUsagePurpose::ClientAuth)?;
    let portfolio = ca.issue(ExtendedKeyUsagePurpose::ClientAuth)?;
    let (server, key) = ca.issue(ExtendedKeyUsagePurpose::ServerAuth)?;
    let mut roots = rustls::RootCertStore::empty();
    roots.add(ca.0.der().clone())?;
    let tls = tls_config(roots.clone(), vec![server], key)?;
    let ([character, portfolio_id], identities) = purpose_identities()?;
    let public_key = ed25519_dalek::SigningKey::from_bytes(&[7; 32]).verifying_key();
    let config = runtime::Config::load(|_| None)?;
    let config = runtime::Config {
        enabled: true,
        public_key: Some(public_key),
        recovery_generation: Some(1),
        key_id: "tls-test".into(),
        ..config
    };
    let game = config
        .boundary(Arc::new(termination::UnavailableBoundary), true)?
        .ok_or("boundary")?;
    let app = routes(
        db.pool.clone(),
        game,
        &config.key_id,
        identities,
        oteryn_characters::lcfa::Settings {
            enabled: true,
            environment: oteryn_core::Environment::Testing,
            source_authority: config.authority,
            ..Default::default()
        },
    );
    let socket = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = socket.local_addr()?;
    let listener = serve(
        socket,
        tls,
        app,
        [
            Sha256::digest(command.0.as_ref()).to_vec(),
            Sha256::digest(portfolio.0.as_ref()).to_vec(),
        ],
        [character, portfolio_id],
    );
    let closure_paths = [
        "/internal/v1/game-auth/account-closure/lifecycle/read",
        "/internal/v1/game-auth/character-commands/pending",
        "/internal/v1/game-auth/character-commands/read",
        "/internal/v1/game-auth/character-commands/receipt",
        "/internal/v1/game-auth/account-closure",
        "/internal/v1/game-auth/account-closure/watermark",
    ];
    let portfolio_paths = [
        "/internal/v1/game-auth/native-account-characters",
        "/internal/v1/game-auth/native-account-characters/watermark",
    ];
    for path in closure_paths.iter().chain(&portfolio_paths) {
        for client in [None, Some(&foreign), Some(&unpinned)] {
            assert!(
                call(address, roots.clone(), client, path).await?.is_none(),
                "unauthorized transport reached {path}"
            );
        }
    }
    for (allowed, denied, paths) in [
        (&command, &portfolio, closure_paths.as_slice()),
        (&portfolio, &command, portfolio_paths.as_slice()),
    ] {
        for path in paths {
            let response = call(address, roots.clone(), Some(denied), path)
                .await?
                .ok_or("expected purpose refusal")?;
            assert!(response.starts_with("HTTP/1.1 401"), "{path}: {response}");
            // Positive controls reach payload validation for every route.
            let response = call(address, roots.clone(), Some(allowed), path)
                .await?
                .ok_or("valid pin refused")?;
            assert!(response.starts_with("HTTP/1.1 400"), "{path}: {response}");
        }
    }
    for table in [
        "plat3_closure_work",
        "plat3_closure_commands",
        "plat3_closure_receipts",
        "plat3_closure_snapshots",
        "account_character_snapshots",
        "account_characters",
    ] {
        let count: i64 =
            sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
                .fetch_one(&db.pool)
                .await?;
        assert_eq!(count, 0, "refused client wrote {table}");
    }
    listener.abort();
    db.close().await?;
    eprintln!(
        "PG dedicated_closure_listener_refuses_untrusted_unpinned_and_cross_purpose_clients duration={:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
