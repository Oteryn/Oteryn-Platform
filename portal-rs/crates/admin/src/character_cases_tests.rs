use super::*;
use crate::web::PortalIdentity;
use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};
use std::sync::Arc;

#[test]
fn character_cases_are_reachable_from_the_staff_panel() {
    assert!(include_str!("../../../templates/admin/roles.html").contains("/admin/character-cases"));
}

fn submission(operation: Operation) -> Submission {
    let create = matches!(operation, Operation::Create);
    Submission {
        _csrf: String::new(),
        id: Uuid::now_v7(),
        ticket: Uuid::now_v7(),
        world: Uuid::now_v7(),
        reviewer: Uuid::now_v7(),
        operation,
        evidence: "Ticket message reference".into(),
        character: operation.has_character().then(Uuid::now_v7),
        expected_revision: operation.has_character().then_some(42),
        name: operation.has_name().then(|| "Aria Stone".into()),
        sex: create.then(|| "Female".into()),
        vocation: create.then(|| "Paladin".into()),
        schedule: matches!(operation, Operation::Restore).then(Uuid::now_v7),
        destination: matches!(operation, Operation::Repair).then(Uuid::now_v7),
    }
}

#[test]
fn the_five_profiles_require_their_exact_fields() {
    for operation in Operation::ALL {
        assert!(submission(operation).validate().is_ok());
    }
    let mut restore = submission(Operation::Restore);
    restore.schedule = None;
    assert!(matches!(restore.validate(), Err(Error::Invalid)));
    let mut repair = submission(Operation::Repair);
    repair.expected_revision = Some(0);
    assert!(matches!(repair.validate(), Err(Error::Invalid)));
    let mut rename = submission(Operation::Rename);
    rename.destination = Some(Uuid::now_v7());
    assert!(matches!(rename.validate(), Err(Error::Invalid)));
    let mut create = submission(Operation::Create);
    create.vocation = Some("paladin".into());
    assert!(matches!(create.validate(), Err(Error::Invalid)));
    let mut create = submission(Operation::Create);
    create.name = Some(" gm ".into());
    assert!(matches!(create.validate(), Err(Error::Invalid)));
    assert!(serde_json::from_value::<Operation>(json!("finalize")).is_err());
    assert!(bounded(&"a".repeat(161), 160).is_err());
}

#[test]
fn case_translations_match() {
    let locales = [
        include_str!("../../../templates/admin/en.ftl"),
        include_str!("../../../templates/admin/pl.ftl"),
    ];
    let names = |source: &str| -> Vec<String> {
        source
            .lines()
            .filter(|line| line.starts_with("character-case-"))
            .filter_map(|line| line.split_once(" = ").map(|(key, _)| key.to_owned()))
            .collect()
    };
    let keys = names(locales[0]);
    assert!(!keys.is_empty());
    assert_eq!(keys, names(locales[1]));
    for lang in ["en", "pl"] {
        for key in &keys {
            assert!(!crate::web::text(lang, key).is_empty(), "{lang}: {key}");
        }
    }
}

async fn account(pool: &PgPool, role: Option<&str>) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at)
         VALUES ($1,$2,'case-test','active','en',now())",
    )
    .bind(id)
    .bind(format!("{id}@example.invalid"))
    .execute(pool)
    .await?;
    if let Some(role) = role {
        sqlx::query(
            "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key=$2",
        )
        .bind(id)
        .bind(role)
        .execute(pool)
        .await?;
    }
    Ok(id)
}

async fn context(pool: &PgPool, actor: Uuid) -> Result<CheckContext, sqlx::Error> {
    Ok(CheckContext {
        account: actor,
        require_staff_mfa: true,
        mfa_verified: true,
        session_proof: Some(crate::test_staff::proof(pool, actor).await?),
    })
}

fn decision(revision: i64, decision: Decision) -> Review {
    Review {
        _csrf: String::new(),
        revision,
        decision,
        note: "Reviewed against the support ticket".into(),
    }
}

#[tokio::test]
async fn postgres_character_case_scope_review_and_audit() -> Result<(), Box<dyn std::error::Error>>
{
    let Some(_guard) = oteryn_db::test_support::global_lock_guard().await? else {
        return Ok(());
    };
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let root = PgPool::connect(&url).await?;
    let schema = format!("staff_cases_{}", Uuid::now_v7().simple());
    sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let search_path = format!("SET search_path TO {schema}");
    let pool = PgPoolOptions::new()
        .after_connect(move |connection, _| {
            let search_path = search_path.clone();
            Box::pin(async move {
                sqlx::query(AssertSqlSafe(search_path))
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    let requester = account(&pool, Some("support")).await?;
    let reviewer = account(&pool, Some("support")).await?;
    let outsider = account(&pool, Some("support")).await?;
    let owner = account(&pool, None).await?;
    let destination = account(&pool, None).await?;
    sqlx::query(
        "WITH r AS (INSERT INTO roles(key) VALUES ('case_review') RETURNING id),
         p AS (INSERT INTO role_permissions(role_id,permission) SELECT id,$2 FROM r)
         INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM r",
    )
    .bind(reviewer)
    .bind(REVIEW.as_str())
    .execute(&pool)
    .await?;
    let ticket: Uuid = sqlx::query_scalar(
        "INSERT INTO tickets(account_id,request_key,category,subject)
         VALUES ($1,uuidv7(),'character','Ownership review') RETURNING id",
    )
    .bind(owner)
    .fetch_one(&pool)
    .await?;
    let auth = Arc::new(Authorizer::new(
        pool.clone(),
        PortalIdentity::configured_from(pool.clone(), &|_| None),
    ));
    let mutation = MutationAuthorization::new(auth.clone());
    let author = context(&pool, requester).await?;
    let independent = context(&pool, reviewer).await?;
    let mut form = submission(Operation::Repair);
    form.ticket = ticket;
    form.reviewer = reviewer;
    form.destination = Some(destination);
    let id = form.id;
    assert_eq!(
        submit(&pool, &auth, &mutation, author, form.clone()).await?,
        id
    );
    assert_eq!(
        submit(&pool, &auth, &mutation, author, form.clone()).await?,
        id
    );
    form.expected_revision = Some(43);
    assert!(matches!(
        submit(&pool, &auth, &mutation, author, form.clone()).await,
        Err(Error::Conflict)
    ));
    assert_eq!(load(&pool, reviewer, id).await?.state(), "pending");
    assert!(matches!(
        load(&pool, outsider, id).await,
        Err(Error::Missing)
    ));
    assert!(matches!(
        decide(
            &pool,
            &auth,
            &mutation,
            author,
            id,
            decision(1, Decision::Accept)
        )
        .await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        decide(
            &pool,
            &auth,
            &mutation,
            independent,
            id,
            decision(2, Decision::Accept)
        )
        .await,
        Err(Error::Conflict)
    ));
    decide(
        &pool,
        &auth,
        &mutation,
        independent,
        id,
        decision(1, Decision::Return),
    )
    .await?;
    let returned = load(&pool, requester, id).await?;
    assert_eq!(returned.state(), "return");
    assert_eq!(returned.revision, 2);
    assert!(matches!(
        decide(
            &pool,
            &auth,
            &mutation,
            independent,
            id,
            decision(1, Decision::Accept)
        )
        .await,
        Err(Error::Conflict)
    ));

    form.id = Uuid::now_v7();
    let replacement = submit(&pool, &auth, &mutation, author, form).await?;
    decide(
        &pool,
        &auth,
        &mutation,
        independent,
        replacement,
        decision(1, Decision::Accept),
    )
    .await?;
    assert_eq!(load(&pool, requester, replacement).await?.state(), "accept");
    let actions: Vec<String> =
        sqlx::query_scalar("SELECT action FROM audit_log WHERE target=$1 ORDER BY id")
            .bind(id.to_string())
            .fetch_all(&pool)
            .await?;
    assert_eq!(
        actions,
        [
            "staff.character_case.submitted",
            "staff.character_case.reviewed"
        ]
    );
    pool.close().await;
    sqlx::query(AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    Ok(())
}
