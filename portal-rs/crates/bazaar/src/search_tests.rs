use super::*;
use crate::AccountState;

fn candidate(id: Uuid) -> SearchCandidate {
    let now = OffsetDateTime::now_utc();
    SearchCandidate {
        listing: SearchListing {
            id,
            name: "Example Knight".into(),
            kind: "character".into(),
            form: "auction".into(),
            currency: "coins".into(),
            price: Some(100),
            level: Some(120),
            achievement_points: Some(10),
            skills: BTreeMap::from([("sword".into(), 90)]),
            facets: BTreeMap::from([(SearchField::World, vec!["world-1".into()])]),
            has_bids: true,
            rare_name: None,
            created_at: now,
            ends_at: now + Duration::days(1),
            ended: false,
            sale_price: None,
        },
        publication_authorized: true,
        privacy_current: true,
        clock_safe: true,
        projection_epoch: 1,
        highest_epoch: 1,
        complete_through: now,
        observed_at: now,
        mine: true,
        my_bid: true,
    }
}

#[test]
fn filters_require_known_approved_values() {
    let row = candidate(Uuid::now_v7());
    let mut q = SearchQuery {
        text: "KNIGHT".into(),
        level_min: Some(120),
        price_max: Some(100),
        skill: Some("sword".into()),
        skill_min: Some(90),
        ..SearchQuery::default()
    };
    assert!(q.validate().is_ok());
    assert!(matches_query(&row.listing, &q));
    q.rare_name = true;
    assert!(!matches_query(&row.listing, &q));
    q.rare_name = false;
    q.facets.insert(SearchField::Quest, "unknown".into());
    assert!(!matches_query(&row.listing, &q));
}

#[test]
fn bad_input_denies() {
    for q in [
        SearchQuery {
            level_min: Some(20),
            level_max: Some(10),
            ..SearchQuery::default()
        },
        SearchQuery {
            price_min: Some(-1),
            ..SearchQuery::default()
        },
        SearchQuery {
            skill: Some("unknown".into()),
            ..SearchQuery::default()
        },
        SearchQuery {
            sort: SearchSort::SkillAsc,
            ..SearchQuery::default()
        },
        SearchQuery {
            text: "\n".into(),
            ..SearchQuery::default()
        },
    ] {
        assert!(matches!(q.validate(), Err(Error::InvalidInput)));
    }
    assert!(serde_json::from_str::<SearchQuery>(r#"{"private_snapshot":true}"#).is_err());
}

#[test]
fn freshness_and_revocation_fail_closed() {
    let now = OffsetDateTime::now_utc();
    let mut r = candidate(Uuid::now_v7());
    assert!(r.visible(now));
    r.privacy_current = false;
    assert!(!r.visible(now));
    r.privacy_current = true;
    r.publication_authorized = false;
    assert!(!r.visible(now));
    r.publication_authorized = true;
    r.highest_epoch = 2;
    assert!(!r.visible(now));
    r.highest_epoch = 1;
    r.complete_through = now - Duration::seconds(30);
    assert!(!r.visible(now));
    r.complete_through = now;
    r.observed_at = now + Duration::seconds(2);
    assert!(!r.visible(now));
}

struct TestAccounts;
impl Accounts for TestAccounts {
    async fn state(
        &self,
        _: &mut PgConnection,
        actor: Uuid,
    ) -> Result<Option<AccountState>, Error> {
        Ok((!actor.is_nil()).then_some(AccountState {
            active: true,
            ..AccountState::default()
        }))
    }
}
struct TestProjection {
    id: Uuid,
}
impl SearchProjection for TestProjection {
    async fn candidates(
        &self,
        _: &mut PgConnection,
        _: Option<Uuid>,
    ) -> Result<Vec<SearchCandidate>, Error> {
        let mut hidden = candidate(Uuid::now_v7());
        hidden.privacy_current = false;
        Ok(vec![hidden, candidate(self.id)])
    }
}

struct ManyProjection;
impl SearchProjection for ManyProjection {
    async fn candidates(
        &self,
        _: &mut PgConnection,
        _: Option<Uuid>,
    ) -> Result<Vec<SearchCandidate>, Error> {
        let mut rows = Vec::new();
        for price in 1..=30 {
            let mut row = candidate(Uuid::now_v7());
            row.listing.price = Some(price);
            rows.push(row);
        }
        let mut ended = candidate(Uuid::now_v7());
        ended.listing.ended = true;
        ended.listing.sale_price = Some(450);
        rows.push(ended);
        let mut unknown = candidate(Uuid::now_v7());
        unknown.listing.currency = "unknown".into();
        rows.push(unknown);
        let mut gold = candidate(Uuid::now_v7());
        gold.listing.currency = "gold".into();
        rows.push(gold);
        Ok(rows)
    }
}

#[tokio::test]
async fn postgres_preferences_and_private_tabs() -> Result<(), Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED search PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let pool = PgPool::connect(&url).await?;
    let schema = format!("search_test_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&pool)
        .await?;
    let path = schema.clone();
    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(3)
        .after_connect(move |db, _| {
            let path = path.clone();
            Box::pin(async move {
                sqlx::query(sqlx::AssertSqlSafe(format!("SET search_path TO {path}")))
                    .execute(db)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261105152830_t57_bazaar_core.sql"
    ))
    .execute(&db)
    .await?;
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261105230104_t61_search.sql"
    ))
    .execute(&db)
    .await?;
    let actor = Uuid::now_v7();
    let other = Uuid::now_v7();
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,ends_at) VALUES($1,$2,'character','auction','coins',100,false,clock_timestamp()+interval '1 day')")
        .bind(id).bind(actor).execute(&db).await?;
    let s = BazaarSearch::new(
        db.clone(),
        TestAccounts,
        TestProjection { id },
        SearchFlags {
            public_listings_enabled: true,
            account_features_enabled: true,
            gold_enabled: false,
        },
    );
    let q = SearchQuery::default();
    assert_eq!(s.search(None, &q).await?.total, 1);
    let watched = SearchQuery {
        tab: SearchTab::Watched,
        ..q.clone()
    };
    assert!(matches!(s.search(None, &watched).await, Err(Error::Denied)));
    s.watch(actor, id, true).await?;
    s.watch(actor, id, true).await?;
    assert_eq!(s.search(Some(actor), &watched).await?.total, 1);
    assert_eq!(s.search(Some(other), &watched).await?.total, 0);
    assert!(matches!(
        s.watch(actor, Uuid::now_v7(), true).await,
        Err(Error::NotFound)
    ));
    s.save(actor, "Knights", &q).await?;
    let changed = SearchQuery {
        level_min: Some(150),
        ..q.clone()
    };
    s.save(actor, "Knights", &changed).await?;
    assert_eq!(s.saved(actor).await?, vec![("Knights".into(), changed)]);
    assert!(s.saved(other).await?.is_empty());
    s.delete_saved(other, "Knights").await?;
    assert_eq!(s.saved(actor).await?.len(), 1);
    assert!(matches!(
        s.save(actor, " ", &q).await,
        Err(Error::InvalidInput)
    ));
    assert!(matches!(s.saved(Uuid::nil()).await, Err(Error::Denied)));
    s.watch(actor, id, false).await?;
    s.watch(actor, id, false).await?;
    assert_eq!(s.search(Some(actor), &watched).await?.total, 0);
    s.delete_saved(actor, "Knights").await?;
    assert!(s.saved(actor).await?.is_empty());
    let disabled = BazaarSearch::new(
        db.clone(),
        TestAccounts,
        TestProjection { id },
        SearchFlags::default(),
    );
    assert!(matches!(
        disabled.search(None, &q).await,
        Err(Error::Disabled)
    ));
    assert!(matches!(
        disabled.save(actor, "x", &q).await,
        Err(Error::Disabled)
    ));
    let many = BazaarSearch::new(
        db.clone(),
        TestAccounts,
        ManyProjection,
        SearchFlags {
            public_listings_enabled: true,
            account_features_enabled: true,
            gold_enabled: false,
        },
    );
    let ascending = SearchQuery {
        sort: SearchSort::PriceAsc,
        ..q.clone()
    };
    let first = many.search(None, &ascending).await?;
    assert_eq!(first.total, 30);
    assert_eq!(first.listings.len(), 24);
    assert_eq!(first.listings[0].price, Some(1));
    let second = many
        .search(
            None,
            &SearchQuery {
                page: 1,
                ..ascending
            },
        )
        .await?;
    assert_eq!(second.listings.len(), 6);
    assert_eq!(second.listings[0].price, Some(25));
    let descending = many
        .search(
            None,
            &SearchQuery {
                sort: SearchSort::PriceDesc,
                ..q.clone()
            },
        )
        .await?;
    assert_eq!(descending.listings[0].price, Some(30));
    let ended = many
        .search(
            None,
            &SearchQuery {
                tab: SearchTab::Ended,
                price_min: Some(400),
                ..q.clone()
            },
        )
        .await?;
    assert_eq!(ended.total, 1);
    assert_eq!(ended.listings[0].price, Some(450));
    for tab in [SearchTab::MyBids, SearchTab::MyListings] {
        assert!(matches!(
            many.search(None, &SearchQuery { tab, ..q.clone() }).await,
            Err(Error::Denied)
        ));
        assert_eq!(
            many.search(Some(actor), &SearchQuery { tab, ..q.clone() })
                .await?
                .total,
            31
        );
    }
    let bad = SearchQuery {
        price_min: Some(-1),
        ..q.clone()
    };
    assert!(matches!(
        many.search(None, &bad).await,
        Err(Error::InvalidInput)
    ));
    db.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&pool)
        .await?;
    pool.close().await;
    Ok(())
}
