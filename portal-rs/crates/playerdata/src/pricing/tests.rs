#![allow(clippy::unwrap_used, clippy::panic)]

use super::*;
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

// SOURCE: Oteryn/Oteryn-Game @857f15f376907c592bb28d854fccff18a15cdf87,
// content/npcs/definitions/npcs-00000-00499.json (services and identity only),
// content/services/trade/trade-00000-00323.json (first declaration).
// Only the small pricing-relevant subset is retained; no source bindings copied.
fn fixture() -> (Value, Value, Item) {
    (
        json!({"declaration": {"kind":"NPC", "identity":{"key":"oteryn:npc.a_fluffy_squirrel","revision":"definition-r1"},
        "services":[{"family":"Service","key":"oteryn:service.trade.a_fluffy_squirrel","revision":"definition-r1"}]}}),
        json!({"declaration": {"kind":"Service", "identity":{"key":"oteryn:service.trade.a_fluffy_squirrel","revision":"definition-r1"},
        "fields":[], "offers":[
            {"direction":"BuyFromPlayer","item":{"family":"Item","key":"oteryn:item.tibia.i10296","revision":"definition-r1"},"unit_price":10},
            {"direction":"BuyFromPlayer","item":{"family":"Item","key":"oteryn:item.tibia.i836","revision":"definition-r1"},"unit_price":80}]}}),
        Item {
            key: "oteryn:item.tibia.i10296".into(),
            revision: "definition-r1".into(),
        },
    )
}
fn npc_selection() -> Selection {
    Selection::Npc {
        npc_key: "oteryn:npc.a_fluffy_squirrel".into(),
        service_key: "oteryn:service.trade.a_fluffy_squirrel".into(),
    }
}

#[test]
fn npc_direction_revision_and_ambiguity() {
    let (npc, mut service, mut item) = fixture();
    let select = |s: &Value, i: &Item, p| {
        npc::select(
            &npc,
            s,
            i,
            p,
            "oteryn:npc.a_fluffy_squirrel",
            "oteryn:service.trade.a_fluffy_squirrel",
        )
    };
    assert_eq!(select(&service, &item, Purpose::Loot).unwrap(), 10);
    assert!(select(&service, &item, Purpose::Supplies).is_err());
    let mut supplies = service.clone();
    supplies["declaration"]["offers"][0]["direction"] = json!("SellToPlayer");
    assert_eq!(select(&supplies, &item, Purpose::Supplies).unwrap(), 10);
    item.revision = "other".into();
    assert!(select(&service, &item, Purpose::Loot).is_err());
    item.revision = "definition-r1".into();
    let offer = service["declaration"]["offers"][0].clone();
    service["declaration"]["offers"]
        .as_array_mut()
        .unwrap()
        .push(offer);
    assert!(select(&service, &item, Purpose::Loot).is_err());
}

#[test]
fn npc_unknown_conditions_currency_and_bundle_deny() {
    let (npc, service, item) = fixture();
    for (field, value) in [
        ("count", json!(500)),
        (
            "currency",
            json!({"family":"Item","key":"token","revision":"r1"}),
        ),
        ("unit_price", json!(-1)),
    ] {
        let mut s = service.clone();
        s["declaration"]["offers"][0][field] = value;
        assert!(
            npc::select(
                &npc,
                &s,
                &item,
                Purpose::Loot,
                "oteryn:npc.a_fluffy_squirrel",
                "oteryn:service.trade.a_fluffy_squirrel"
            )
            .is_err()
        );
    }
    let mut s = service;
    s["declaration"]["fields"] = json!([{"unknown":"condition"}]);
    assert!(
        npc::select(
            &npc,
            &s,
            &item,
            Purpose::Loot,
            "oteryn:npc.a_fluffy_squirrel",
            "oteryn:service.trade.a_fluffy_squirrel"
        )
        .is_err()
    );
}

#[test]
fn closed_preference_schema_and_bounds() {
    assert!(
        serde_json::from_value::<Selection>(
            json!({"source":"custom","unit_gold":12,"account":"other"})
        )
        .is_err()
    );
    assert!(serde_json::from_value::<Selection>(json!({"source":"unknown"})).is_err());
    for amount in [-1, GOLD_CEILING + 1] {
        assert!(validate_selection(&Selection::Custom { unit_gold: amount }).is_err());
    }
    assert!(validate_selection(&Selection::Custom { unit_gold: 0 }).is_ok());
    assert!(!valid_item(&Item {
        key: "<script>".into(),
        revision: "r1".into()
    }));
}

#[derive(Debug, Clone)]
struct TestAccess(Arc<AtomicBool>);
impl Access for TestAccess {
    async fn allows(
        &self,
        _: &mut PgConnection,
        actor: Uuid,
        context: &Context,
    ) -> Result<bool, Error> {
        Ok(actor == context.account && self.0.load(Ordering::SeqCst))
    }
}
#[derive(Debug, Clone)]
struct Market(Option<MarketPrice>);
impl BazaarPrices for Market {
    async fn estimate(
        &self,
        _: &Context,
        _: &Item,
        _: Purpose,
    ) -> Result<Option<MarketPrice>, Error> {
        Ok(self.0.clone())
    }
}

#[derive(Debug, Clone)]
struct InterleavingMarket {
    pool: PgPool,
    access: TestAccess,
    price: MarketPrice,
    change_selection: bool,
}
impl BazaarPrices for InterleavingMarket {
    async fn estimate(
        &self,
        context: &Context,
        _: &Item,
        _: Purpose,
    ) -> Result<Option<MarketPrice>, Error> {
        if self.change_selection {
            sqlx::query("UPDATE pd_price_overrides SET selection=$1 WHERE account_id=$2")
                .bind(sqlx::types::Json(Selection::Custom { unit_gold: 99 }))
                .bind(context.account)
                .execute(&self.pool)
                .await?;
        } else {
            self.access.0.store(false, Ordering::SeqCst);
        }
        Ok(Some(self.price.clone()))
    }
}

struct Database {
    admin: PgPool,
    pool: PgPool,
    schema: String,
    context: Context,
    access: TestAccess,
    item: Item,
}
impl Database {
    async fn new() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let url = match std::env::var("PORTAL_TEST_DATABASE_URL") {
            Ok(url) => url,
            Err(std::env::VarError::NotPresent) => {
                eprintln!("SKIPPED PostgreSQL pricing test: PORTAL_TEST_DATABASE_URL is unset");
                return Ok(None);
            }
            Err(e) => return Err(e.into()),
        };
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await?;
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&admin)
            .await?;
        assert_eq!(version, "180006");
        let schema = format!("pricing_test_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(1)
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
        oteryn_db::migrate(&pool).await?;
        let context = Context {
            account: Uuid::now_v7(),
            character: Uuid::now_v7(),
            world: Uuid::now_v7(),
            cut: ReceiptCut {
                character_revision: 1,
                from_projection_revision: 1,
                to_projection_revision: 2,
                projection_epoch: 1,
                recovery_generation: 1,
            },
            catalogue_version: Uuid::now_v7(),
            catalogue_revision: "fixture.v1".into(),
        };
        let (npc, service, item) = fixture();
        sqlx::query("INSERT INTO world_versions (id,source_commit,status,document_count) VALUES ($1,$2,'ready',3)")
            .bind(context.catalogue_version).bind("a".repeat(40)).execute(&pool).await?;
        sqlx::query("UPDATE world_active_version SET version_id=$1 WHERE singleton")
            .bind(context.catalogue_version)
            .execute(&pool)
            .await?;
        for (kind, key, data) in [
            ("NPC", "oteryn:npc.a_fluffy_squirrel", npc),
            ("Service", "oteryn:service.trade.a_fluffy_squirrel", service),
            (
                "Item",
                item.key.as_str(),
                json!({"declaration":{"identity":{"key":item.key,"revision":item.revision}}}),
            ),
        ] {
            sqlx::query("INSERT INTO world_entities (version_id,kind,key,data,publishable) VALUES ($1,$2,$3,$4,true)")
                .bind(context.catalogue_version).bind(kind).bind(key).bind(data).execute(&pool).await?;
            sqlx::query("INSERT INTO world_visibility (kind,key,allowed) VALUES ($1,$2,true)")
                .bind(kind)
                .bind(key)
                .execute(&pool)
                .await?;
        }
        Ok(Some(Self {
            admin,
            pool,
            schema,
            context,
            access: TestAccess(Arc::new(AtomicBool::new(true))),
            item,
        }))
    }
    fn service(&self, bazaar: Market) -> Pricing<TestAccess, Market> {
        Pricing::new(
            self.pool.clone(),
            WorldStore::new(
                self.pool.clone(),
                oteryn_world::WorldOptions { enabled: true },
            ),
            self.access.clone(),
            bazaar,
            Flags {
                enabled: true,
                bazaar_enabled: true,
            },
        )
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

#[tokio::test]
async fn postgres_custom_persistence_isolation_and_clear() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let s = db.service(Market(None));
    let c = &db.context;
    let item = &db.item;
    assert!(matches!(
        s.estimate(c.account, c, item, Purpose::Loot, 1).await,
        Err(Error::Unavailable)
    ));
    s.set_selection(
        c.account,
        c,
        item,
        Purpose::Loot,
        Selection::Custom { unit_gold: 12 },
    )
    .await?;
    let quote = s.estimate(c.account, c, item, Purpose::Loot, 3).await?;
    assert_eq!(quote.total_gold, 36);
    assert!(matches!(quote.provenance, Provenance::Player { .. }));
    assert!(matches!(
        s.estimate(c.account, c, item, Purpose::Supplies, 3).await,
        Err(Error::Unavailable)
    ));
    let mut other = c.clone();
    other.world = Uuid::now_v7();
    assert!(matches!(
        s.estimate(other.account, &other, item, Purpose::Loot, 3)
            .await,
        Err(Error::Unavailable)
    ));
    other = c.clone();
    other.account = Uuid::now_v7();
    assert!(matches!(
        s.estimate(other.account, &other, item, Purpose::Loot, 3)
            .await,
        Err(Error::Unavailable)
    ));
    s.clear_selection(c.account, c, item, Purpose::Loot).await?;
    assert!(matches!(
        s.estimate(c.account, c, item, Purpose::Loot, 3).await,
        Err(Error::Unavailable)
    ));
    db.close().await
}

#[tokio::test]
async fn postgres_owner_refusal_revocation_bad_input_and_overflow() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let s = db.service(Market(None));
    let c = &db.context;
    let item = &db.item;
    let stranger = Uuid::now_v7();
    assert!(matches!(
        s.set_selection(
            stranger,
            c,
            item,
            Purpose::Loot,
            Selection::Custom { unit_gold: 1 }
        )
        .await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        s.clear_selection(stranger, c, item, Purpose::Loot).await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        s.estimate(stranger, c, item, Purpose::Loot, 1).await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        s.set_selection(
            c.account,
            c,
            item,
            Purpose::Loot,
            Selection::Custom { unit_gold: -1 }
        )
        .await,
        Err(Error::BadInput)
    ));
    s.set_selection(
        c.account,
        c,
        item,
        Purpose::Loot,
        Selection::Custom {
            unit_gold: GOLD_CEILING,
        },
    )
    .await?;
    assert!(matches!(
        s.estimate(c.account, c, item, Purpose::Loot, 0).await,
        Err(Error::BadInput)
    ));
    assert!(matches!(
        s.estimate(c.account, c, item, Purpose::Loot, i64::MAX)
            .await,
        Err(Error::BadInput)
    ));
    db.access.0.store(false, Ordering::SeqCst);
    assert!(matches!(
        s.estimate(c.account, c, item, Purpose::Loot, 1).await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        s.clear_selection(c.account, c, item, Purpose::Loot).await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        s.set_selection(
            c.account,
            c,
            item,
            Purpose::Loot,
            Selection::Custom { unit_gold: 1 }
        )
        .await,
        Err(Error::Denied)
    ));
    let mut disabled = s.clone();
    disabled.flags = Flags::default();
    assert!(matches!(
        disabled
            .estimate(c.account, c, item, Purpose::Loot, 1)
            .await,
        Err(Error::Disabled)
    ));
    let deny = Pricing::new(
        db.pool.clone(),
        s.world.clone(),
        DenyAccess,
        NoBazaarPrices,
        Flags {
            enabled: true,
            bazaar_enabled: false,
        },
    );
    assert!(matches!(
        deny.estimate(c.account, c, item, Purpose::Loot, 1).await,
        Err(Error::Denied)
    ));
    db.close().await
}

#[tokio::test]
async fn postgres_npc_provenance_and_unknown_catalogue() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let s = db.service(Market(None));
    let c = &db.context;
    let item = &db.item;
    s.set_selection(c.account, c, item, Purpose::Loot, npc_selection())
        .await?;
    let quote = s.estimate(c.account, c, item, Purpose::Loot, 4).await?;
    assert_eq!(quote.total_gold, 40);
    assert!(matches!(quote.provenance, Provenance::Npc { .. }));
    s.set_selection(c.account, c, item, Purpose::Supplies, npc_selection())
        .await?;
    assert!(matches!(
        s.estimate(c.account, c, item, Purpose::Supplies, 1).await,
        Err(Error::Unavailable)
    ));
    let mut wrong = c.clone();
    wrong.catalogue_version = Uuid::now_v7();
    assert!(matches!(
        s.estimate(c.account, &wrong, item, Purpose::Loot, 1).await,
        Err(Error::Unavailable)
    ));
    let unknown = Item {
        key: item.key.clone(),
        revision: "other".into(),
    };
    assert!(matches!(
        s.set_selection(
            c.account,
            c,
            &unknown,
            Purpose::Loot,
            Selection::Custom { unit_gold: 1 }
        )
        .await,
        Err(Error::Unavailable)
    ));
    sqlx::query("UPDATE world_visibility SET allowed=false WHERE kind='NPC'")
        .execute(&db.pool)
        .await?;
    assert!(matches!(
        s.estimate(c.account, c, item, Purpose::Loot, 1).await,
        Err(Error::Unavailable)
    ));
    db.close().await
}

#[tokio::test]
async fn postgres_market_evidence_freshness_binding_and_flags() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let c = &db.context;
    let item = &db.item;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&db.pool)
            .await?;
    let market = MarketPrice {
        world: c.world,
        item: item.clone(),
        unit_gold: 17,
        observed_at: now - 60,
        valid_until: now + 3600,
        sample_count: 5,
        provenance: "settled-gold-sales.v1".into(),
    };
    let s = db.service(Market(Some(market.clone())));
    s.set_selection(c.account, c, item, Purpose::Loot, Selection::Bazaar)
        .await?;
    assert_eq!(
        s.estimate(c.account, c, item, Purpose::Loot, 2)
            .await?
            .total_gold,
        34
    );
    for bad in [
        MarketPrice {
            valid_until: now - 1,
            ..market.clone()
        },
        MarketPrice {
            sample_count: 0,
            ..market.clone()
        },
        MarketPrice {
            world: Uuid::now_v7(),
            ..market.clone()
        },
        MarketPrice {
            unit_gold: -1,
            ..market.clone()
        },
        MarketPrice {
            observed_at: now + 999,
            ..market.clone()
        },
    ] {
        assert!(matches!(
            db.service(Market(Some(bad)))
                .estimate(c.account, c, item, Purpose::Loot, 1)
                .await,
            Err(Error::Unavailable)
        ));
    }
    assert!(matches!(
        db.service(Market(None))
            .estimate(c.account, c, item, Purpose::Loot, 1)
            .await,
        Err(Error::Unavailable)
    ));
    let mut disabled = s;
    disabled.flags.bazaar_enabled = false;
    assert!(matches!(
        disabled
            .estimate(c.account, c, item, Purpose::Loot, 1)
            .await,
        Err(Error::Disabled)
    ));
    for change_selection in [true, false] {
        db.access.0.store(true, Ordering::SeqCst);
        let interleaving = Pricing::new(
            db.pool.clone(),
            disabled.world.clone(),
            db.access.clone(),
            InterleavingMarket {
                pool: db.pool.clone(),
                access: db.access.clone(),
                price: market.clone(),
                change_selection,
            },
            Flags {
                enabled: true,
                bazaar_enabled: true,
            },
        );
        // The preceding race changed the choice; restore it for each independent interleaving.
        db.service(Market(Some(market.clone())))
            .set_selection(c.account, c, item, Purpose::Loot, Selection::Bazaar)
            .await?;
        let result = interleaving
            .estimate(c.account, c, item, Purpose::Loot, 1)
            .await;
        if change_selection {
            assert!(matches!(result, Err(Error::Unavailable)));
        } else {
            assert!(matches!(result, Err(Error::Denied)));
        }
    }
    db.close().await
}
