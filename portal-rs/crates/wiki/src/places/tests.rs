#![allow(clippy::unwrap_used)]
use super::*;
#[test]
fn success_refusal_and_bad_input() {
    let card = Card {
        key: "place:cave".into(),
        title: "Cave".into(),
        purpose: Purpose::Hunt,
        level: 20,
        party_size: 1,
        experience_per_hour: Some(100),
        profit_per_hour: Some(-10),
        points: None,
        completion_seconds: None,
        editorial_summary: "<script>".into(),
    };
    assert_eq!(
        select(vec![card.clone()], &Filters::default())
            .unwrap()
            .len(),
        1
    );
    assert!(
        select(
            vec![],
            &Filters {
                party: Some(3),
                ..Default::default()
            }
        )
        .is_err()
    );
    let rendered = Page {
        locale: "en".into(),
        title: text("en", "title").unwrap(),
        unavailable: text("en", "unavailable").unwrap(),
        reports: text("en", "reports").unwrap(),
        cards: vec![card],
        labels: vec![String::new(); 6],
    }
    .render()
    .unwrap();
    assert!(!rendered.contains("<script>"));
    assert!(text("de", "title").is_err());
}

#[derive(Debug)]
struct Empty;
impl PublicPlaces for Empty {
    fn cards<'a>(&'a self, _: &'a str) -> CardsFuture<'a> {
        Box::pin(async { Ok(vec![]) })
    }
}
#[tokio::test]
async fn http_success_disabled_and_invalid_filters() {
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;
    for (enabled, path, expected) in [
        (true, "/en/wiki/places", StatusCode::OK),
        (true, "/pl/wiki/places", StatusCode::OK),
        (false, "/en/wiki/places", StatusCode::SERVICE_UNAVAILABLE),
        (
            true,
            "/en/wiki/places?sort=invalid",
            StatusCode::BAD_REQUEST,
        ),
        (true, "/en/wiki/places?party=3", StatusCode::BAD_REQUEST),
        (true, "/de/wiki/places", StatusCode::NOT_FOUND),
    ] {
        let response = router(PlacesState {
            enabled,
            source: Arc::new(Empty),
        })
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
        assert_eq!(response.status(), expected);
        if expected == StatusCode::OK {
            let body = to_bytes(response.into_body(), 100_000).await.unwrap();
            assert!(
                String::from_utf8(body.to_vec())
                    .unwrap()
                    .contains("<main class=\"op-stack\">")
            );
        }
    }
}

#[test]
fn numeric_sort_keeps_missing_rates_last_and_filters_party() {
    fn card(key: &str, rate: Option<i64>, party: u8) -> Card {
        Card {
            key: key.into(),
            title: key.into(),
            purpose: Purpose::Hunt,
            level: 20,
            party_size: party,
            experience_per_hour: None,
            profit_per_hour: rate,
            points: None,
            completion_seconds: None,
            editorial_summary: String::new(),
        }
    }
    let filters = Filters {
        sort: Some("profit".into()),
        party: Some(1),
        purpose: None,
    };
    let result = select(
        vec![
            card("unknown", None, 1),
            card("negative", Some(-20), 1),
            card("positive", Some(100), 1),
            card("duo", Some(1000), 2),
        ],
        &filters,
    )
    .unwrap();
    assert_eq!(
        result.iter().map(|c| c.key.as_str()).collect::<Vec<_>>(),
        vec!["positive", "negative", "unknown"]
    );
    assert!(select(vec![card("bad", Some(1), 3)], &Filters::default()).is_err());
}

#[tokio::test]
async fn enabled_without_qualified_source_still_denies() {
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    let response = router(PlacesState {
        enabled: true,
        source: Arc::new(Unavailable),
    })
    .oneshot(
        Request::builder()
            .uri("/en/wiki/places")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(response.headers()["cache-control"], "private, no-store");
}
