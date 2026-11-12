#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

const REQUEST: &str = "01934f10-7c02-7001-805b-3b1122334401";
const PACKAGE: &str = "01934f10-7c02-7001-805b-3b1122334402";
const QUOTE: &str = "01934f10-7c02-7001-805b-3b1122334403";
const CHARACTER: &str = "01934f10-7c02-7001-805b-3b1122334404";

fn body(character: &str) -> String {
    format!(
        r#"{{"version":1,"request_id":"{REQUEST}","package_id":"{PACKAGE}","quote_id":"{QUOTE}","character_id":{character}}}"#
    )
}

#[test]
fn order_request_is_strict() {
    let request = CoinOrderRequest::decode(body("null").as_bytes()).unwrap();
    assert_eq!(request.character_id, None);
    assert_eq!(request.request_id.to_string(), REQUEST);
    let with = CoinOrderRequest::decode(body(&format!("\"{CHARACTER}\"")).as_bytes()).unwrap();
    assert_eq!(with.character_id.unwrap().to_string(), CHARACTER);
    for bad in [
        // nullable member missing
        format!(
            r#"{{"version":1,"request_id":"{REQUEST}","package_id":"{PACKAGE}","quote_id":"{QUOTE}"}}"#
        ),
        // unknown member
        body("null").replace("}", r#","coins":5}"#),
        // unsupported versions and forms
        body("null").replace(r#""version":1"#, r#""version":2"#),
        body("null").replace(r#""version":1"#, r#""version":1.0"#),
        body("null").replace(r#""version":1"#, r#""version":"1""#),
        // upper case, v4 and braced IDs
        body("null").replace(REQUEST, &REQUEST.to_uppercase()),
        body("null").replace(REQUEST, "6f1c1c1e-6b1a-4c1a-8f1a-1a1a1a1a1a1a"),
        body("null").replace(REQUEST, &format!("{{{REQUEST}}}")),
        body("null").replace(REQUEST, "00000000-0000-0000-0000-000000000000"),
        body(r#""not-a-uuid""#),
        // duplicate member
        body("null").replace(r#""version":1"#, r#""version":1,"version":1"#),
        String::new(),
        "[]".into(),
    ] {
        assert!(CoinOrderRequest::decode(bad.as_bytes()).is_none(), "{bad}");
    }
    let padded = format!("{}{}", body("null"), " ".repeat(MAX_REQUEST_BYTES));
    assert!(CoinOrderRequest::decode(padded.as_bytes()).is_none());
}

#[test]
fn lifecycle_never_moves_backwards() {
    use OrderState::*;
    let all = [
        AwaitingPayment,
        Processing,
        Paid,
        Failed,
        Refunded,
        Chargeback,
    ];
    for next in all {
        assert_eq!(AwaitingPayment.may_become(next), next != AwaitingPayment);
        assert!(!Failed.may_become(next));
        assert!(!Chargeback.may_become(next));
        assert!(!Processing.may_become(AwaitingPayment));
        assert_eq!(OrderState::parse(next.as_str()), Some(next));
        assert!((1..=96).contains(&next.code().len()));
    }
    assert!(Paid.may_become(Refunded) && Paid.may_become(Chargeback));
    assert!(!Paid.may_become(Failed) && !Paid.may_become(Processing));
    assert!(Refunded.may_become(Chargeback) && !Refunded.may_become(Paid));
    assert_eq!(
        serde_json::to_string(&AwaitingPayment).unwrap(),
        r#""awaiting_payment""#
    );
}

#[test]
fn settings_default_off_and_never_production() {
    let env = |pairs: &'static [(&'static str, &'static str)]| {
        move |key: &str| {
            pairs
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_owned())
        }
    };
    assert!(!CoinSettings::from_lookup(|_| None).enabled);
    let on = CoinSettings::from_lookup(env(&[
        ("PORTAL_COIN_PURCHASES_ENABLED", "true"),
        ("OTERYN_ENV", "testing"),
        ("OTERYN_PUBLIC_ORIGIN", "https://portal.example"),
    ]));
    assert!(on.enabled && on.packages.is_empty());
    for denied in [
        env(&[
            ("OTERYN_ENV", "testing"),
            ("OTERYN_PUBLIC_ORIGIN", "https://portal.example"),
        ]),
        env(&[
            ("PORTAL_COIN_PURCHASES_ENABLED", "true"),
            ("OTERYN_ENV", "production"),
            ("OTERYN_PUBLIC_ORIGIN", "https://portal.example"),
        ]),
        env(&[
            ("PORTAL_COIN_PURCHASES_ENABLED", "true"),
            ("OTERYN_ENV", "testing"),
            ("OTERYN_PUBLIC_ORIGIN", "https://portal.example/"),
        ]),
        env(&[
            ("PORTAL_COIN_PURCHASES_ENABLED", "true"),
            ("OTERYN_ENV", "testing"),
            ("OTERYN_PUBLIC_ORIGIN", "https://portal.example"),
            ("PORTAL_COIN_PACKAGES", "{}"),
        ]),
    ] {
        assert!(!CoinSettings::from_lookup(denied).enabled);
    }
    assert_eq!(
        on.checkout_url(Uuid::parse_str(REQUEST).unwrap()),
        format!("https://portal.example/native-account/coin-checkout/{REQUEST}")
    );
}

#[test]
fn origins_and_packages_are_validated() {
    for good in [
        "https://portal.example",
        "https://a.b:8443",
        "http://127.0.0.1:8080",
        "http://localhost",
    ] {
        assert!(valid_origin(good), "{good}");
    }
    for bad in [
        "http://portal.example",
        "https://user@portal.example",
        "https://portal.example/x",
        "https://portal.example?x",
        "https://portal.example#x",
        "https://Portal.example",
        "https://portal.example:0",
        "ftp://portal.example",
        "https://",
    ] {
        assert!(!valid_origin(bad), "{bad}");
    }
    let one = format!(
        r#"[{{"id":"{PACKAGE}","title":"500 coins","coins":500,"price":{{"minor_units":499,"currency":"EUR","exponent":2}},"valid_for_seconds":120}}]"#
    );
    let packages = parse_packages(&one).unwrap();
    assert_eq!(packages[0].price.display(), "4.99 EUR");
    for bad in [
        one.replace("\"coins\":500", "\"coins\":0"),
        one.replace("120", "301"),
        one.replace("\"exponent\":2", "\"exponent\":4"),
        one.replace("EUR", "eur"),
        one.replace("500 coins", ""),
        one.replace("}]", r#","extra":1}]"#),
        format!("[{},{}]", &one[1..one.len() - 1], &one[1..one.len() - 1]),
    ] {
        assert!(parse_packages(&bad).is_none(), "{bad}");
    }
    let price = |minor_units, exponent| Price {
        minor_units,
        currency: "PLN".into(),
        exponent,
    };
    assert_eq!(price(5, 2).display(), "0.05 PLN");
    assert_eq!(price(1234, 0).display(), "1234 PLN");
    assert_eq!(price(1000, 3).display(), "1.000 PLN");
}

#[test]
fn only_an_exact_checkout_path_is_a_return_target() {
    let order = Uuid::parse_str(REQUEST).unwrap();
    assert!(is_checkout_return(&checkout_path(order)));
    for refused in [
        format!("{CHECKOUT_PATH}{REQUEST}?x=1"),
        format!("{CHECKOUT_PATH}{REQUEST}#f"),
        format!("{CHECKOUT_PATH}{REQUEST}/"),
        format!("{CHECKOUT_PATH}{}", REQUEST.to_uppercase()),
        format!("//evil.example{CHECKOUT_PATH}{REQUEST}"),
        format!("https://evil.example{CHECKOUT_PATH}{REQUEST}"),
        format!("{CHECKOUT_PATH}01934f10-7c02-4001-805b-3b1122334401"),
        CHECKOUT_PATH.to_owned(),
        "/pl/".to_owned(),
    ] {
        assert!(!is_checkout_return(&refused), "{refused}");
    }
}
