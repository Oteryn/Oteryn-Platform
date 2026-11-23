#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use webauthn_authenticator_rs::{WebauthnAuthenticator, softpasskey::SoftPasskey};
#[path = "passkeys_test_db.rs"]
mod database;
const BINDING: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OTHER: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
fn service(pool: PgPool) -> Passkeys {
    Passkeys::new(pool, "example.test", "https://example.test").unwrap()
}
async fn account(pool: &PgPool) -> Uuid {
    sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,email_verified_at,locale) VALUES ($1,'unused','active',now(),'en') RETURNING id")
        .bind(format!("{}@example.test",Uuid::now_v7())).fetch_one(pool).await.unwrap()
}
impl Passkeys {
    async fn fixture_proof(&self, binding: &str, account: Uuid) -> crate::login::Authorization {
        sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES($1,$2,$3,'test',now()+interval '1 hour',now()+interval '1 day') ON CONFLICT(id_hash) DO UPDATE SET account_id=EXCLUDED.account_id,csrf_token=EXCLUDED.csrf_token")
            .bind(hash(binding)).bind(account).bind(binding).execute(&self.pool).await.unwrap();
        let generation =
            sqlx::query_scalar("SELECT native_security_generation FROM accounts WHERE id=$1")
                .bind(account)
                .fetch_one(&self.pool)
                .await
                .unwrap();
        crate::login::Authorization {
            account,
            generation,
        }
    }
    async fn begin_registration(&self, binding: &str, account: Uuid) -> Result<Challenge, Error> {
        if !sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM accounts WHERE id=$1)")
            .bind(account)
            .fetch_one(&self.pool)
            .await?
        {
            return Err(Error::Refused);
        }
        self.begin_registration_authorized(binding, self.fixture_proof(binding, account).await)
            .await
    }
    async fn remove(&self, account: Uuid, id: Uuid) -> Result<(), Error> {
        if !sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM accounts WHERE id=$1)")
            .bind(account)
            .fetch_one(&self.pool)
            .await?
        {
            return Err(Error::Refused);
        }
        self.remove_authorized(BINDING, self.fixture_proof(BINDING, account).await, id)
            .await
    }
}
#[tokio::test]
async fn invalid_configuration_is_refused() {
    let pool = oteryn_db::lazy_pool("postgres://localhost/unused").unwrap();
    for (rp, origin) in [
        ("example.test", "http://example.test"),
        ("other.test", "https://example.test"),
        ("example.test", "https://example.test/path"),
        ("example.test", "https://example.test/?x=1"),
        ("example.test", "invalid"),
    ] {
        assert!(Passkeys::new(pool.clone(), rp, origin).is_err());
    }
}
async fn registration_input(
    service: &Passkeys,
    account: Uuid,
    auth: &mut SoftPasskey,
) -> Completion {
    let start = service.begin_registration(BINDING, account).await.unwrap();
    registration_completion(start, auth)
}
fn registration_completion(start: Challenge, auth: &mut SoftPasskey) -> Completion {
    let mut options = start.options;
    // The software test authenticator is not resident-capable. It still signs genuine proofs.
    options["publicKey"]["authenticatorSelection"]["residentKey"] = "discouraged".into();
    options["publicKey"]["authenticatorSelection"]["requireResidentKey"] = false.into();
    let response = auth
        .do_registration(
            Url::parse("https://example.test").unwrap(),
            decode(options).unwrap(),
        )
        .unwrap();
    Completion {
        token: start.token,
        credential: encode(&response).unwrap(),
    }
}
async fn registered(service: &Passkeys, account: Uuid, auth: &mut SoftPasskey) -> Value {
    let input = registration_input(service, account, auth).await;
    let value = input.credential;
    let token = input.token;
    service
        .finish_registration(
            BINDING,
            account,
            Completion {
                token: token.clone(),
                credential: value.clone(),
            },
        )
        .await
        .unwrap();
    assert!(matches!(
        service
            .finish_registration(
                BINDING,
                account,
                Completion {
                    token,
                    credential: value.clone()
                }
            )
            .await,
        Err(Error::Refused)
    ));
    value
}
async fn assertion(service: &Passkeys, account: Uuid, auth: &mut SoftPasskey) -> Completion {
    assertion_at_origin(service, account, auth, "https://example.test").await
}
async fn assertion_at_origin(
    service: &Passkeys,
    account: Uuid,
    auth: &mut SoftPasskey,
    origin: &str,
) -> Completion {
    let start = service.begin_authentication(BINDING).await.unwrap();
    let key: Value =
        sqlx::query_scalar("SELECT credential FROM account_passkeys WHERE account_id=$1 ORDER BY created_at,id LIMIT 1")
            .bind(account)
            .fetch_one(&service.pool)
            .await
            .unwrap();
    let key: Passkey = decode(key).unwrap();
    let mut options = start.options;
    // Supply the known ID to the non-resident test authenticator, and the browser user handle.
    options["publicKey"]["allowCredentials"] =
        serde_json::json!([{"type":"public-key","id":URL_SAFE_NO_PAD.encode(key.cred_id())}]);
    options.as_object_mut().unwrap().remove("mediation");
    let response = auth
        .do_authentication(Url::parse(origin).unwrap(), decode(options).unwrap())
        .unwrap();
    let mut credential = encode(&response).unwrap();
    credential["response"]["userHandle"] = URL_SAFE_NO_PAD.encode(account.as_bytes()).into();
    Completion {
        token: start.token,
        credential,
    }
}
#[tokio::test]
async fn postgres_registration_login_removal_and_replay() {
    let Some(db) = database::Database::open().await.unwrap() else {
        return;
    };
    let service = service(db.pool.clone());
    let id = account(&db.pool).await;
    let mut auth = SoftPasskey::new(true);
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,user_agent_summary,csrf_token,idle_expires_at,absolute_expires_at) VALUES ($1,$2,'test',$3,now()+interval '1 hour',now()+interval '1 day')")
        .bind(hash(OTHER)).bind(id).bind(BINDING).execute(&db.pool).await.unwrap();
    let before = crate::native_recovery_test_support::handle(&db.pool, id)
        .await
        .unwrap();
    registered(&service, id, &mut auth).await;
    // Registration ends the account's native recovery handles.
    assert!(
        crate::native_recovery_test_support::revoked(&db.pool, &before)
            .await
            .unwrap()
    );
    let sessions: i64 = sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE account_id=$1")
        .bind(id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(sessions, 0);
    let input = assertion(&service, id, &mut auth).await;
    let replay = Completion {
        token: input.token.clone(),
        credential: input.credential.clone(),
    };
    assert_eq!(
        service.finish_authentication(BINDING, input).await.unwrap(),
        id
    );
    assert!(matches!(
        service.finish_authentication(BINDING, replay).await,
        Err(Error::Refused)
    ));
    let key = service.list(id).await.unwrap()[0];
    let handle = crate::native_recovery_test_support::handle(&db.pool, id)
        .await
        .unwrap();
    assert!(matches!(
        service.remove(Uuid::now_v7(), key).await,
        Err(Error::Refused)
    ));
    // A refused removal keeps the handle; a removal ends it.
    assert!(
        !crate::native_recovery_test_support::revoked(&db.pool, &handle)
            .await
            .unwrap()
    );
    service.remove(id, key).await.unwrap();
    assert!(
        crate::native_recovery_test_support::revoked(&db.pool, &handle)
            .await
            .unwrap()
    );
    assert!(service.list(id).await.unwrap().is_empty());
    assert!(matches!(service.remove(id, key).await, Err(Error::Refused)));
    let audit: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE actor=$1")
        .bind(id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert!(audit >= 3);
    db.close().await.unwrap();
}
#[tokio::test]
async fn postgres_challenge_binding_expiry_purpose_and_bad_input() {
    let Some(db) = database::Database::open().await.unwrap() else {
        return;
    };
    let service = service(db.pool.clone());
    let id = account(&db.pool).await;
    let start = service.begin_registration(BINDING, id).await.unwrap();
    assert!(
        service
            .consume::<PasskeyRegistration>(OTHER, &start.token, "register", Some(id))
            .await
            .is_err()
    );
    assert!(
        service
            .consume::<PasskeyRegistration>(BINDING, &start.token, "authenticate", Some(id))
            .await
            .is_err()
    );
    assert!(
        service
            .consume::<PasskeyRegistration>(BINDING, "bad", "register", Some(id))
            .await
            .is_err()
    );
    assert!(
        service
            .consume::<PasskeyRegistration>("bad", &start.token, "register", Some(id))
            .await
            .is_err()
    );
    sqlx::query("UPDATE passkey_ceremonies SET expires_at=now()-interval '1 second'")
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(
        service
            .consume::<PasskeyRegistration>(BINDING, &start.token, "register", Some(id))
            .await
            .is_err()
    );
    let start = service.begin_registration(BINDING, id).await.unwrap();
    assert!(
        service
            .consume::<PasskeyRegistration>(BINDING, &start.token, "register", Some(Uuid::now_v7()))
            .await
            .is_err()
    );
    assert!(
        service
            .consume::<PasskeyRegistration>(BINDING, &start.token, "register", Some(id))
            .await
            .is_err()
    );
    let start = service.begin_registration(BINDING, id).await.unwrap();
    assert!(
        service
            .finish_registration(
                BINDING,
                id,
                Completion {
                    token: start.token.clone(),
                    credential: serde_json::json!({})
                }
            )
            .await
            .is_err()
    );
    assert!(
        service
            .consume::<PasskeyRegistration>(BINDING, &start.token, "register", Some(id))
            .await
            .is_err()
    );
    let start = service.begin_authentication(BINDING).await.unwrap();
    assert!(
        service
            .finish_authentication(
                BINDING,
                Completion {
                    token: start.token.clone(),
                    credential: serde_json::json!({})
                }
            )
            .await
            .is_err()
    );
    assert!(
        service
            .consume::<DiscoverableAuthentication>(BINDING, &start.token, "authenticate", None)
            .await
            .is_err()
    );
    assert!(service.begin_authentication("bad").await.is_err());
    service.begin_authentication(BINDING).await.unwrap();
    service.begin_authentication(OTHER).await.unwrap();
    sqlx::query(
        "UPDATE passkey_ceremonies SET expires_at=now()-interval '1 second' WHERE session_hash=$1",
    )
    .bind(hash(BINDING))
    .execute(&db.pool)
    .await
    .unwrap();
    oteryn_db::outbox::Cleanup::clean(&PasskeyCleanup {
        pool: db.pool.clone(),
    })
    .await
    .unwrap();
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM passkey_ceremonies")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(remaining, 1);
    db.close().await.unwrap();
}
#[tokio::test]
async fn postgres_account_refusals_and_registration_limit() {
    let Some(db) = database::Database::open().await.unwrap() else {
        return;
    };
    let service = service(db.pool.clone());
    assert!(
        service
            .begin_registration(BINDING, Uuid::now_v7())
            .await
            .is_err()
    );
    let id = account(&db.pool).await;
    for statement in [
        "UPDATE accounts SET status='suspended' WHERE id=$1",
        "UPDATE accounts SET status='closed' WHERE id=$1",
        "UPDATE accounts SET status='pending_verification' WHERE id=$1",
        "UPDATE accounts SET status='active',email_verified_at=NULL WHERE id=$1",
        "UPDATE accounts SET email_verified_at=now(),login_mfa_required=true WHERE id=$1",
    ] {
        sqlx::query(sqlx::AssertSqlSafe(statement))
            .bind(id)
            .execute(&db.pool)
            .await
            .unwrap();
        assert!(service.begin_registration(BINDING, id).await.is_err());
        assert!(service.list(id).await.is_err());
        assert!(service.remove(id, Uuid::now_v7()).await.is_err());
    }
    sqlx::query("UPDATE accounts SET login_mfa_required=false WHERE id=$1")
        .bind(id)
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO account_passkeys(account_id,credential_id,credential) SELECT $1,decode(lpad(to_hex(n),2,'0'),'hex'),'{}' FROM generate_series(1,10) n").bind(id).execute(&db.pool).await.unwrap();
    assert!(service.begin_registration(BINDING, id).await.is_err());
    sqlx::query("DELETE FROM account_passkeys WHERE account_id=$1")
        .bind(id)
        .execute(&db.pool)
        .await
        .unwrap();
    let mut auth = SoftPasskey::new(true);
    let input = registration_input(&service, id, &mut auth).await;
    sqlx::query("INSERT INTO account_passkeys(account_id,credential_id,credential) SELECT $1,decode(lpad(to_hex(n),2,'0'),'hex'),'{}' FROM generate_series(1,10) n").bind(id).execute(&db.pool).await.unwrap();
    assert!(matches!(
        service.finish_registration(BINDING, id, input).await,
        Err(Error::Refused)
    ));
    sqlx::query("DELETE FROM account_passkeys WHERE account_id=$1")
        .bind(id)
        .execute(&db.pool)
        .await
        .unwrap();
    let input = registration_input(&service, id, &mut auth).await;
    let proof: RegisterPublicKeyCredential = decode(input.credential.clone()).unwrap();
    sqlx::query(
        "INSERT INTO account_passkeys(account_id,credential_id,credential) VALUES ($1,$2,'{}')",
    )
    .bind(id)
    .bind(proof.raw_id)
    .execute(&db.pool)
    .await
    .unwrap();
    assert!(matches!(
        service.finish_registration(BINDING, id, input).await,
        Err(Error::Refused)
    ));
    assert!(matches!(
        service.begin_registration(BINDING, id).await,
        Err(Error::Refused)
    ));
    sqlx::query("DELETE FROM account_passkeys WHERE account_id=$1")
        .bind(id)
        .execute(&db.pool)
        .await
        .unwrap();
    let input = registration_input(&service, id, &mut auth).await;
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(id)
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(matches!(
        service.finish_registration(BINDING, id, input).await,
        Err(Error::Refused)
    ));
    db.close().await.unwrap();
}
#[tokio::test]
async fn postgres_concurrent_consumption_and_replacement() {
    let Some(db) = database::Database::open().await.unwrap() else {
        return;
    };
    let service = service(db.pool.clone());
    let first = service.begin_authentication(BINDING).await.unwrap();
    let second = service.begin_authentication(BINDING).await.unwrap();
    assert!(
        service
            .consume::<DiscoverableAuthentication>(BINDING, &first.token, "authenticate", None)
            .await
            .is_err()
    );
    let (a, b) = tokio::join!(
        service.consume::<DiscoverableAuthentication>(BINDING, &second.token, "authenticate", None),
        service.consume::<DiscoverableAuthentication>(BINDING, &second.token, "authenticate", None)
    );
    assert_ne!(a.is_ok(), b.is_ok());
    db.close().await.unwrap();
}
#[tokio::test]
async fn postgres_bad_signature_origin_and_unverified_authenticator() {
    let Some(db) = database::Database::open().await.unwrap() else {
        return;
    };
    let service = service(db.pool.clone());
    let id = account(&db.pool).await;
    let mut auth = SoftPasskey::new(true);
    registered(&service, id, &mut auth).await;
    let other_origin =
        assertion_at_origin(&service, id, &mut auth, "https://example.test:444").await;
    assert!(matches!(
        service.finish_authentication(BINDING, other_origin).await,
        Err(Error::Refused)
    ));
    for field in [
        "signature",
        "authenticatorData",
        "clientDataJSON",
        "userHandle",
    ] {
        let mut input = assertion(&service, id, &mut auth).await;
        input.credential["response"][field] = "AAAA".into();
        assert!(service.finish_authentication(BINDING, input).await.is_err());
    }
    let input = assertion(&service, id, &mut auth).await;
    assert_eq!(
        service.finish_authentication(BINDING, input).await.unwrap(),
        id
    );
    let start = service.begin_registration(BINDING, id).await.unwrap();
    let mut options = start.options;
    options["publicKey"]["authenticatorSelection"]["residentKey"] = "discouraged".into();
    options["publicKey"]["authenticatorSelection"]["requireResidentKey"] = false.into();
    options["publicKey"]["authenticatorSelection"]["userVerification"] = "preferred".into();
    let response = SoftPasskey::new(false)
        .do_registration(
            Url::parse("https://example.test").unwrap(),
            decode(options).unwrap(),
        )
        .unwrap();
    assert!(
        service
            .finish_registration(
                BINDING,
                id,
                Completion {
                    token: start.token,
                    credential: encode(&response).unwrap()
                }
            )
            .await
            .is_err()
    );
    db.close().await.unwrap();
}
#[tokio::test]
async fn postgres_account_rechecked_and_removed_credential_denied() {
    let Some(db) = database::Database::open().await.unwrap() else {
        return;
    };
    let service = service(db.pool.clone());
    let id = account(&db.pool).await;
    let mut auth = SoftPasskey::new(true);
    registered(&service, id, &mut auth).await;
    for statement in [
        "UPDATE accounts SET status='suspended' WHERE id=$1",
        "UPDATE accounts SET status='active',email_verified_at=NULL WHERE id=$1",
        "UPDATE accounts SET email_verified_at=now(),login_mfa_required=true WHERE id=$1",
    ] {
        let input = assertion(&service, id, &mut auth).await;
        sqlx::query(sqlx::AssertSqlSafe(statement))
            .bind(id)
            .execute(&db.pool)
            .await
            .unwrap();
        assert!(service.finish_authentication(BINDING, input).await.is_err());
    }
    sqlx::query("UPDATE accounts SET login_mfa_required=false WHERE id=$1")
        .bind(id)
        .execute(&db.pool)
        .await
        .unwrap();
    let input = assertion(&service, id, &mut auth).await;
    sqlx::query("DELETE FROM account_passkeys WHERE account_id=$1")
        .bind(id)
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(service.finish_authentication(BINDING, input).await.is_err());
    db.close().await.unwrap();
}

#[tokio::test]
async fn audit02_completed_passkey_proof_refuses_removed_key()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = database::Database::open()
        .await
        .map_err(|_| "passkey regression")?
    else {
        return Ok(());
    };
    let service = service(db.pool.clone());
    let id = account(&db.pool).await;
    let mut auth = SoftPasskey::new(true);
    registered(&service, id, &mut auth).await;
    let input = assertion(&service, id, &mut auth).await;
    let proof = service
        .finish_authorized(BINDING, input)
        .await
        .map_err(|_| "passkey regression")?;
    assert!(
        service
            .authentication_current(&proof)
            .await
            .map_err(|_| "passkey regression")?
    );
    let key = service.list(id).await.map_err(|_| "passkey regression")?[0];
    service
        .remove(id, key)
        .await
        .map_err(|_| "passkey regression")?;
    assert!(
        !service
            .authentication_current(&proof)
            .await
            .map_err(|_| "passkey regression")?
    );
    db.close().await
}

#[tokio::test]
async fn audit02_removing_another_passkey_invalidates_earlier_authentication()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = database::Database::open().await? else {
        return Ok(());
    };
    let service = service(db.pool.clone());
    let id = account(&db.pool).await;
    let mut first = SoftPasskey::new(true);
    registered(&service, id, &mut first).await;
    let mut second = SoftPasskey::new(true);
    registered(&service, id, &mut second).await;
    let input = assertion(&service, id, &mut first).await;
    let proof = service
        .finish_authorized(BINDING, input)
        .await
        .map_err(|_| "proof")?;
    assert!(
        service
            .authentication_current(&proof)
            .await
            .map_err(|_| "proof")?
    );
    let keys = service.list(id).await.map_err(|_| "keys")?;
    service.remove(id, keys[1]).await.map_err(|_| "remove")?;
    assert_eq!(service.list(id).await.map_err(|_| "keys")?.len(), 1);
    assert!(
        !service
            .authentication_current(&proof)
            .await
            .map_err(|_| "proof")?
    );
    db.close().await
}

#[tokio::test]
async fn audit02_registration_and_removal_refuse_revoked_authorization() {
    let started = std::time::Instant::now();
    let Some(db) = database::Database::open().await.unwrap() else {
        return;
    };
    let service = service(db.pool.clone());
    let id = account(&db.pool).await;
    sqlx::query("UPDATE accounts SET password_hash=$2 WHERE id=$1")
        .bind(id)
        .bind(crate::password::hash("compromised owner password").unwrap())
        .execute(&db.pool)
        .await
        .unwrap();
    let _ = service.fixture_proof(BINDING, id).await;
    let proof = service
        .confirm_password(id, "compromised owner password".into())
        .await
        .unwrap();
    let start = service
        .begin_registration_authorized(BINDING, proof)
        .await
        .unwrap();
    let input = registration_completion(start, &mut SoftPasskey::new(true));
    let mut recovery = db.pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(id)
        .execute(&mut *recovery)
        .await
        .unwrap();
    let recovery_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *recovery)
        .await
        .unwrap();
    let task = tokio::spawn({
        let service = service.clone();
        async move { service.finish_registration(BINDING, id, input).await }
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let count: i64 =
                sqlx::query_scalar("SELECT count(*) FROM passkey_ceremonies WHERE account_id=$1")
                    .bind(id)
                    .fetch_one(&db.pool)
                    .await
                    .unwrap();
            if count == 0 {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let blocked: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)))",
            )
            .bind(recovery_pid)
            .fetch_one(&db.pool)
            .await
            .unwrap();
            if blocked {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    crate::recovery_key::reset_base(&mut recovery, id)
        .await
        .unwrap();
    crate::recovery_key::integrated_security(&mut recovery, id)
        .await
        .unwrap();
    sqlx::query("UPDATE accounts SET password_hash=$2 WHERE id=$1")
        .bind(id)
        .bind(crate::password::hash("recovered owner password").unwrap())
        .execute(&mut *recovery)
        .await
        .unwrap();
    recovery.commit().await.unwrap();
    assert!(matches!(task.await.unwrap(), Err(Error::Refused)));
    assert!(service.list(id).await.unwrap().is_empty());
    let _ = service.fixture_proof(BINDING, id).await;
    let proof = service
        .confirm_password(id, "recovered owner password".into())
        .await
        .unwrap();
    sqlx::query(
        "UPDATE accounts SET native_security_generation=native_security_generation+1 WHERE id=$1",
    )
    .bind(id)
    .execute(&db.pool)
    .await
    .unwrap();
    assert!(matches!(
        service.begin_registration_authorized(BINDING, proof).await,
        Err(Error::Refused)
    ));
    registered(&service, id, &mut SoftPasskey::new(true)).await;
    let key = service.list(id).await.unwrap()[0];
    let _ = service.fixture_proof(BINDING, id).await;
    let proof = service
        .confirm_password(id, "recovered owner password".into())
        .await
        .unwrap();
    sqlx::query(
        "UPDATE accounts SET native_security_generation=native_security_generation+1 WHERE id=$1",
    )
    .bind(id)
    .execute(&db.pool)
    .await
    .unwrap();
    assert!(matches!(
        service.remove_authorized(BINDING, proof, key).await,
        Err(Error::Refused)
    ));
    assert_eq!(service.list(id).await.unwrap(), vec![key]);
    let proof = service
        .confirm_password(id, "recovered owner password".into())
        .await
        .unwrap();
    sqlx::query("DELETE FROM web_sessions WHERE account_id=$1")
        .bind(id)
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(matches!(
        service.remove_authorized(BINDING, proof, key).await,
        Err(Error::Refused)
    ));
    assert_eq!(service.list(id).await.unwrap(), vec![key]);

    eprintln!(
        "audit02_registration_and_removal_refuse_revoked_authorization duration: {:?}",
        started.elapsed()
    );
    db.close().await.unwrap();
}

/// Holds the account row, lets the ceremony expire while `run` waits for it, then releases it.
async fn ceremony_expires_during_account_lock_wait<T: Send + 'static>(
    pool: &PgPool,
    id: Uuid,
    purpose: &str,
    run: impl FnOnce() -> tokio::task::JoinHandle<Result<T, Error>>,
) -> Result<T, Error> {
    let mut holder = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(id)
        .execute(&mut *holder)
        .await
        .unwrap();
    let holder_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *holder)
        .await
        .unwrap();
    // Fixture: a ceremony that is still valid when consumed but expires during the lock wait.
    let expires_at: sqlx::types::time::OffsetDateTime = sqlx::query_scalar(
        "UPDATE passkey_ceremonies SET expires_at=clock_timestamp()+interval '2 seconds' WHERE purpose=$1 RETURNING expires_at",
    )
    .bind(purpose)
    .fetch_one(pool)
    .await
    .unwrap();
    let task = run();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let blocked: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)))",
            )
            .bind(holder_pid)
            .fetch_one(pool)
            .await
            .unwrap();
            if blocked {
                break;
            }
            assert!(
                !task.is_finished(),
                "{purpose} returned before the lock wait"
            );
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let expired: bool = sqlx::query_scalar("SELECT clock_timestamp()>$1")
                .bind(expires_at)
                .fetch_one(pool)
                .await
                .unwrap();
            if expired {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    holder.commit().await.unwrap();
    task.await.unwrap()
}

#[tokio::test]
async fn d3_authentication_ceremony_expires_while_waiting_for_account_lock() {
    let Some(db) = database::Database::open().await.unwrap() else {
        return;
    };
    let service = service(db.pool.clone());
    let id = account(&db.pool).await;
    let mut auth = SoftPasskey::new(true);
    registered(&service, id, &mut auth).await;
    let input = assertion(&service, id, &mut auth).await;
    let result = ceremony_expires_during_account_lock_wait(&db.pool, id, "authenticate", || {
        let service = service.clone();
        tokio::spawn(async move { service.finish_authorized(BINDING, input).await })
    })
    .await;
    assert!(
        matches!(result, Err(Error::Refused)),
        "expired ceremony accepted after the account lock wait"
    );
    db.close().await.unwrap();
}

#[tokio::test]
async fn d3_registration_ceremony_expires_while_waiting_for_account_lock() {
    let Some(db) = database::Database::open().await.unwrap() else {
        return;
    };
    let service = service(db.pool.clone());
    let id = account(&db.pool).await;
    let mut auth = SoftPasskey::new(true);
    let input = registration_input(&service, id, &mut auth).await;
    let result = ceremony_expires_during_account_lock_wait(&db.pool, id, "register", || {
        let service = service.clone();
        tokio::spawn(async move { service.finish_registration(BINDING, id, input).await })
    })
    .await;
    assert!(
        matches!(result, Err(Error::Refused)),
        "expired registration ceremony accepted after the account lock wait"
    );
    db.close().await.unwrap();
}
