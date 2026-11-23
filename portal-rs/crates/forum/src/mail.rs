use crate::view;
use sqlx::PgConnection;
use time::OffsetDateTime;
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct Origins {
    public: Option<String>,
    staff: Option<String>,
}

fn https_origin(value: &str) -> Option<String> {
    let url = Url::parse(value).ok()?;
    if url.scheme() == "https"
        && url.host().is_some()
        && url.username().is_empty()
        && url.password().is_none()
        && url.path() == "/"
        && url.query().is_none()
        && url.fragment().is_none()
    {
        Some(url.origin().ascii_serialization())
    } else {
        None
    }
}

impl Origins {
    pub fn configured(public: Option<&str>, admin_host: Option<&str>) -> Self {
        Self {
            public: public.and_then(https_origin),
            staff: admin_host.and_then(|host| https_origin(&format!("https://{host}"))),
        }
    }

    fn origin(&self, visibility: &str) -> Option<&str> {
        if visibility == "staff" {
            self.staff.as_deref()
        } else {
            self.public.as_deref()
        }
    }

    pub fn available(&self, visibility: &str) -> bool {
        self.origin(visibility).is_some()
    }

    pub async fn queue(
        &self,
        db: &mut PgConnection,
        thread: Uuid,
        author: Uuid,
        posted_at: OffsetDateTime,
        visibility: &str,
    ) -> Result<(), sqlx::Error> {
        let Some(origin) = self.origin(visibility) else {
            return Ok(());
        };
        let links = |lang: &str| {
            if visibility == "staff" {
                (
                    format!("{origin}/admin/forum/threads/{thread}?lang={lang}"),
                    format!("{origin}/admin/forum/preferences?lang={lang}"),
                )
            } else {
                (
                    format!("{origin}/{lang}/forum/threads/{thread}"),
                    format!("{origin}/{lang}/forum/preferences"),
                )
            }
        };
        let (pl_thread, pl_prefs) = links("pl");
        let (en_thread, en_prefs) = links("en");
        let pl = view::email("pl", &pl_thread, &pl_prefs)?;
        let en = view::email("en", &en_thread, &en_prefs)?;

        // One transactional, set-based enqueue per enabled subscriber per thread and minute.
        sqlx::query(include_str!("queue_mail.sql"))
            .bind(thread)
            .bind(author)
            .bind(posted_at)
            .bind(visibility)
            .bind(pl.subject)
            .bind(pl.text)
            .bind(pl.html)
            .bind(en.subject)
            .bind(en.text)
            .bind(en.html)
            .execute(db)
            .await?;
        Ok(())
    }
}
