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
    let start = service.begin_authentication(BINDING).await.unwrap();
    let key: Value =
        sqlx::query_scalar("SELECT credential FROM account_passkeys WHERE account_id=$1")
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
        .do_authentication(
            Url::parse("https://example.test").unwrap(),
            decode(options).unwrap(),
        )
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
    registered(&service, id, &mut auth).await;
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
    assert!(matches!(
        service.remove(Uuid::now_v7(), key).await,
        Err(Error::Refused)
    ));
    service.remove(id, key).await.unwrap();
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
