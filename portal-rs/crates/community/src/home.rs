//! Live home-page blocks composed from this crate's read models and CMS news.
//! Each block degrades to "unavailable" on its own; nothing is invented.
use oteryn_web::{HomeFeed, Language};
use sqlx::PgPool;
use std::sync::Arc;

/// Feed for [`oteryn_web::HomeFeed::layer`]: status, three news entries and
/// the top five visible characters.
pub fn feed(db: PgPool) -> HomeFeed {
    let privacy = Arc::new(crate::privacy::Authority::configured(db.clone()));
    HomeFeed(Arc::new(move |lang: Language| {
        let db = db.clone();
        let privacy = privacy.clone();
        Box::pin(async move { load(&db, privacy.as_ref(), lang).await })
    }))
}

async fn load(
    db: &PgPool,
    privacy: &dyn crate::rankings::PublicVisibility,
    lang: Language,
) -> oteryn_ui::HomeData {
    let on = |key| std::env::var(key).as_deref() == Ok("true");
    let (status, news, ranking) = tokio::join!(
        crate::worlds::home_status(db),
        oteryn_content::news::headlines(db, lang, 3),
        async {
            if on("PORTAL_PUBLIC_RANKINGS_ENABLED") {
                crate::rankings::home_top(
                    db,
                    privacy,
                    lang,
                    5,
                    on("PORTAL_PUBLIC_PROFILES_ENABLED"),
                )
                .await
                .ok()
            } else {
                None
            }
        },
    );
    let (ranking, ranking_as_of) =
        ranking.map_or((None, None), |(rows, as_of)| (Some(rows), Some(as_of)));
    oteryn_ui::HomeData {
        status,
        news: news.ok(),
        ranking,
        ranking_as_of,
    }
}
