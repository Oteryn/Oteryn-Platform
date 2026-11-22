use oteryn_admin::{
    audit::{self, AuditContext, Entry},
    rbac::Error,
};
use oteryn_identity::consents::PLACEHOLDER_LEGAL_VERSION as VERSION;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

const OPERATOR: &str = include_str!("legal/operator.md");
const EFFECTIVE: &str = "2026-11-22";

type Edition = (&'static str, &'static str, &'static str);

const DOCUMENTS: [(&str, [Edition; 2]); 3] = [
    (
        "terms-of-service",
        [
            ("en", "Terms of Service", include_str!("legal/terms.en.md")),
            ("pl", "Regulamin Oteryn", include_str!("legal/terms.pl.md")),
        ],
    ),
    (
        "privacy-policy",
        [
            ("en", "Privacy Policy", include_str!("legal/privacy.en.md")),
            (
                "pl",
                "Polityka prywatności",
                include_str!("legal/privacy.pl.md"),
            ),
        ],
    ),
    (
        "cookie-policy",
        [
            ("en", "Cookie Policy", include_str!("legal/cookies.en.md")),
            (
                "pl",
                "Polityka plików cookie",
                include_str!("legal/cookies.pl.md"),
            ),
        ],
    ),
];

/// Install the default bilingual documents, returning the number of document pairs installed.
pub async fn install(pool: &PgPool) -> Result<usize, Error> {
    let mut tx = pool.begin().await?;
    let mut installed = 0;
    let context = AuditContext {
        actor: None,
        request_id: Uuid::now_v7(),
    };
    for (slug, editions) in DOCUMENTS {
        // Share the CMS lock so the eligibility check and staff saves cannot interleave.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,71102))")
            .bind(slug)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO pages(id,slug) VALUES($1,$2) ON CONFLICT(slug) DO NOTHING")
            .bind(Uuid::now_v7())
            .bind(slug)
            .execute(&mut *tx)
            .await?;
        let id: Uuid = sqlx::query_scalar("SELECT id FROM pages WHERE slug=$1 FOR UPDATE")
            .bind(slug)
            .fetch_one(&mut *tx)
            .await?;
        // Install a matched translation pair. Even a single staff draft or prior
        // publication owns the document and must not be paired with default text.
        let owned: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM editorial_pages WHERE page_id=$1)
                 OR EXISTS(SELECT 1 FROM page_legal_versions WHERE page_id=$1)",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        if owned {
            continue;
        }

        let mut source_revision = 1_i64;
        for (locale, title, markdown) in editions {
            let body = markdown.replace("{{operator}}", OPERATOR.trim_end());
            let revision = sqlx::query_scalar(
                "INSERT INTO editorial_pages
                    (page_id,locale,title,body,published_at,legal_version,legal_effective_date,source_revision)
                 VALUES ($1,$2,$3,$4,now(),$5,$6::text::date,$7)
                 RETURNING revision",
            )
            .bind(id)
            .bind(locale)
            .bind(title)
            .bind(&body)
            .bind(VERSION)
            .bind(EFFECTIVE)
            .bind(source_revision)
            .fetch_one(&mut *tx)
            .await?;
            source_revision = revision;
            sqlx::query(
                "INSERT INTO page_legal_versions (page_id,locale,version,content) VALUES ($1,$2,$3,$4)",
            )
            .bind(id)
            .bind(locale)
            .bind(VERSION)
            .bind(json!({"title": title, "body": body, "effective": EFFECTIVE}))
            .execute(&mut *tx)
            .await?;
        }
        audit::append(
            &mut tx,
            Entry {
                context,
                action: "cms.legal.defaults_installed",
                target: slug,
                before: json!(null),
                after: json!({"version": VERSION, "effective": EFFECTIVE, "locales": ["en", "pl"]}),
            },
        )
        .await?;
        installed += 1;
    }
    tx.commit().await?;
    Ok(installed)
}
