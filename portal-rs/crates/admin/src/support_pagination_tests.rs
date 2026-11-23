use super::*;
#[derive(Clone)]
struct Active;
impl AccountStateReader for Active {
    // Synthetic session authority for this fixture; production denies missing proofs.
    async fn authenticated_in(
        &self,
        connection: &mut sqlx::PgConnection,
        context: oteryn_admin::rbac::CheckContext,
    ) -> std::result::Result<bool, oteryn_admin::rbac::AccountStateUnavailable> {
        if let Some(proof) = context.session_proof {
            oteryn_admin::rbac::durable_session_in(connection, context.account, proof).await
        } else {
            Ok(!context.require_staff_mfa || context.mfa_verified)
        }
    }

    async fn state(
        &self,
        _: Uuid,
    ) -> Result<crate::rbac::AccountState, crate::rbac::AccountStateUnavailable> {
        Ok(crate::rbac::AccountState::Active)
    }
}
#[derive(Clone)]
struct Queue;
impl Backend for Queue {
    async fn list(&self, area: Area, page: u32) -> Result<Vec<Record>, Error> {
        Ok(if page == 10000 {
            (0..30)
                .map(|_| Record {
                    id: Uuid::now_v7(),
                    version: 1,
                    title: "Queue record".into(),
                    status: match area {
                        Area::Tickets => "open",
                        Area::Reports => "submitted",
                        Area::Enforcement => "active",
                    }
                    .into(),
                    appeal_status: None,
                    messages: vec![],
                    next_message: None,
                    forum_post: None,
                    enforcement: None,
                })
                .collect()
        } else {
            vec![]
        })
    }
    async fn load(&self, _: Area, _: Uuid, _: Option<Uuid>) -> Result<Record, Error> {
        Err(Error::NotFound)
    }
    async fn apply(
        &self,
        _: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        _: crate::rbac::CheckContext,
        _: Uuid,
        _: i64,
        _: &Command,
    ) -> Result<(serde_json::Value, serde_json::Value), Error> {
        Err(Error::Unavailable)
    }
}
#[tokio::test]
async fn pagination_admin_queue_caps_and_empty() -> Result<(), Box<dyn std::error::Error>> {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let pool = PgPool::connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")?;
    let state = Arc::new(Panel {
        pool: pool.clone(),
        auth: Authorizer::new(pool, Active),
        backend: Queue,
    });
    for area in [Area::Tickets, Area::Reports, Area::Enforcement] {
        for page in [1, 10000] {
            let response = list(
                State(state.clone()),
                Extension(area),
                Query(Paging { page: Some(page) }),
                HeaderMap::new(),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
            let bytes = axum::body::to_bytes(response.into_body(), 100000).await?;
            let html = std::str::from_utf8(&bytes)?;
            assert!(!html.contains("?page="));
        }
        let response = list(
            State(state.clone()),
            Extension(area),
            Query(Paging { page: Some(10001) }),
            HeaderMap::new(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
    Ok(())
}
