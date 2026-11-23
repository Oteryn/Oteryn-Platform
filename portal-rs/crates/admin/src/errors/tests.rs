use super::*;
use crate::rbac::tests::Database;
use std::sync::Arc;

#[test]
fn filters_accept_status_and_validate_route_and_cursor() -> Result<(), Box<dyn std::error::Error>> {
    let filters: Filters = serde_urlencoded::from_str("status=all&route=%2Fatlas")?;
    assert!(filters.status.and_then(ReviewStatus::filter).is_none());
    assert_eq!(filters.route, "/atlas");
    filters.validate()?;
    assert!(serde_urlencoded::from_str::<Filters>("status=unknown").is_err());
    let invalid: Filters = serde_urlencoded::from_str("before=invalid")?;
    assert!(matches!(invalid.cursor(), Err(Error::Invalid)));
    let invalid: Filters = serde_urlencoded::from_str("route=%0A")?;
    assert!(matches!(invalid.validate(), Err(Error::Invalid)));
    Ok(())
}

#[tokio::test]
async fn review_checks_permissions_and_observed_count_and_records_audit()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Database::open().await else {
        return Ok(());
    };
    let pool = &db.pool;
    let actor: Uuid = sqlx::query_scalar(
        "INSERT INTO accounts(email,password_hash,status,locale,email_verified_at)
         VALUES('observability@example.invalid','fixture','active','en',now()) RETURNING id",
    )
    .fetch_one(pool)
    .await?;
    let action = |role: &'static str| {
        sqlx::query(
            "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key=$2",
        )
        .bind(actor)
        .bind(role)
        .execute(pool)
    };
    action("support").await?;
    let ctx = CheckContext {
        account: actor,
        require_staff_mfa: true,
        mfa_verified: true,
        session_proof: Some(crate::test_staff::proof(pool, actor).await?),
    };
    let auth = MutationAuthorization::new(Arc::new(Authorizer::new(
        pool.clone(),
        web::PortalIdentity::configured_from(pool.clone(), &|_| None),
    )));
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO portal_errors(route,error_kind,last_request_id)
         VALUES('/fixture','http_500',uuidv7()) RETURNING id",
    )
    .fetch_one(pool)
    .await?;
    assert!(matches!(
        review_group(pool, id, ctx, &auth, None, 1, Action::Resolve).await,
        Err(Error::Authorization(rbac::Error::Denied))
    ));
    action("platform_admin").await?;
    assert!(matches!(
        review_group(pool, id, ctx, &auth, None, 0, Action::Resolve).await,
        Err(Error::Invalid)
    ));
    assert!(matches!(
        review_group(pool, id, ctx, &auth, None, 2, Action::Resolve).await,
        Err(Error::Conflict)
    ));
    let request = RequestId(Uuid::now_v7());
    let review = review_group(pool, id, ctx, &auth, Some(request), 1, Action::Resolve)
        .await?
        .into_response();
    assert_eq!(review.status(), StatusCode::SEE_OTHER);
    assert!(matches!(
        review_group(pool, id, ctx, &auth, None, 1, Action::Ignore).await,
        Err(Error::Conflict)
    ));
    let audit: (String, Uuid, Uuid, String, String) = sqlx::query_as(
        "SELECT action,actor,request_id,before->>'status',after->>'status'
         FROM audit_log WHERE target = $1",
    )
    .bind(id.to_string())
    .fetch_one(pool)
    .await?;
    assert_eq!(
        audit,
        (
            "admin.errors.resolve".into(),
            actor,
            request.0,
            "open".into(),
            "resolved".into()
        )
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM audit_log WHERE target=$1")
            .bind(id.to_string())
            .fetch_one(pool)
            .await?,
        1
    );
    db.close().await;
    Ok(())
}
