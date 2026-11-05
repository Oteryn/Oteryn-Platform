#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
const GOLDEN: &str = r#"{"contract_version":2,"operation":"PublishWorldDailyV2","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","projection_epoch":"2","projection_revision":"17","source_observed_at":"1790003610","daily":[{"reset_epoch":"42","effective_from":"1790000000","effective_until":"1790086400","creature_key":"rat","boss_key":"ferumbras","history_complete_from":"1789913600","history":[{"reset_epoch":"41","effective_from":"1789913600","effective_until":"1790000000","creature_key":"dragon","boss_key":"orshabaal"}],"raids":[{"raid_key":"raid:badger","scheduled_at":"1790003600","state":"running"}]}]}"#;
fn snapshot() -> Snapshot {
    match decode(GOLDEN.as_bytes(), false, 1790003610).unwrap() {
        Publication::Snapshot(s) => s,
        _ => panic!("snapshot expected"),
    }
}
#[test]
fn golden_and_additive_version_refusals() {
    snapshot();
    for bad in [
        GOLDEN.replace("\"contract_version\":2", "\"contract_version\":3"),
        GOLDEN.replace("PublishWorldDailyV2", "UnknownV2"),
        GOLDEN.replace(
            "\"projection_revision\":\"17\"",
            "\"projection_revision\":17",
        ),
        GOLDEN.replace("\"daily\":[", "\"daily\":null,\"other\":["),
        GOLDEN.replace("rat", "r\\u0061t"),
        GOLDEN.replace(
            "\"contract_version\":2,",
            "\"operation\":\"PublishWorldDailyV2\",\"contract_version\":2,",
        ),
    ] {
        assert!(decode(bad.as_bytes(), false, 1790003610).is_err());
    }
    for family in ["unknown", "character_profile", "world_online"] {
        let raw = format!(
            r#"{{"contract_version":2,"operation":"PublishPublicProjectionWatermarkV2","source_authority":"oteryn:game:primary","family":"{family}","projection_epoch":"2","complete_through":"1790003610","observed_at":"1790003610"}}"#
        );
        assert!(decode(raw.as_bytes(), true, 1790003610).is_err());
    }
}
#[test]
fn interval_history_order_and_coverage_bounds() {
    let s = snapshot();
    let mut d = s.daily[0].clone();
    d.reset_epoch = "0".into();
    d.history.clear();
    d.history_complete_from = d.effective_from.clone();
    assert!(d.validate(1790003610).is_ok());
    for bad in ["-1", "01", "18446744073709551616"] {
        d.reset_epoch = bad.into();
        assert!(d.validate(1790003610).is_err());
    }
    let mut d = s.daily[0].clone();
    d.history.push(d.history[0].clone());
    assert!(d.validate(1790003610).is_err());
    let mut d = s.daily[0].clone();
    d.history[0].effective_until = "1790000001".into();
    assert!(d.validate(1790003610).is_err());
    let mut d = s.daily[0].clone();
    d.history_complete_from = "1787407999".into();
    assert!(d.validate(1790003610).is_err());
    let mut d = s.daily[0].clone();
    d.history[0].reset_epoch = d.reset_epoch.clone();
    assert!(d.validate(1790003610).is_err());
    let mut d = s.daily[0].clone();
    d.effective_until = d.effective_from.clone();
    assert!(d.validate(1790003610).is_err());
}
#[test]
fn key_integer_and_schedule_bounds() {
    assert!(decimal("18446744073709551615", false).is_ok());
    assert!(decimal("0", false).is_err());
    assert!(seconds("9223372036854776").is_err());
    assert!(world_id("01934f10-7C02-7001-805b-3b1122334401").is_err());
    let mut d = snapshot().daily.remove(0);
    d.creature_key = "A:/._-9".repeat(36);
    assert!(d.validate(1790003610).is_ok());
    d.creature_key = "a".repeat(256);
    assert!(d.validate(1790003610).is_err());
    let mut d = snapshot().daily.remove(0);
    d.raids = (0..256)
        .map(|i| Raid {
            raid_key: format!("raid:{i:03}"),
            scheduled_at: "1790003600".into(),
            state: "done".into(),
        })
        .collect();
    assert!(d.validate(1790003610).is_ok());
    let bytes = serde_json::to_vec(&Snapshot {
        daily: vec![d.clone()],
        ..snapshot()
    })
    .unwrap();
    assert!(decode(&bytes, false, 1790003610).is_ok());
    d.raids.push(Raid {
        raid_key: "raid:256".into(),
        scheduled_at: "1790003600".into(),
        state: "done".into(),
    });
    assert!(d.validate(1790003610).is_err());
    for bad in ["1790086400", "1789999999", "1790003611"] {
        let mut d = snapshot().daily.remove(0);
        d.raids[0].scheduled_at = bad.into();
        assert!(d.validate(1790003610).is_err());
    }
    let mut d = snapshot().daily.remove(0);
    d.raids.push(d.raids[0].clone());
    assert!(d.validate(1790003610).is_err());
}
