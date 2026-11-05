//! Versioned legal acceptance recorded in the registration transaction.
use crate::accounts::AccountId;
use sqlx::PgConnection;
use std::net::IpAddr;
#[derive(Debug, Clone)]
pub struct ConsentConfig {
    pub terms_version: String,
    pub privacy_version: String,
}
impl Default for ConsentConfig {
    fn default() -> Self {
        Self {
            terms_version: "2026-11-05".into(),
            privacy_version: "2026-11-05".into(),
        }
    }
}
impl ConsentConfig {
    pub fn validate(&self) -> bool {
        [&self.terms_version, &self.privacy_version]
            .into_iter()
            .all(|v| !v.trim().is_empty() && v.chars().count() <= 128)
    }
}
pub async fn record(
    conn: &mut PgConnection,
    id: AccountId,
    config: &ConsentConfig,
    ip: Option<IpAddr>,
) -> Result<(), sqlx::Error> {
    for (kind, version) in [
        ("terms", &config.terms_version),
        ("privacy", &config.privacy_version),
    ] {
        sqlx::query("INSERT INTO account_consents (account_id,document_kind,document_version,ip) VALUES ($1,$2,$3,$4::text::inet)")
            .bind(id).bind(kind).bind(version).bind(ip.map(|v|v.to_string())).execute(&mut *conn).await?;
    }
    Ok(())
}
