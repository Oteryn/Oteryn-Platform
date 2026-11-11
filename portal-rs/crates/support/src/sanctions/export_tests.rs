use super::*;
use axum::body::to_bytes;
use oteryn_internal_api::guard::VerifiedClient;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const REQUEST: &[u8] =
    include_bytes!("../../../../../docs/contracts/fixtures/sanction-export-v1/request.json");
const RESPONSE: &[u8] =
    include_bytes!("../../../../../docs/contracts/fixtures/sanction-export-v1/page.json");
const EMPTY: &[u8] =
    include_bytes!("../../../../../docs/contracts/fixtures/sanction-export-v1/empty.json");
fn map() -> TestResult<IdentityMap> {
    Ok(identities(vec![(
        "PORTAL_SANCTION_EXPORT_IDENTITY".into(),
        "sanctions.game.example.invalid".into(),
    )])?)
}
fn request(body: impl Into<Body>, identity: Option<&str>) -> TestResult<Request> {
    let mut req = Request::builder()
        .method("POST")
        .uri(PATH)
        .header(header::CONTENT_TYPE, "application/json")
        .body(body.into())?;
    if let Some(san) = identity {
        req.extensions_mut()
            .insert(VerifiedClient::from_verified_tls13_http11(Identity::san(
                san,
            )?));
    }
    Ok(req)
}
fn lazy_pool() -> TestResult<PgPool> {
    Ok(PgPoolOptions::new().connect_lazy("postgres://localhost/unused")?)
}
fn app(pool: PgPool, enabled: bool) -> TestResult<Router> {
    Ok(router(pool, Settings { enabled }, Arc::new(map()?)))
}
#[tokio::test]
async fn sanction_export_disabled_503() -> TestResult {
    assert!(!Settings::from_lookup(|_| None).enabled);
    let response = app(lazy_pool()?, false)?
        .oneshot(request(REQUEST, None)?)
        .await?;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(to_bytes(response.into_body(), 256).await?.is_empty());
    let response = router(
        lazy_pool()?,
        Settings { enabled: true },
        Arc::new(IdentityMap::default()),
    )
    .oneshot(request(REQUEST, None)?)
    .await?;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    for vars in [
        vec![],
        vec![("PORTAL_SANCTION_EXPORT_IDENTITY".into(), "bad name".into())],
        vec![
            (
                "PORTAL_SANCTION_EXPORT_IDENTITY".into(),
                "peer.example".into(),
            ),
            ("OTHER_MTLS_CLIENT_IDENTITY".into(), "peer.example".into()),
        ],
        vec![
            (
                "PORTAL_SANCTION_EXPORT_IDENTITY".into(),
                "peer.example".into(),
            ),
            ("OTHER_IDENTITIES".into(), r#"{"peer.example":[]}"#.into()),
        ],
    ] {
        assert!(identities(vars).is_err());
    }
    Ok(())
}
#[tokio::test]
async fn sanction_export_wrong_identity_refused() -> TestResult {
    for identity in [None, Some("other.game.example.invalid")] {
        let mut req = request(REQUEST, identity)?;
        req.headers_mut().insert(
            "x-client-san",
            header::HeaderValue::from_static("sanctions.game.example.invalid"),
        );
        let response = app(lazy_pool()?, true)?.oneshot(req).await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(to_bytes(response.into_body(), 256).await?.is_empty());
    }
    Ok(())
}
#[tokio::test]
async fn sanction_export_bad_body_400() -> TestResult {
    let app = app(lazy_pool()?, true)?;
    for body in [
        "{}",
        r#"{"schema":"other","after":null,"limit":1}"#,
        r#"{"schema":"oteryn.sanction_stream_request.v1","after":null,"limit":0}"#,
        r#"{"schema":"oteryn.sanction_stream_request.v1","after":null,"limit":501}"#,
        r#"{"schema":"oteryn.sanction_stream_request.v1","after":"v1:01","limit":1}"#,
        r#"{"schema":"oteryn.sanction_stream_request.v1","after":"v1:9223372036854775808","limit":1}"#,
        r#"{"schema":"oteryn.sanction_stream_request.v1","after":null,"limit":1.0}"#,
        r#"{"schema":"oteryn.sanction_stream_request.v1","after":null,"limit":1,"limit":2}"#,
        r#"{"schema":"oteryn.sanction_stream_request.v1","after":null,"limit":1,"extra":true}"#,
        &" ".repeat(257),
    ] {
        let response = app
            .clone()
            .oneshot(request(
                body.to_owned(),
                Some("sanctions.game.example.invalid"),
            )?)
            .await?;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{body}");
    }
    Ok(())
}
#[tokio::test]
async fn sanction_export_rate_limit_retry_after() -> TestResult {
    let app = app(lazy_pool()?, true)?;
    for _ in 0..1200 {
        assert_eq!(
            app.clone()
                .oneshot(request("{}", Some("sanctions.game.example.invalid"))?)
                .await?
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    let response = app
        .oneshot(request("{}", Some("sanctions.game.example.invalid"))?)
        .await?;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(response.headers()[header::RETRY_AFTER], "60");
    Ok(())
}
struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
    actor: Uuid,
    owner: Uuid,
}
impl Database {
    async fn new() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIP sanction export PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await?;
        let schema = format!("sanction_export_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(5)
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
            .await?;
        for migration in [
            include_str!("../../../../migrations/20261105155256_t03_accounts.sql"),
            include_str!("../../../../migrations/20261105170000_t10_rbac_audit.sql"),
            include_str!("../../../../migrations/20261105190100_t19_native_admissions.sql"),
            include_str!("../../../../migrations/20261105220235_t34_sanctions.sql"),
            include_str!("../../../../migrations/20261105220236_t34b_appeals.sql"),
            include_str!("../../../../migrations/20261105231600_support_sanction_revisions.sql"),
            include_str!("../../../../migrations/20261111140002_n009_sanction_export.sql"),
        ] {
            sqlx::raw_sql(migration).execute(&pool).await?;
        }
        let actor = Uuid::now_v7();
        let owner = Uuid::parse_str("019a0000-0000-7000-8000-000000000002")?;
        for id in [actor, owner] {
            sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale) VALUES ($1,$2,'synthetic-hash','active','en')").bind(id).bind(format!("{id}@example.invalid")).execute(&pool).await?;
        }
        sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='moderator'").bind(actor).execute(&pool).await?;
        Ok(Some(Self {
            pool,
            admin,
            schema,
            actor,
            owner,
        }))
    }
    async fn insert(
        &self,
        conn: &mut sqlx::PgConnection,
        id: Uuid,
        category: &str,
        reason: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO sanctions(id,account_id,category,status,public_reason,effective_at,expires_at,created_by) VALUES ($1,$2,$3,'active',$4,to_timestamp(1700000000),to_timestamp(1700000060),$5)")
            .bind(id).bind(self.owner).bind(category).bind(reason).bind(self.actor).execute(conn).await?;
        Ok(())
    }
    async fn add(&self, category: &str, reason: &str) -> Result<Uuid, sqlx::Error> {
        let id = Uuid::now_v7();
        let mut conn = self.pool.acquire().await?;
        self.insert(&mut conn, id, category, reason).await?;
        Ok(id)
    }
    async fn cleanup(self) -> TestResult {
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
async fn page(db: &Database, after: Option<i64>, limit: i64) -> TestResult<serde_json::Value> {
    Ok(serde_json::from_slice(
        &read_page(&db.pool, Page { after, limit }).await?,
    )?)
}
#[tokio::test]
async fn sanction_export_paging_cursor_stable() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let first = db.add("warning", "Public warning").await?;
    let mut pending = db.pool.begin().await?;
    db.insert(
        &mut pending,
        Uuid::now_v7(),
        "suspension",
        "Public suspension",
    )
    .await?;
    let visible = page(&db, None, 500).await?;
    assert_eq!(
        visible["items"]
            .as_array()
            .ok_or("items must be an array")?
            .len(),
        1
    );
    assert_eq!(visible["items"][0]["sanction_stream_id"], first.to_string());
    let pool = db.pool.clone();
    let owner = db.owner;
    let actor = db.actor;
    let mut later = tokio::spawn(async move {
        sqlx::query("INSERT INTO sanctions(id,account_id,category,status,public_reason,effective_at,expires_at,created_by) VALUES ($1,$2,'warning','active','Later public warning',to_timestamp(1700000000),to_timestamp(1700000060),$3)").bind(Uuid::now_v7()).bind(owner).bind(actor).execute(&pool).await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut later)
            .await
            .is_err()
    );
    pending.commit().await?;
    later.await??;
    let second = page(&db, Some(1), 1).await?;
    let third = page(&db, Some(2), 1).await?;
    assert_eq!(second["next_cursor"], "v1:2");
    assert_eq!(third["next_cursor"], "v1:3");
    assert_eq!(page(&db, Some(1), 1).await?, second);
    assert_eq!(
        page(&db, Some(3), 500).await?["next_cursor"],
        serde_json::Value::Null
    );
    let mut rollback = db.pool.begin().await?;
    db.insert(
        &mut rollback,
        Uuid::now_v7(),
        "warning",
        "Rolled back warning",
    )
    .await?;
    rollback.rollback().await?;
    db.add("warning", "Committed warning").await?;
    assert_eq!(page(&db, Some(3), 500).await?["next_cursor"], "v1:4");
    db.cleanup().await
}
#[tokio::test]
async fn sanction_export_revision_on_revoke_and_appeal() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let id = db.add("suspension", "Public suspension").await?;
    let service = super::super::Service::new(
        db.pool.clone(),
        super::super::Features {
            sanctions: true,
            game_access: false,
        },
    );
    service
        .revoke(
            oteryn_admin::rbac::CheckContext {
                account: db.actor,
                require_staff_mfa: true,
                mfa_verified: true,
            },
            Uuid::now_v7(),
            id,
            1,
        )
        .await?;
    let appeals = crate::appeals::Appeals { enabled: true };
    let mut tx = db.pool.begin().await?;
    appeals.register(&mut tx, id, db.owner).await?;
    sqlx::query("UPDATE support_appeal_workflows SET status='requested',message='Player appeal' WHERE sanction_id=$1").bind(id).execute(&mut *tx).await?;
    sqlx::query("UPDATE support_appeal_workflows SET status='rejected',outcome='Private outcome detail' WHERE sanction_id=$1").bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    let result = page(&db, None, 500).await?;
    let items = result["items"].as_array().ok_or("items must be an array")?;
    assert_eq!(items.len(), 3);
    for (i, item) in items.iter().enumerate() {
        assert_eq!(item["decision_revision"], i + 1);
        assert_eq!(item["revoked"], i > 0);
        assert_eq!(item["reason_text"], "Public suspension");
    }
    let accepted = db.add("warning", "Appealed public warning").await?;
    let mut tx = db.pool.begin().await?;
    appeals.register(&mut tx, accepted, db.owner).await?;
    sqlx::query("UPDATE support_appeal_workflows SET status='accepted',message='Player appeal',outcome='Accepted outcome' WHERE sanction_id=$1").bind(accepted).execute(&mut *tx).await?;
    tx.commit().await?;
    assert_eq!(page(&db, Some(4), 500).await?["items"][0]["revoked"], true);
    db.add("restriction", "Private platform restriction")
        .await?;
    assert_eq!(
        page(&db, None, 500).await?["items"]
            .as_array()
            .ok_or("items must be an array")?
            .len(),
        5
    );
    db.cleanup().await
}
#[tokio::test]
async fn sanction_export_reason_text_limits() -> TestResult {
    assert!(valid_reason(&"ą".repeat(60)));
    for reason in [
        "".to_owned(),
        "ą".repeat(61),
        "x\ny".into(),
        "x\u{0085}y".into(),
    ] {
        assert!(!valid_reason(&reason));
    }
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    db.add("warning", &format!("\n{}\tSuffix", "ą".repeat(100)))
        .await?;
    assert_eq!(
        page(&db, None, 500).await?["items"][0]["reason_text"],
        "ą".repeat(60)
    );
    assert!(db.add("warning", "\n\t").await.is_err());
    db.add("warning", "x\u{0085}y").await?;
    assert_eq!(
        page(&db, Some(1), 500).await?["items"][0]["reason_text"],
        "x y"
    );
    let mut tx = db.pool.begin().await?;
    assert!(matches!(
        set_public_reason(&mut tx, Uuid::now_v7(), 0, "Public reason").await,
        Err(Error::Invalid)
    ));
    assert!(matches!(
        set_public_reason(&mut tx, Uuid::now_v7(), 1, &"a".repeat(121)).await,
        Err(Error::Invalid)
    ));
    tx.rollback().await?;
    db.cleanup().await
}
#[tokio::test]
async fn sanction_export_fixtures_match() -> TestResult {
    assert_eq!(
        Page::decode(&canonical::decode(REQUEST, 256, 4)?)?,
        Page {
            after: None,
            limit: 1
        }
    );
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    assert_eq!(
        read_page(
            &db.pool,
            Page {
                after: None,
                limit: 1
            }
        )
        .await?,
        EMPTY
    );
    let id = Uuid::parse_str("019a0000-0000-7000-8000-000000000001")?;
    let mut conn = db.pool.acquire().await?;
    db.insert(&mut conn, id, "suspension", "Public rules violation")
        .await?;
    drop(conn);
    let response = app(db.pool.clone(), true)?
        .oneshot(request(REQUEST, Some("sanctions.game.example.invalid"))?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "no-store, private"
    );
    assert_eq!(
        to_bytes(response.into_body(), MAX_RESPONSE).await?.as_ref(),
        RESPONSE
    );
    db.cleanup().await
}
#[tokio::test]
async fn sanction_export_typed_revisions_and_private_notation() -> TestResult {
    use super::super::{Features, SanctionAction, SanctionIntent, Service};
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let service = Service::new(
        db.pool.clone(),
        Features {
            sanctions: true,
            game_access: false,
        },
    );
    let mut tx = db.pool.begin().await?;
    let mut intent = SanctionIntent {
        account: db.owner,
        sanction_id: Uuid::now_v7(),
        character: None,
        action: SanctionAction::Ban {
            expires_at_ms: None,
        },
    };
    service.allocate_and_record(&mut tx, &intent).await?;
    set_public_reason(&mut tx, intent.sanction_id, 42, "Public ban reason").await?;
    intent.action = SanctionAction::Unban;
    service.allocate_and_record(&mut tx, &intent).await?;
    tx.commit().await?;
    let items = page(&db, None, 500).await?;
    assert_eq!(items["items"][0]["kind"], 4);
    assert_eq!(items["items"][1]["revoked"], true);
    assert_eq!(items["items"][1]["decision_revision"], 2);
    let mut tx = db.pool.begin().await?;
    intent.sanction_id = Uuid::now_v7();
    intent.action = SanctionAction::Notation;
    service.allocate_and_record(&mut tx, &intent).await?;
    set_public_reason(&mut tx, intent.sanction_id, 42, "Staff-only notation").await?;
    tx.commit().await?;
    assert_eq!(
        page(&db, None, 500).await?["items"]
            .as_array()
            .ok_or("items must be an array")?
            .len(),
        2
    );
    db.cleanup().await
}

#[tokio::test]
async fn sanction_export_namelock_and_immutable_journal() -> TestResult {
    use super::super::{Features, SanctionAction, SanctionIntent, Service};
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let service = Service::new(
        db.pool.clone(),
        Features {
            sanctions: true,
            game_access: false,
        },
    );
    let mut tx = db.pool.begin().await?;
    let mut intent = SanctionIntent {
        account: db.owner,
        sanction_id: Uuid::now_v7(),
        character: Some(Uuid::now_v7()),
        action: SanctionAction::Namelock {
            expires_at_ms: None,
        },
    };
    service.allocate_and_record(&mut tx, &intent).await?;
    set_public_reason(&mut tx, intent.sanction_id, 65535, "Public name rule").await?;
    intent.action = SanctionAction::LiftNamelock;
    service.allocate_and_record(&mut tx, &intent).await?;
    tx.commit().await?;
    let result = page(&db, None, 500).await?;
    assert_eq!(result["items"][0]["kind"], 5);
    assert_eq!(
        result["items"][0]["character_id"],
        intent.character.ok_or("character is required")?.to_string()
    );
    assert_eq!(result["items"][0]["ends_at_ms"], serde_json::Value::Null);
    assert_eq!(result["items"][1]["revoked"], true);
    assert!(
        sqlx::query("UPDATE support_sanction_export_revisions SET revoked=false")
            .execute(&db.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM support_sanction_export_revisions")
            .execute(&db.pool)
            .await
            .is_err()
    );
    db.cleanup().await
}

#[test]
fn sanction_export_full_page_byte_bound() -> TestResult {
    let raw = serde_json::from_slice::<serde_json::Value>(RESPONSE)?;
    let mut item = raw["items"][0].clone();
    item["decision_revision"] = i64::MAX.into();
    item["character_id"] = "019a0000-0000-7000-8000-000000000003".into();
    item["kind"] = 3.into();
    item["reason_code"] = 65535.into();
    item["starts_at_ms"] = (i64::MAX - 1).into();
    item["ends_at_ms"] = i64::MAX.into();
    for (reason, expected_count) in [("a".repeat(120), Some(500)), ("\\".repeat(120), None)] {
        item["reason_text"] = reason.into();
        let value = canonical::decode(&serde_json::to_vec(&item)?, 1024, 4)?;
        let bytes = encode_page((1..=500).map(|n| (n, value.clone())).collect())?;
        assert!(bytes.len() <= 256 * 1024);
        let page: serde_json::Value = serde_json::from_slice(&bytes)?;
        let count = page["items"]
            .as_array()
            .ok_or("items must be an array")?
            .len();
        if let Some(expected) = expected_count {
            assert_eq!(count, expected);
        } else {
            assert!(count < 500);
        }
        assert_eq!(page["next_cursor"], format!("v1:{count}"));
        let bytes = encode_page((1..=200).map(|n| (n, value.clone())).collect())?;
        let page: serde_json::Value = serde_json::from_slice(&bytes)?;
        assert_eq!(
            page["items"]
                .as_array()
                .ok_or("items must be an array")?
                .len(),
            200
        );
    }
    Ok(())
}
#[tokio::test]
async fn sanction_export_reused_internal_identities_503() -> TestResult {
    for key in [
        "PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_MTLS_CLIENT_IDENTITY",
        "PRODUCTS_ENTITLEMENTS_LOYALTY_SNAPSHOT_MTLS_CLIENT_IDENTITY",
        "GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY",
        "GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_MTLS_CLIENT_IDENTITY",
        "PORTAL_GAME_COINS_IDENTITIES",
    ] {
        let san = "sanctions.game.example.invalid";
        let other = if key.ends_with("_IDENTITIES") {
            format!("{{\"{san}\":[]}}")
        } else {
            san.into()
        };
        let config = identities(vec![
            ("PORTAL_SANCTION_EXPORT_IDENTITY".into(), san.into()),
            (key.into(), other),
        ]);
        assert!(config.is_err());
        let response = router(
            lazy_pool()?,
            Settings {
                enabled: config.is_ok(),
            },
            Arc::new(config.unwrap_or_default()),
        )
        .oneshot(request(REQUEST, Some(san))?)
        .await?;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE, "{key}");
    }
    Ok(())
}

#[tokio::test]
async fn nwave_lifted_sanction_requires_new_identity() -> TestResult {
    use super::super::{Features, SanctionAction, SanctionIntent, Service};
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let service = Service::new(
        db.pool.clone(),
        Features {
            sanctions: true,
            game_access: false,
        },
    );
    for (active, lifted, character) in [
        (
            SanctionAction::Ban {
                expires_at_ms: None,
            },
            SanctionAction::Unban,
            None,
        ),
        (
            SanctionAction::Namelock {
                expires_at_ms: None,
            },
            SanctionAction::LiftNamelock,
            Some(Uuid::now_v7()),
        ),
    ] {
        let mut intent = SanctionIntent {
            account: db.owner,
            sanction_id: Uuid::now_v7(),
            character,
            action: active.clone(),
        };
        let mut tx = db.pool.begin().await?;
        service.allocate_and_record(&mut tx, &intent).await?;
        set_public_reason(&mut tx, intent.sanction_id, 42, "Public rule").await?;
        intent.action = lifted;
        let lift = service.allocate_and_record(&mut tx, &intent).await?;
        assert_eq!(
            service
                .record_at_revision(&mut tx, &intent, lift.revision)
                .await?,
            lift
        );
        tx.commit().await?;
        intent.action = active;
        let mut tx = db.pool.begin().await?;
        assert!(matches!(
            service.allocate_and_record(&mut tx, &intent).await,
            Err(super::super::Error::Conflict)
        ));
        tx.rollback().await?;
        intent.sanction_id = Uuid::now_v7();
        let mut tx = db.pool.begin().await?;
        service.allocate_and_record(&mut tx, &intent).await?;
        set_public_reason(&mut tx, intent.sanction_id, 42, "New public rule").await?;
        tx.commit().await?;
    }
    let result = page(&db, None, 500).await?;
    let items = result["items"].as_array().ok_or("items required")?;
    assert_eq!(items.len(), 6);
    for group in items.chunks(3) {
        assert_eq!(
            group[0]["sanction_stream_id"],
            group[1]["sanction_stream_id"]
        );
        assert_eq!(group[1]["revoked"], true);
        assert_ne!(
            group[1]["sanction_stream_id"],
            group[2]["sanction_stream_id"]
        );
        assert_eq!(group[2]["revoked"], false);
        assert_eq!(group[2]["decision_revision"], 1);
    }
    db.cleanup().await
}

#[tokio::test]
async fn sanction_export_temporary_namelock_revokes_on_expiry() -> TestResult {
    use super::super::{Features, SanctionAction, SanctionIntent, Service};
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let service = Service::new(
        db.pool.clone(),
        Features {
            sanctions: true,
            game_access: false,
        },
    );
    let mut tx = db.pool.begin().await?;
    let ends: i64 = sqlx::query_scalar(
        "SELECT floor(extract(epoch FROM clock_timestamp()+interval '1 hour')*1000)::bigint",
    )
    .fetch_one(&mut *tx)
    .await?;
    let intent = SanctionIntent {
        account: db.owner,
        sanction_id: Uuid::parse_str("019a0000-0000-7000-8000-000000000003")?,
        character: Some(Uuid::parse_str("019a0000-0000-7000-8000-000000000004")?),
        action: SanctionAction::Namelock {
            expires_at_ms: Some(ends),
        },
    };
    service.allocate_and_record(&mut tx, &intent).await?;
    sqlx::query("UPDATE support_sanction_revisions SET decision_at=to_timestamp(1700000000) WHERE sanction_id=$1")
        .bind(intent.sanction_id).execute(&mut *tx).await?;
    set_public_reason(&mut tx, intent.sanction_id, 42, "Temporary name rule").await?;
    tx.commit().await?;
    let initial = page(&db, None, 500).await?;
    assert_eq!(initial["items"][0]["revoked"], false);
    assert_eq!(initial["items"][0]["ends_at_ms"], serde_json::Value::Null);
    // Move the stored deadline into the past without sleeping. The normal
    // decision writer rejects already-expired input; this models elapsed time.
    sqlx::query("UPDATE support_sanction_revisions SET expires_at=to_timestamp(1700000060) WHERE sanction_id=$1")
        .bind(intent.sanction_id).execute(&db.pool).await?;
    let after: i64 = initial["next_cursor"]
        .as_str()
        .ok_or("cursor required")?
        .strip_prefix("v1:")
        .ok_or("cursor version")?
        .parse()?;
    let mut publisher = db.pool.begin().await?;
    sqlx::query("SELECT position FROM support_sanction_export_counter WHERE singleton FOR UPDATE")
        .execute(&mut *publisher)
        .await?;
    let while_publishing =
        tokio::time::timeout(Duration::from_secs(1), page(&db, Some(after), 500)).await??;
    assert!(
        while_publishing["items"]
            .as_array()
            .ok_or("items")?
            .is_empty()
    );
    publisher.rollback().await?;
    let expired = page(&db, Some(after), 500).await?;
    assert_eq!(expired["items"].as_array().ok_or("items")?.len(), 1);
    assert_eq!(expired["items"][0]["revoked"], true);
    assert_eq!(expired["items"][0]["decision_revision"], 2);
    assert_eq!(expired["items"][0]["ends_at_ms"], serde_json::Value::Null);
    assert_eq!(
        read_page(
            &db.pool,
            Page {
                after: Some(after),
                limit: 500
            }
        )
        .await?,
        include_bytes!(
            "../../../../../docs/contracts/fixtures/sanction-export-v1/expired-namelock.json"
        )
    );
    let mut tx = db.pool.begin().await?;
    assert!(
        matches!(
            service.allocate_and_record(&mut tx, &intent).await,
            Err(super::super::Error::Conflict)
        ),
        "an expired namelock needs a new stream identity"
    );
    tx.rollback().await?;
    // A publisher that started before expiry can have waited behind the poll's
    // counter lock. The publication trigger must reject revival after revocation
    // even if that writer's earlier application-level eligibility check passed.
    let mut delayed = db.pool.begin().await?;
    let revival = sqlx::query("INSERT INTO support_sanction_revisions(account_id,revision,sanction_id,intent,active,decision_at,expires_at) SELECT account_id,revision+1,sanction_id,intent,true,clock_timestamp(),clock_timestamp()+interval '1 hour' FROM support_sanction_revisions WHERE sanction_id=$1 ORDER BY revision DESC LIMIT 1")
        .bind(intent.sanction_id).execute(&mut *delayed).await;
    assert!(
        revival.is_err(),
        "a delayed writer revived an automatically revoked stream"
    );
    delayed.rollback().await?;
    assert_eq!(
        page(&db, None, 500).await?["items"]
            .as_array()
            .ok_or("items")?
            .len(),
        2
    );
    db.cleanup().await
}
