use super::*;
use serde_json::json;

fn diff(lang: &str, action: &str, before: &Value, after: &Value) -> Vec<Change> {
    changes(lang, action, "terms-of-service", before, after)
}

#[tokio::test]
async fn postgres_audit_projects_only_fields_for_the_stored_action_and_target()
-> Result<(), Box<dyn std::error::Error>> {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let db = super::super::players::tests::Database::open(&std::env::var(
        "PORTAL_TEST_DATABASE_URL",
    )?)
    .await?;
    let request_id = Uuid::now_v7();
    for (action, target, before, after) in [
        (
            "cms.news_updated",
            "news",
            json!({"body":"x".repeat(100_000),"body_pl":"y".repeat(100_000),"status":"active","title":"old"}),
            json!({"body":"x".repeat(100_000),"body_pl":"y".repeat(100_000),"title_pl":"new","totp_secret":"excluded"}),
        ),
        (
            "cms.page.saved",
            "draft-client-case-4821",
            Value::Null,
            json!({"body":"excluded", "title":"draft","legal_version":"discarded","legal_effective_date":"2020-01-01"}),
        ),
        (
            "cms.page.saved",
            "privacy-policy",
            json!({"body":"excluded"}),
            json!({"legal_version":"v1","legal_effective_date":"2020-01-01","title":"Policy"}),
        ),
        (
            "rbac.role.granted",
            "account",
            json!({"assigned":false,"permissions":["excluded"]}),
            json!({"assigned":true,"role":"00000000-0000-7000-8000-000000000001"}),
        ),
    ] {
        let mut tx = db.pool.begin().await?;
        crate::audit::append(
            &mut tx,
            crate::audit::Entry {
                context: crate::audit::AuditContext {
                    actor: None,
                    request_id,
                },
                action,
                target,
                before,
                after,
            },
        )
        .await?;
        tx.commit().await?;
    }
    let rows = platform_rows(
        &db.pool,
        &Filter {
            request_id: Some(request_id),
            ..Default::default()
        },
    )
    .await?;
    assert_eq!(rows.len(), 4);
    assert_eq!(
        (&rows[3].before, &rows[3].after),
        (&json!({"title":"old"}), &json!({"title_pl":"new"}))
    );
    assert_eq!(
        (&rows[2].before, &rows[2].after),
        (&Value::Null, &json!({"title":"draft"}))
    );
    assert_eq!(
        (&rows[1].before, &rows[1].after),
        (
            &json!({}),
            &json!({"legal_version":"v1","legal_effective_date":"2020-01-01","title":"Policy"})
        )
    );
    assert_eq!(
        (&rows[0].before, &rows[0].after),
        (
            &json!({"assigned":false}),
            &json!({"assigned":true,"role":"00000000-0000-7000-8000-000000000001"})
        )
    );
    db.close().await?;
    Ok(())
}

#[tokio::test]
async fn postgres_audit_normalizes_timestamp_offsets_and_fractional_seconds()
-> Result<(), Box<dyn std::error::Error>> {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let db = super::super::players::tests::Database::open(&std::env::var(
        "PORTAL_TEST_DATABASE_URL",
    )?)
    .await?;
    let request_id = Uuid::now_v7();
    for (legacy, canonical) in [
        ("2020-01-01 00:00:00+0000", "2020-01-01T00:00:00Z"),
        ("2020-01-01 00:00:00-0230", "2020-01-01T02:30:00Z"),
        ("2020-01-01 00:00:00+01:30:59", "2019-12-31T22:29:01Z"),
        (
            "2020-01-01 00:00:00.1234565Z",
            "2020-01-01T00:00:00.123456Z",
        ),
        ("2020-01-01 00:00:00.0002495Z", "2020-01-01T00:00:00.00025Z"),
        ("2020-01-01 23:59:59.9999995Z", "2020-01-02T00:00:00Z"),
    ] {
        let mut tx = db.pool.begin().await?;
        crate::audit::append(
            &mut tx,
            crate::audit::Entry {
                context: crate::audit::AuditContext {
                    actor: None,
                    request_id,
                },
                action: "cms.news_updated",
                target: legacy,
                before: json!({"published_at":canonical,"translation_published_at":canonical}),
                after: json!({"published_at":legacy,"translation_published_at":legacy}),
            },
        )
        .await?;
        tx.commit().await?;
    }
    let mut tx = db.pool.begin().await?;
    crate::audit::append(&mut tx, crate::audit::Entry {
        context: crate::audit::AuditContext { actor: None, request_id },
        action: "cms.news_updated",
        target: "invalid",
        before: json!({"published_at":null,"translation_published_at":null}),
        after: json!({"published_at":"today","translation_published_at":"2020-02-30T00:00:00+0000"}),
    }).await?;
    tx.commit().await?;

    let rows = platform_rows(
        &db.pool,
        &Filter {
            request_id: Some(request_id),
            ..Default::default()
        },
    )
    .await?;
    assert_eq!(rows.len(), 7);
    for row in rows {
        let changes = changes("en", &row.action, &row.target, &row.before, &row.after);
        if row.target == "invalid" {
            assert_eq!(changes.len(), 2);
            assert!(
                changes
                    .iter()
                    .all(|change| change.before == "—" && change.after == "Unknown")
            );
        } else {
            assert_eq!(row.before, row.after, "{}", row.target);
            assert!(changes.is_empty(), "{}", row.target);
        }
    }
    db.close().await?;
    Ok(())
}

#[test]
fn diff_masks_email_and_omits_credentials_unknown_fields_and_unchanged_values() {
    let before = json!({"email":"alice@example.invalid", "title":"Private draft alice@example.invalid", "slug":"same"});
    let after = json!({
        "email":"amber@example.invalid",
        "title":"Private draft 192.0.2.42",
        "slug":"same",
        "password_hash":"placeholder",
        "access_token":"placeholder",
        "totp_secret":"placeholder",
        "recovery_codes":["placeholder"],
        "body":"free text"
    });
    assert_eq!(
        diff("en", "cms.news_updated", &before, &after),
        [
            Change {
                field: "Title".into(),
                before: "Value hidden".into(),
                after: "Changed (value hidden)".into()
            },
            Change {
                field: "Email".into(),
                before: "a•••@•••".into(),
                after: "a•••@•••".into()
            },
        ]
    );
}

#[test]
fn diff_refuses_wrong_action_and_nested_or_malformed_values() {
    assert!(diff("en", "unknown", &Value::Null, &json!({"title":"private"})).is_empty());
    assert!(
        diff(
            "en",
            "rbac.role.created",
            &Value::Null,
            &json!({
                "key":{"token":"placeholder"},
                "permissions":["admin.access", {"token":"placeholder"}],
                "email":"invalid"
            })
        )
        .is_empty()
    );
}

#[test]
fn diff_treats_legacy_page_deletion_forms_as_an_absent_after_value() {
    assert_eq!(
        diff(
            "en",
            "cms.page.deleted",
            &json!({"title":"Original"}),
            &json!({"title":"Submitted"})
        ),
        [Change {
            field: "Title".into(),
            before: "Value hidden".into(),
            after: "—".into()
        }]
    );
}

#[test]
fn equivalent_legacy_publication_timestamps_do_not_create_a_change() {
    let before = json!({"published_at":"2020-01-01T00:00:00+00:00", "translation_published_at":"2020-01-01 00:00:00+00"});
    let after = json!({"published_at":"2020-01-01T00:00:00Z", "translation_published_at":"2020-01-01T01:00:00+01:00"});
    assert!(diff("en", "cms.news_updated", &before, &after).is_empty());
    assert!(diff("en", "cms.page.saved", &before, &after).is_empty());
    assert_eq!(
        diff(
            "en",
            "cms.page.saved",
            &before,
            &json!({"published_at":"2020-01-02T01:00:00+01:00"})
        ),
        [Change {
            field: "Publication time".into(),
            before: "2020-01-01T00:00:00Z".into(),
            after: "2020-01-02T00:00:00Z".into(),
        }]
    );
}

#[test]
fn diff_hides_embedded_pii_in_legal_versions() {
    assert_eq!(
        diff(
            "en",
            "cms.page.saved",
            &json!({"legal_version":"v1 reviewer@example.invalid"}),
            &json!({"legal_version":"v2 2001:db8::42"}),
        ),
        [Change {
            field: "Legal version".into(),
            before: "Value hidden".into(),
            after: "Changed (value hidden)".into(),
        }]
    );
}

#[test]
fn diff_hides_editorial_slugs_and_custom_role_keys() {
    for (action, field, label) in [
        ("cms.news_updated", "slug", "URL slug"),
        ("rbac.role.created", "key", "Role key"),
    ] {
        assert_eq!(
            diff(
                "en",
                action,
                &Value::Null,
                &json!({(field):"private-case-42"})
            ),
            [Change {
                field: label.into(),
                before: "—".into(),
                after: "Changed (value hidden)".into(),
            }]
        );
    }
}

#[test]
fn diff_distinguishes_unparseable_historical_timestamps_from_absence() {
    for (before, after, previous, current) in [
        (Value::Null, json!("today"), "—", "Unknown"),
        (json!("today"), json!("tomorrow"), "Unknown", "Unknown"),
        (
            json!("today"),
            json!("2020-01-01T00:00:00Z"),
            "Unknown",
            "2020-01-01T00:00:00Z",
        ),
    ] {
        assert_eq!(
            diff(
                "en",
                "cms.news_updated",
                &json!({"published_at":before}),
                &json!({"published_at":after})
            ),
            [Change {
                field: "Publication time".into(),
                before: previous.into(),
                after: current.into()
            }]
        );
    }
}

#[test]
fn diff_ignores_legacy_legal_inputs_on_non_legal_pages() {
    let before = json!({"locale":"en","title":"Previous"});
    let after = json!({"locale":"en","title":"Updated","legal_version":"discarded","legal_effective_date":"2020-01-01"});
    let changes = changes("en", "cms.page.saved", "support", &before, &after);
    assert_eq!(
        changes,
        [Change {
            field: "Title".into(),
            before: "Value hidden".into(),
            after: "Changed (value hidden)".into()
        }]
    );
    let legal_changes = diff("en", "cms.page.saved", &before, &after);
    assert_eq!(legal_changes.len(), 3);
    assert_eq!(legal_changes[2].after, "2020-01-01");
}

#[test]
fn diff_localizes_known_workflow_values_in_both_languages() {
    for (lang, values) in [
        (
            "en",
            [
                "Draft",
                "Published",
                "Under review",
                "Accepted",
                "Warning",
                "Restriction",
            ],
        ),
        (
            "pl",
            [
                "Szkic",
                "Opublikowany",
                "W trakcie rozpatrywania",
                "Uwzględnione",
                "Ostrzeżenie",
                "Ograniczenie",
            ],
        ),
    ] {
        for ((field, value), label) in [
            ("state", "draft"),
            ("status", "published"),
            ("appeal_status", "reviewing"),
            ("appeal_status", "accepted"),
            ("category", "warning"),
            ("category", "restriction"),
        ]
        .into_iter()
        .zip(values)
        {
            assert_eq!(display(lang, field, &json!(value)).as_deref(), Some(label));
        }
    }
}

#[test]
fn a_legacy_sanction_edit_does_not_infer_an_appeal_status_change() {
    let before = json!({"status":"active"});
    let after = json!({"status":"revoked","appeal_status":"reviewing"});
    assert_eq!(
        diff("en", "support.enforcement_updated", &before, &after),
        [Change {
            field: "Status".into(),
            before: "Active".into(),
            after: "Revoked".into()
        }]
    );
}

#[test]
fn a_legacy_appeal_snapshot_marks_missing_prior_status_unknown() {
    assert_eq!(
        diff(
            "en",
            "support.enforcement_appeal_updated",
            &json!({"status":"active"}),
            &json!({"status":"active","appeal_status":"reviewing"})
        ),
        [Change {
            field: "Appeal status".into(),
            before: "Unknown".into(),
            after: "Under review".into()
        }]
    );
}

#[test]
fn diff_distinguishes_a_missing_value_from_an_empty_permission_set() {
    assert_eq!(
        diff(
            "pl",
            "rbac.role.created",
            &Value::Null,
            &json!({"permissions":[]})
        ),
        [Change {
            field: "Uprawnienia".into(),
            before: "—".into(),
            after: "[]".into()
        }]
    );
}
