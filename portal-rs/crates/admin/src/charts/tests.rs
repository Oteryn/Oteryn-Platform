#![allow(clippy::unwrap_used)]

use super::*;
use std::collections::BTreeSet;

fn date(value: &str) -> Date {
    Date::parse(value, &Iso8601::DATE).unwrap()
}

#[test]
fn presets_and_custom_dates_use_equal_adjacent_periods() {
    let today = date("2026-11-23");
    for days in [7, 30, 90] {
        let selection = Selection {
            range: Some(days.to_string()),
            ..Selection::default()
        };
        let range = selection.range(today, false).unwrap();
        assert_eq!(range.end, date("2026-11-24"));
        assert_eq!((range.end - range.from).whole_days(), days);
        assert_eq!((range.from - range.previous).whole_days(), days);
    }
    let selection = Selection {
        range: Some("custom".into()),
        from: Some("2024-02-29".into()),
        to: Some("2024-02-29".into()),
        ..Selection::default()
    };
    let range = selection.range(today, false).unwrap();
    assert_eq!(range.previous, date("2024-02-28"));
    assert_eq!(range.end, date("2024-03-01"));
}

#[test]
fn rejects_invalid_dates_ranges_and_worlds() {
    let today = date("2026-11-23");
    for (from, to) in [
        ("2026-02-29", "2026-03-01"),
        ("2026-11-23", "2026-11-22"),
        ("2026-11-23", "2026-11-24"),
        ("2026-01-01", "2026-04-01"),
        ("0001-01-01", "0001-01-03"),
        ("2026-11-1", "2026-11-23"),
    ] {
        let selection = Selection {
            range: Some("custom".into()),
            from: Some(from.into()),
            to: Some(to.into()),
            ..Selection::default()
        };
        assert!(matches!(selection.range(today, false), Err(Error::Invalid)));
    }
    let selection = Selection {
        world: Some(Uuid::now_v7().to_string()),
        ..Selection::default()
    };
    assert!(matches!(selection.range(today, false), Err(Error::Invalid)));
    assert!(selection.range(today, true).is_ok());
    let bad = Selection {
        world: Some("not-a-world".into()),
        ..Selection::default()
    };
    assert!(matches!(bad.range(today, true), Err(Error::Invalid)));
    assert!(matches!(
        Chart::from_file("game-online.csv", false),
        Err(Error::Missing)
    ));
    assert!(matches!(
        Chart::from_file("unknown.csv", true),
        Err(Error::Missing)
    ));
}

#[test]
fn csv_distinguishes_missing_samples_from_zero_and_kpis_compare_periods() {
    let today = date("2026-11-23");
    let range = Selection::default().range(today, true).unwrap();
    let values = BTreeMap::from([
        ("2026-11-12".into(), 2),
        ("2026-11-17".into(), 0),
        ("2026-11-19".into(), 5),
    ]);
    let stats = Card::new(
        Chart::Online,
        BTreeMap::from([(String::new(), values.clone())]),
        range,
        today,
    );
    assert_eq!(stats.series[0].value, Some(5));
    assert_eq!(stats.series[0].change, Some(3));
    assert_eq!(stats.series[0].marks.len(), 2);
    assert!(stats.csv().contains("2026-11-17,game-online,0\r\n"));
    assert!(stats.csv().contains("2026-11-18,game-online,\r\n"));
    let failures = Series::new(Chart::LoginsFailed, String::new(), &values, range, today);
    assert_eq!(failures.value, None);
    assert_eq!(failures.change, None);
    assert_eq!(failures.rows.last().unwrap().1, Some(0));
}

#[test]
fn empty_ledger_has_no_operation_series() {
    let today = date("2026-11-23");
    let range = Selection::default().range(today, true).unwrap();
    let card = Card::new(Chart::Ledger, BTreeMap::new(), range, today);
    assert!(card.series.is_empty());
    assert_eq!(card.csv(), "date_utc,series,value\r\n");
}

#[test]
fn i18n_parity_covers_all_chart_labels() {
    let keys = |source: &'static str| -> BTreeSet<&'static str> {
        source
            .lines()
            .filter_map(|line| line.split_once(" = ").map(|(key, _)| key))
            .collect()
    };
    let polish = keys(include_str!("../../../../templates/admin/charts/pl.ftl"));
    let english = keys(include_str!("../../../../templates/admin/charts/en.ftl"));
    assert_eq!(polish, english);
    for language in ["pl", "en"] {
        let bundle = labels(language).unwrap();
        for key in &english {
            assert!(!text(&bundle, key).is_empty(), "{language}: {key}");
        }
        for chart in Chart::ALL {
            assert!(english.contains(chart.key()));
        }
    }
}

#[tokio::test]
async fn report_requests_share_a_query_budget() {
    let state = Charts {
        pool: sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://unused@127.0.0.1:1/unused")
            .unwrap(),
        extended: false,
        queries: Arc::new(Semaphore::new(2)),
    };
    let _running = state.queries.acquire_many(2).await.unwrap();
    let dashboard = page(
        State(state.clone()),
        Extension(CanExport(false)),
        HeaderMap::new(),
        Query(Selection::default()),
    )
    .await
    .unwrap_err()
    .into_response();
    let download = export(
        State(state.clone()),
        Path("registrations.csv".into()),
        Query(Selection::default()),
    )
    .await
    .unwrap_err()
    .into_response();
    for response in [dashboard, download] {
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers()[header::RETRY_AFTER], "1");
    }
}
