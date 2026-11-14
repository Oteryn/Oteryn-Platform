#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! §19.4, §19.7 and §19.8 wire bytes against the protocol 3 fixtures.
use super::*;

const FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../docs/contracts/fixtures/native-gateway-protocol3-v1/"
);
const ATTEMPT: &str = "0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b";

fn fixture(name: &str) -> Vec<u8> {
    let raw = std::fs::read(format!("{FIXTURES}{name}")).unwrap();
    raw.strip_suffix(b"\n").unwrap_or(&raw).to_vec()
}

#[test]
fn request_fixture_decodes_and_binds_character_and_offer() {
    let raw = fixture("recover-request.json");
    let request = RecoveryRequest::decode(&raw, false).unwrap();
    assert_eq!(request.attempt_ref.to_string(), ATTEMPT);
    // The digest binds the offer: reordering members keeps it, a change does not.
    let text = String::from_utf8(raw).unwrap();
    let changed = text.replace(
        "\"client_build\":\"0.1.0+abc123\"",
        "\"client_build\":\"0.1.1+abc123\"",
    );
    assert_ne!(changed, text);
    match RecoveryRequest::decode(changed.as_bytes(), false) {
        Ok(other) => assert_ne!(other.digest, request.digest),
        Err(error) => assert_eq!(error, RecoveryError::OfferUnsupported),
    }
}

#[test]
fn bad_requests_are_refused() {
    let text = String::from_utf8(fixture("recover-request.json")).unwrap();
    let cases: Vec<(String, RecoveryError)> = vec![
        (
            text.replace("\"protocol_version\":3", "\"protocol_version\":2"),
            RecoveryError::UnsupportedVersion,
        ),
        (text.replace("otr1.", "otr2."), RecoveryError::Malformed),
        (
            text.replacen('{', "{\"device_proof\":\"x\",", 1),
            RecoveryError::Malformed,
        ),
        (
            text.replacen('{', "{\"channel_id\":null,", 1),
            RecoveryError::Malformed,
        ),
        (
            text.replace(ATTEMPT, "0192B3C4-5D6E-7F80-9A1B-2C3D4E5F6A7B"),
            RecoveryError::Malformed,
        ),
        ("[]".into(), RecoveryError::Malformed),
        ("{".into(), RecoveryError::Malformed),
    ];
    for (raw, expected) in cases {
        assert_eq!(
            RecoveryRequest::decode(raw.as_bytes(), false).unwrap_err(),
            expected,
            "{raw}"
        );
    }
}

/// §19.15.2: the proof input is the JCS of the fixture's five members, and the
/// fixture signature verifies under the synthetic key (seed [7; 32]).
#[test]
fn device_proof_fixture_decodes_and_signs_the_contract_input() {
    let raw = fixture("recover-request-device-proof.json");
    let request = RecoveryRequest::decode(&raw, true).unwrap();
    let plain = RecoveryRequest::decode(&fixture("recover-request.json"), false).unwrap();
    // The proof is outside the request digest; the rest is the same request.
    assert_eq!(request.digest, plain.digest);
    assert_eq!(request.handle_hash, plain.handle_hash);
    let proof = request.proof.as_ref().unwrap();
    let key = ed25519_dalek::SigningKey::from_bytes(&[7; 32]).verifying_key();
    let thumbprint = URL_SAFE_NO_PAD.encode(Sha256::digest(key.as_bytes()));
    assert_eq!(proof.kid, thumbprint);
    let input = proof_input(&request, &thumbprint);
    assert_eq!(
        input.as_bytes(),
        fixture("recover-proof-input.json").as_slice()
    );
    key.verify_strict(input.as_bytes(), &Signature::from_bytes(&proof.sig))
        .unwrap();
    // Binding on requires the member; binding off refuses it.
    assert_eq!(
        RecoveryRequest::decode(&fixture("recover-request.json"), true).unwrap_err(),
        RecoveryError::Malformed
    );
    assert_eq!(
        RecoveryRequest::decode(&raw, false).unwrap_err(),
        RecoveryError::Malformed
    );
    let text = String::from_utf8(raw).unwrap();
    let kid = format!("\"kid\":\"{thumbprint}\"");
    let sig = text
        .split("\"sig\":\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .unwrap()
        .to_owned();
    let mut padded = thumbprint.clone();
    padded.push('=');
    let mut noncanonical = sig.clone();
    noncanonical.pop();
    // The last of 86 characters carries 2 bits; 'B' sets a trailing bit.
    noncanonical.push(if sig.ends_with('B') { 'C' } else { 'B' });
    for bad in [
        text.replace(&kid, &kid.replace(&thumbprint, &thumbprint[..42])),
        text.replace(&kid, &kid.replace(&thumbprint, &padded)),
        text.replace(&kid, "\"kid\":43"),
        text.replace(&sig, &sig[..85]),
        text.replace(&sig, &noncanonical),
        text.replace(&sig, &sig.replace('-', "+").replace('_', "/")),
        text.replace(&kid, &format!("{kid},\"alg\":\"EdDSA\"")),
        text.replace(&format!("{kid},"), ""),
        text.replace(
            &format!("\"device_proof\":{{{kid},\"sig\":\"{sig}\"}}"),
            "\"device_proof\":null",
        ),
    ] {
        assert_ne!(bad, text);
        assert_eq!(
            RecoveryRequest::decode(bad.as_bytes(), true).unwrap_err(),
            RecoveryError::Malformed,
            "{bad}"
        );
    }
}

#[test]
fn success_matches_fixture() {
    let expected = fixture("recover-success.json");
    let value: serde_json::Value = serde_json::from_slice(&expected).unwrap();
    let endpoint: Endpoint = serde_json::from_value(value["endpoint"].clone()).unwrap();
    let body = success(
        value["attempt_ref"].as_str().unwrap().parse().unwrap(),
        value["world_id"].as_str().unwrap().parse().unwrap(),
        value["channel_id"].as_str().unwrap().parse().unwrap(),
        &endpoint,
        value["grant"]["token"].as_str().unwrap(),
        value["grant"]["valid_for_seconds"].as_i64().unwrap(),
    )
    .unwrap();
    assert_eq!(body, expected);
    assert!(body.len() <= MAX_RESPONSE);
}

#[test]
fn errors_match_fixtures() {
    let attempt = Some(ATTEMPT);
    for (name, error, attempt) in [
        (
            "recover-request-malformed.json",
            RecoveryError::Malformed,
            attempt,
        ),
        (
            "recover-authentication-required.json",
            RecoveryError::HandleRejected,
            attempt,
        ),
        (
            "recover-attempt-conflict.json",
            RecoveryError::AttemptConflict,
            attempt,
        ),
        (
            "recover-attempt-pending.json",
            RecoveryError::AttemptPending,
            attempt,
        ),
        (
            "recover-character-conflict.json",
            RecoveryError::CharacterConflict,
            attempt,
        ),
        (
            "recover-world-stale.json",
            RecoveryError::WorldStale,
            attempt,
        ),
        (
            "recover-route-unavailable.json",
            RecoveryError::RouteUnavailable,
            attempt,
        ),
        (
            "recover-grant-expired.json",
            RecoveryError::GrantExpired,
            attempt,
        ),
        (
            "recover-reconciliation-required.json",
            RecoveryError::ReconciliationRequired,
            attempt,
        ),
        (
            "recover-unavailable.json",
            RecoveryError::Unavailable,
            attempt,
        ),
        (
            "recover-unsupported-version.json",
            RecoveryError::UnsupportedVersion,
            attempt,
        ),
        (
            "recover-offer-unsupported.json",
            RecoveryError::OfferUnsupported,
            attempt,
        ),
        (
            "recover-rate-limited.json",
            RecoveryError::RateLimited,
            attempt,
        ),
        (
            "recover-revoke-malformed.json",
            RecoveryError::Malformed,
            None,
        ),
    ] {
        assert_eq!(error.body(attempt, 3), fixture(name), "{name}");
    }
    // Every SECURITY_TERMINAL reason has the same public bytes (§19.8).
    for error in [
        RecoveryError::DeviceProofRejected,
        RecoveryError::SecurityDenied,
    ] {
        assert_eq!(
            error.body(attempt, 3),
            fixture("recover-authentication-required.json")
        );
    }
    let statuses = [
        (RecoveryError::Malformed, 400),
        (RecoveryError::UnsupportedVersion, 400),
        (RecoveryError::HandleRejected, 401),
        (RecoveryError::DeviceProofRejected, 401),
        (RecoveryError::SecurityDenied, 401),
        (RecoveryError::OfferUnsupported, 409),
        (RecoveryError::AttemptConflict, 409),
        (RecoveryError::AttemptPending, 409),
        (RecoveryError::CharacterConflict, 409),
        (RecoveryError::WorldStale, 409),
        (RecoveryError::GrantExpired, 409),
        (RecoveryError::RateLimited, 429),
        (RecoveryError::RouteUnavailable, 503),
        (RecoveryError::ReconciliationRequired, 503),
        (RecoveryError::Unavailable, 503),
    ];
    for (error, status) in statuses {
        assert_eq!(error.status().as_u16(), status, "{error:?}");
    }
}

#[test]
fn revoke_fixtures() {
    let hash = decode_revoke(&fixture("recover-revoke-request.json")).unwrap();
    let request = RecoveryRequest::decode(&fixture("recover-request.json"), false).unwrap();
    assert_eq!(hash, request.handle_hash);
    assert_eq!(REVOKED, fixture("recover-revoke-success.json").as_slice());
    assert_eq!(
        decode_revoke(br#"{"protocol_version":3,"recovery_handle":"otr1.short"}"#).unwrap_err(),
        RecoveryError::Malformed
    );
    assert_eq!(
        decode_revoke(br#"{"protocol_version":3}"#).unwrap_err(),
        RecoveryError::Malformed
    );
}

#[test]
fn claims_are_in_contract_order() {
    let report = canonical::decode(
        br#"{"content_revision":"c1","map_revision":"m1","ruleset_revision":"r1","world_policy_revision":"p1"}"#,
        1024,
        5,
    )
    .unwrap();
    let attempt: Uuid = ATTEMPT.parse().unwrap();
    let bytes = payload(&Claims {
        iat: 1_790_000_000,
        ttl: 30,
        jti: "A".repeat(43),
        attempt,
        account: attempt,
        character: attempt,
        world: attempt,
        security_generation: 7,
        report: &report,
    })
    .unwrap();
    let text = String::from_utf8(bytes).unwrap();
    let a = ATTEMPT;
    assert_eq!(
        text,
        format!(
            r#"{{"iss":"urn:oteryn:platform:game-recovery","aud":"urn:oteryn:game:recovery","iat":1790000000,"nbf":1790000000,"exp":1790000030,"jti":"{jti}","profile":"oteryn-reauth-recovery-v1","purpose":"existing_actor_recovery","attempt_ref":"{a}","account_id":"{a}","character_id":"{a}","world_id":"{a}","account_security_generation":"7","protocol_major":1,"transport_profile":1,"ruleset_revision":"r1","content_revision":"c1","map_revision":"m1","world_policy_revision":"p1"}}"#,
            jti = "A".repeat(43)
        )
    );
}

/// Account state straight from `accounts`, locked like the real lookup.
#[derive(Debug)]
struct Accounts;
impl AccountStateLookup for Accounts {
    fn state<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        account: Uuid,
    ) -> store::Future<'a, AccountState> {
        Box::pin(async move {
            let status: Option<String> =
                sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR UPDATE")
                    .bind(account)
                    .fetch_optional(connection)
                    .await
                    .map_err(|_| Error::Unavailable)?;
            Ok(match status.as_deref() {
                Some("active") => AccountState::Active,
                _ => AccountState::Unknown,
            })
        })
    }
}

/// A migrated schema of its own, dropped by [`close`].
async fn database() -> Option<(PgPool, PgPool, String)> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED recovery PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return None;
    };
    let admin = oteryn_db::lazy_pool(&url).unwrap();
    let schema = format!("recovery_test_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await
        .unwrap();
    let path = schema.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .after_connect(move |conn, _| {
            let path = path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(path)
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .unwrap();
    oteryn_db::migrate(&pool).await.unwrap();
    Some((admin, pool, schema))
}
async fn close(admin: PgPool, pool: PgPool, schema: String) {
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}
async fn check(pool: &PgPool, request: &RecoveryRequest) -> Result<(), RecoveryError> {
    let mut conn = pool.acquire().await.unwrap();
    authenticate(&mut conn, request, &Accounts, 2_000_000_000)
        .await
        .map(|_| ())
}
async fn count(pool: &PgPool, sql: &'static str, account: Uuid) -> i64 {
    sqlx::query_scalar(sql)
        .bind(account)
        .fetch_one(pool)
        .await
        .unwrap()
}
/// Re-arms the fixture handle at the account's current generation.
async fn rearm(pool: &PgPool, account: Uuid) {
    sqlx::query("UPDATE native_recovery_handles SET revoked_at=NULL, security_generation=(SELECT native_security_generation FROM accounts WHERE id=$1) WHERE account_id=$1")
        .bind(account)
        .execute(pool)
        .await
        .unwrap();
}

/// §19.3: every credential or status change ends the account's handles in its
/// own transaction, a retry of an outstanding attempt is then refused, the
/// attempt record stays until retirement, and the rows go with their account
/// or device family (ON DELETE CASCADE).
#[tokio::test]
async fn postgres_security_change_revokes_handles_and_refuses_retry() {
    let Some((admin, pool, schema)) = database().await else {
        return;
    };
    let request = RecoveryRequest::decode(&fixture("recover-request.json"), false).unwrap();
    let account: Uuid = sqlx::query_scalar("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES(uuidv7(),$1,'hash','active','en',now()) RETURNING id")
        .bind(format!("{}@example.test", Uuid::now_v7()))
        .fetch_one(&pool).await.unwrap();
    sqlx::query("INSERT INTO native_play_generations(account_id,generation) VALUES($1,1)")
        .bind(account)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO native_recovery_handles(handle_hash,account_id,character_id,world_id,channel_id,security_generation,play_generation,attempt_ref,issued_at,expires_at) VALUES($1,$2,$3,uuidv7(),uuidv7(),1,1,$4,1,9999999999)")
        .bind(&request.handle_hash[..]).bind(account).bind(request.character_id).bind(request.attempt_ref)
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO native_recoveries(attempt_ref,handle_hash,request_digest,account_id,response_body,expires_at_ms,retires_at_ms) VALUES($1,$2,$3,$4,'\\x7b7d',9999999999000,9999999999000)")
        .bind(request.attempt_ref).bind(&request.handle_hash[..]).bind(&request.digest[..]).bind(account)
        .execute(&pool).await.unwrap();
    rearm(&pool, account).await;
    check(&pool, &request).await.unwrap();
    // Bad input: an unknown handle is refused like a revoked one.
    let mut unknown = RecoveryRequest::decode(&fixture("recover-request.json"), false).unwrap();
    unknown.handle_hash = [0; 32];
    assert_eq!(
        check(&pool, &unknown).await.unwrap_err(),
        RecoveryError::HandleRejected
    );
    // A change that is not a credential keeps the handle.
    sqlx::query("UPDATE accounts SET locale='pl' WHERE id=$1")
        .bind(account)
        .execute(&pool)
        .await
        .unwrap();
    check(&pool, &request).await.unwrap();
    for change in [
        "password_hash='changed'",
        "email='changed-' || email",
        "email_verified_at=email_verified_at - interval '1 second'",
        "login_mfa_required=true",
        "login_mfa_required=false",
        "native_security_generation=native_security_generation+1",
        "game_auth_generation=game_auth_generation+1",
        "status='suspended'",
    ] {
        rearm(&pool, account).await;
        check(&pool, &request).await.unwrap();
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE accounts SET {change} WHERE id=$1"
        )))
        .bind(account)
        .execute(&pool)
        .await
        .unwrap();
        let revoked: bool = sqlx::query_scalar(
            "SELECT revoked_at IS NOT NULL FROM native_recovery_handles WHERE handle_hash=$1",
        )
        .bind(&request.handle_hash[..])
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(revoked, "{change}");
        assert!(check(&pool, &request).await.is_err(), "{change}");
    }
    // The attempt record is kept until retirement (§6.1, §19.4 step 3).
    let records = "SELECT count(*) FROM native_recoveries WHERE account_id=$1";
    let handles = "SELECT count(*) FROM native_recovery_handles WHERE account_id=$1";
    assert_eq!(count(&pool, records, account).await, 1);
    // A device-bound handle goes with its family.
    let family: Uuid = sqlx::query_scalar("INSERT INTO device_sessions(id,account_id,client_id,enrollment_token_id,game_generation,native_generation,absolute_expires_at,idle_expires_at) VALUES(uuidv7(),$1,uuidv7(),'recovery-test',1,1,9999999999,9999999999) RETURNING id")
        .bind(account).fetch_one(&pool).await.unwrap();
    sqlx::query("INSERT INTO native_recovery_handles(handle_hash,account_id,character_id,world_id,channel_id,security_generation,play_generation,attempt_ref,device_family_id,device_key_thumbprint,issued_at,expires_at) VALUES(decode(repeat('ab',32),'hex'),$1,uuidv7(),uuidv7(),uuidv7(),1,1,uuidv7(),$2,repeat('a',43),1,9999999999)")
        .bind(account).bind(family).execute(&pool).await.unwrap();
    assert_eq!(count(&pool, handles, account).await, 2);
    sqlx::query("DELETE FROM device_sessions WHERE id=$1")
        .bind(family)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(count(&pool, handles, account).await, 1);
    // Handles and attempt records go with their account.
    sqlx::query("DELETE FROM accounts WHERE id=$1")
        .bind(account)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(count(&pool, handles, account).await, 0);
    assert_eq!(count(&pool, records, account).await, 0);
    close(admin, pool, schema).await;
}
