#![allow(clippy::unwrap_used)]

use super::*;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
struct Source(Arc<Mutex<Option<StatsProjection>>>);
impl StatsSource for Source {
    async fn read(&self) -> Result<Option<StatsProjection>, Error> {
        Ok(self.0.lock().map_err(|_| Error::Denied)?.clone())
    }
}
fn fixture() -> (Source, StatsQuery, OffsetDateTime) {
    let now = OffsetDateTime::from_unix_timestamp(1_790_000_000).unwrap();
    let world = Uuid::now_v7();
    let asset = StatsAsset::Item {
        definition: "fixture-sword".into(),
        revision: "v1".into(),
        attributes_hash: [1; 32],
    };
    let query = StatsQuery {
        world,
        currency: Currency::Coins,
        asset: asset.clone(),
        from: now - Duration::days(30),
        until: now,
    };
    let observations = [100, 200, 300, 400, 500, 600]
        .into_iter()
        .map(|unit_price| StatsObservation {
            listing: Uuid::now_v7(),
            world,
            currency: Currency::Coins,
            asset: asset.clone(),
            closed_at: now - Duration::hours(1),
            auction: true,
            outcome: StatsOutcome::Completed {
                unit_price,
                quantity: 2,
            },
        })
        .collect();
    (
        Source(Arc::new(Mutex::new(Some(StatsProjection {
            observed_at: now,
            valid_until: now + Duration::seconds(30),
            observations,
            characters: vec![],
        })))),
        query,
        now,
    )
}
fn edit(source: &Source, change: impl FnOnce(&mut StatsProjection)) {
    change(source.0.lock().unwrap().as_mut().unwrap());
}
fn service(source: Source) -> BazaarStats<Source> {
    BazaarStats::new(source, StatsFlags { enabled: true })
}

#[tokio::test]
async fn history_median_turnover_record_and_terminal_auction_rate() -> Result<(), Error> {
    let (source, query, now) = fixture();
    edit(&source, |p| {
        let mut row = p.observations[0].clone();
        row.listing = Uuid::now_v7();
        row.outcome = StatsOutcome::Unsold;
        p.observations.push(row);
    });
    let summary = service(source).summary(&query, now).await?.unwrap();
    assert_eq!(summary.sales, 6);
    assert_eq!(summary.market_price.twice_units, 700);
    assert_eq!(summary.turnover, 4200);
    assert_eq!(summary.record_unit_price, 600);
    assert_eq!(summary.auction_success, Some((6, 7)));
    assert_eq!(summary.history.len(), 1);
    assert_eq!(summary.history[0].market_price.twice_units, 700);
    Ok(())
}
#[tokio::test]
async fn currencies_worlds_variants_and_windows_never_mix() -> Result<(), Error> {
    let (source, query, now) = fixture();
    edit(&source, |p| {
        let base = p.observations[0].clone();
        for index in 0..6 {
            let mut row = base.clone();
            row.listing = Uuid::now_v7();
            row.outcome = StatsOutcome::Completed {
                unit_price: 9999,
                quantity: 1,
            };
            match index {
                0 => row.currency = Currency::Gold,
                1 => row.world = Uuid::now_v7(),
                2 => {
                    row.asset = StatsAsset::Item {
                        definition: "fixture-sword".into(),
                        revision: "v2".into(),
                        attributes_hash: [1; 32],
                    }
                }
                3 => row.closed_at = query.from - Duration::seconds(1),
                4 => {
                    row.asset = StatsAsset::Item {
                        definition: "fixture-sword".into(),
                        revision: "v1".into(),
                        attributes_hash: [2; 32],
                    }
                }
                _ => row.closed_at = query.until,
            }
            p.observations.push(row);
        }
    });
    assert_eq!(
        service(source).summary(&query, now).await?.unwrap().sales,
        6
    );
    Ok(())
}
#[tokio::test]
async fn small_cohorts_and_daily_groups_are_unavailable() -> Result<(), Error> {
    let (source, query, now) = fixture();
    edit(&source, |p| {
        p.observations[0].closed_at -= Duration::days(1)
    });
    assert_eq!(
        service(source.clone())
            .summary(&query, now)
            .await?
            .unwrap()
            .history
            .len(),
        1
    );
    edit(&source, |p| p.observations.truncate(4));
    assert!(
        service(source.clone())
            .summary(&query, now)
            .await?
            .is_none()
    );
    edit(&source, |p| p.observations.clear());
    assert!(service(source).summary(&query, now).await?.is_none());
    Ok(())
}
#[tokio::test]
async fn defaults_and_missing_authority_deny() {
    let (source, query, now) = fixture();
    assert!(matches!(
        BazaarStats::new(source, StatsFlags::default())
            .summary(&query, now)
            .await,
        Err(Error::Disabled)
    ));
    assert!(matches!(
        BazaarStats::new(UnavailableStatsSource, StatsFlags { enabled: true })
            .summary(&query, now)
            .await,
        Err(Error::Denied)
    ));
}
#[tokio::test]
async fn revocation_is_observed_without_cached_results() -> Result<(), Error> {
    let (source, query, now) = fixture();
    let stats = service(source.clone());
    assert!(stats.summary(&query, now).await?.is_some());
    *source.0.lock().unwrap() = None;
    assert!(matches!(
        stats.summary(&query, now).await,
        Err(Error::Denied)
    ));
    Ok(())
}
#[tokio::test]
async fn stale_future_duplicate_and_oversized_evidence_deny() {
    for case in 0..5 {
        let (source, query, now) = fixture();
        edit(&source, |p| match case {
            0 => p.valid_until = now,
            1 => p.observed_at = now + Duration::seconds(1),
            2 => p.observations.push(p.observations[0].clone()),
            3 => p.observations[0].closed_at = now + Duration::seconds(1),
            _ => p.observations = vec![p.observations[0].clone(); MAX_ROWS + 1],
        });
        assert!(matches!(
            service(source).summary(&query, now).await,
            Err(Error::Denied)
        ));
    }
}
#[tokio::test]
async fn bad_queries_and_amounts_refuse() {
    for case in 0..7 {
        let (source, mut query, now) = fixture();
        match case {
            0 => query.world = Uuid::nil(),
            1 => query.from = query.until,
            2 => query.until = now + Duration::seconds(1),
            3 => query.from = now - Duration::days(91),
            4 => edit(&source, |p| {
                p.observations[0].outcome = StatsOutcome::Completed {
                    unit_price: -1,
                    quantity: 1,
                }
            }),
            5 => edit(&source, |p| {
                p.observations[0].outcome = StatsOutcome::Completed {
                    unit_price: 1,
                    quantity: 0,
                }
            }),
            _ => edit(&source, |p| {
                p.observations[0].currency = Currency::Gold;
                p.observations[0].outcome = StatsOutcome::Completed {
                    unit_price: 999_999_999_999,
                    quantity: 2,
                };
            }),
        }
        assert!(matches!(
            service(source).summary(&query, now).await,
            Err(Error::InvalidInput)
        ));
    }
}
#[test]
fn exact_medians_and_inclusive_price_warning() -> Result<(), Error> {
    assert_eq!(median(&mut [1, 2, 3, 4, 5]).twice_units, 6);
    assert_eq!(median(&mut [1, 2]).twice_units, 3);
    assert_eq!(
        median(&mut [i64::MAX; 6]).twice_units,
        i128::from(i64::MAX) * 2
    );
    let price = MarketPrice { twice_units: 200 };
    assert!(price.warns(75)?);
    assert!(price.warns(125)?);
    assert!(!price.warns(76)?);
    assert!(!price.warns(124)?);
    assert!(price.warns(0).is_err());
    Ok(())
}
fn characters(source: &Source, world: Uuid) -> (Uuid, Uuid) {
    let left = Uuid::now_v7();
    let right = Uuid::now_v7();
    edit(source, |p| {
        p.characters = vec![
            StatsCharacter {
                listing: left,
                world,
                vocation: 4,
                level: 100,
                skills: Some(BTreeMap::from([(StatsSkill::Sword, 80)])),
            },
            StatsCharacter {
                listing: right,
                world,
                vocation: 4,
                level: 110,
                skills: Some(BTreeMap::from([
                    (StatsSkill::Sword, 90),
                    (StatsSkill::Magic, 20),
                ])),
            },
        ];
        for (index, row) in p.observations.iter_mut().enumerate() {
            row.asset = StatsAsset::Character {
                vocation: 4,
                level: 90 + index as u32 * 4,
            };
            row.outcome = StatsOutcome::Completed {
                unit_price: 100 + index as i64,
                quantity: 1,
            };
        }
    });
    (left, right)
}
#[tokio::test]
async fn estimates_use_similar_sales_comparison_uses_supported_facts() -> Result<(), Error> {
    let (source, query, now) = fixture();
    let (left, right) = characters(&source, query.world);
    let stats = service(source.clone());
    let estimate = stats
        .estimate_character(left, Currency::Coins, query.from, now, now)
        .await?
        .unwrap();
    assert_eq!(estimate.sales, 6);
    assert_eq!(estimate.market_price.twice_units, 205);
    let comparison = stats.compare_characters(left, right, now).await?;
    assert_eq!(comparison.level_difference, 10);
    assert!(comparison.same_world && comparison.same_vocation);
    assert_eq!(
        comparison.skill_differences,
        Some(BTreeMap::from([(StatsSkill::Sword, 10)]))
    );
    edit(&source, |p| p.characters[1].skills = None);
    assert_eq!(
        stats
            .compare_characters(left, right, now)
            .await?
            .skill_differences,
        None
    );
    edit(&source, |p| {
        p.observations[0].asset = StatsAsset::Character {
            vocation: 2,
            level: 100,
        };
        p.observations[1].asset = StatsAsset::Character {
            vocation: 4,
            level: 200,
        };
    });
    assert!(
        stats
            .estimate_character(left, Currency::Coins, query.from, now, now)
            .await?
            .is_none()
    );
    Ok(())
}
#[tokio::test]
async fn comparison_refuses_unapproved_and_invalid_characters() {
    let (source, query, now) = fixture();
    let (left, right) = characters(&source, query.world);
    let stats = service(source.clone());
    assert!(matches!(
        stats.compare_characters(left, left, now).await,
        Err(Error::InvalidInput)
    ));
    assert!(matches!(
        stats.compare_characters(left, Uuid::now_v7(), now).await,
        Err(Error::NotFound)
    ));
    edit(&source, |p| p.characters.retain(|c| c.listing != right));
    assert!(matches!(
        stats.compare_characters(left, right, now).await,
        Err(Error::NotFound)
    ));
}

#[tokio::test]
async fn vocation_groups_suppress_small_samples_and_separate_currency() -> Result<(), Error> {
    let (source, query, now) = fixture();
    characters(&source, query.world);
    let stats = service(source.clone());
    let groups = stats
        .vocation_breakdown(query.world, Currency::Coins, query.from, now, now)
        .await?;
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[&4].sales, 6);
    assert!(
        stats
            .vocation_breakdown(query.world, Currency::Gold, query.from, now, now)
            .await?
            .is_empty()
    );
    edit(&source, |p| p.observations.truncate(4));
    assert!(
        stats
            .vocation_breakdown(query.world, Currency::Coins, query.from, now, now)
            .await?
            .is_empty()
    );
    Ok(())
}

#[test]
fn untrusted_price_cannot_overflow_warning() {
    assert!(matches!(
        MarketPrice {
            twice_units: i128::MAX
        }
        .warns(1),
        Err(Error::Overflow)
    ));
}

#[tokio::test]
async fn daily_history_groups_by_utc_date() -> Result<(), Error> {
    let (source, query, now) = fixture();
    edit(&source, |p| {
        p.observations[0].closed_at = p.observations[0]
            .closed_at
            .to_offset(time::UtcOffset::from_hms(23, 0, 0).unwrap());
    });
    let summary = service(source).summary(&query, now).await?.unwrap();
    assert_eq!(summary.history.len(), 1);
    assert_eq!(summary.history[0].sales, 6);
    Ok(())
}

#[tokio::test]
async fn estimate_accepts_a_complete_cut_and_refuses_a_later_window() -> Result<(), Error> {
    let (source, query, now) = fixture();
    let (left, _) = characters(&source, query.world);
    let cut = now - Duration::seconds(10);
    edit(&source, |p| p.observed_at = cut);
    let stats = service(source);
    assert!(
        stats
            .estimate_character(left, Currency::Coins, query.from, cut, now)
            .await?
            .is_some()
    );
    assert!(matches!(
        stats
            .estimate_character(left, Currency::Coins, query.from, now, now)
            .await,
        Err(Error::Denied)
    ));
    Ok(())
}
