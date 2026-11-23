use super::*;

#[test]
fn dates_are_inclusive_utc_and_bounded() -> Result<(), Box<dyn std::error::Error>> {
    let today = Date::from_calendar_date(2026, time::Month::November, 23)?;
    let window = Dates::default().window(today)?;
    assert_eq!(window.through, today);
    assert_eq!((window.end - window.from).whole_days(), 30);
    let window = Dates {
        from: Some("2026-10-24".into()),
        through: Some("2026-11-23".into()),
        ..Dates::default()
    }
    .window(today)?;
    assert_eq!((window.end - window.from).whole_days(), 31);

    for (from, through) in [
        ("2026-11-22", "2026-11-21"),
        ("2026-11-01", "2026-11-24"),
        ("2026-10-23", "2026-11-23"),
        ("2026-02-29", "2026-11-23"),
        ("2026-W01-1", "2026-11-23"),
        ("2026-11-1", "2026-11-23"),
    ] {
        assert!(matches!(
            Dates {
                from: Some(from.into()),
                through: Some(through.into()),
                ..Dates::default()
            }
            .window(today),
            Err(Error::Invalid)
        ));
    }
    Ok(())
}

#[test]
fn inputs_accept_only_bounded_identifiers_and_valid_case_references() {
    assert!(case_id(Uuid::now_v7()).is_ok());
    assert!(case_id(Uuid::nil()).is_err());
    assert!(optional_uuid(&Uuid::nil().to_string()).is_err());
    assert!(optional_uuid("invalid").is_err());
    assert!(identifier("tibia.World-1_2"));
    for value in ["", "x/y", " x", "ą", &"x".repeat(129)] {
        assert!(!identifier(value));
    }
}

#[test]
fn localized_timeline_labels_cover_the_projection() -> Result<(), Box<dyn std::error::Error>> {
    for lang in ["en", "pl"] {
        let mut headers = HeaderMap::new();
        headers.insert("accept-language", lang.parse()?);
        let page = Page::new(&headers)?;
        assert_eq!(page.lang, lang);
        assert!(!page.unavailable.is_empty());
        for kind in [
            "ops.playerdata_binding_conflict",
            "incident.created",
            "incident.assigned",
            "request",
            "dry_run",
            "transition",
            "preview",
            "approved",
            "approval_used",
        ] {
            assert!(!page.event_label(kind).is_empty());
        }
        for status in [
            "delivered",
            "accepted",
            "in_progress",
            "succeeded",
            "failed",
            "rejected",
            "expired",
            "cancelled",
        ] {
            assert!(!page.status_label(status).is_empty());
        }
    }
    Ok(())
}
