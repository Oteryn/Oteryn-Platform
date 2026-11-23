use super::*;

pub(super) const LIMIT: usize = 50;

#[derive(Debug, Clone, Copy, Default, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SearchField {
    #[default]
    Auto,
    Email,
    AccountId,
    Character,
}

impl SearchField {
    pub(super) const fn key(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Email => "email",
            Self::AccountId => "account-id",
            Self::Character => "character",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MatchMode {
    #[default]
    Exact,
    Prefix,
}

pub enum SearchQuery<'a> {
    Email(&'a str, MatchMode),
    AccountId(Uuid),
    IdPrefix(String),
    Character(&'a str, MatchMode),
}

impl std::fmt::Debug for SearchQuery<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SearchQuery")
            .finish_non_exhaustive()
    }
}

impl<'a> SearchQuery<'a> {
    pub(super) fn parse(raw: &'a str, field: SearchField, mode: MatchMode) -> Result<Self, Error> {
        let term = raw.trim();
        if term.is_empty() || term.len() > 254 || raw.chars().any(char::is_control) {
            return Err(Error::BadInput);
        }
        if matches!(mode, MatchMode::Prefix) && term.chars().count() < 3 {
            return Err(Error::BadInput);
        }

        match (field, mode) {
            (SearchField::Auto, MatchMode::Exact) => {
                let field = if term.contains('@') {
                    SearchField::Email
                } else {
                    SearchField::AccountId
                };
                Self::parse(term, field, mode)
            }
            (SearchField::Email, MatchMode::Exact) if oteryn_identity::reset::valid_email(term) => {
                Ok(Self::Email(term, mode))
            }
            (SearchField::Email, MatchMode::Prefix)
                if term.is_ascii() && !term.bytes().any(|c| c.is_ascii_whitespace()) =>
            {
                Ok(Self::Email(term, mode))
            }
            (SearchField::AccountId, MatchMode::Exact) => {
                let id: Uuid = term.parse().map_err(|_| Error::BadInput)?;
                valid_id(id)?;
                if !id.to_string().eq_ignore_ascii_case(term) {
                    return Err(Error::BadInput);
                }
                Ok(Self::AccountId(id))
            }
            (SearchField::AccountId, MatchMode::Prefix) => {
                if term.len() > 36
                    || !term.bytes().enumerate().all(|(index, byte)| {
                        if matches!(index, 8 | 13 | 18 | 23) {
                            byte == b'-'
                        } else {
                            byte.is_ascii_hexdigit()
                        }
                    })
                {
                    return Err(Error::BadInput);
                }
                Ok(Self::IdPrefix(term.to_ascii_lowercase()))
            }
            (SearchField::Character, _) if term.len() <= 64 => Ok(Self::Character(term, mode)),
            _ => Err(Error::BadInput),
        }
    }
}

impl IdentityAccounts {
    pub(super) async fn search_target(
        &self,
        query: SearchQuery<'_>,
    ) -> Result<Vec<Account>, Error> {
        let pool = &self.accounts.pool;
        let rows = match query {
            SearchQuery::Email(email, MatchMode::Exact) => self
                .accounts
                .find_by_email(email)
                .await?
                .into_iter()
                .collect(),
            SearchQuery::AccountId(id) => self.accounts.find_by_id(id).await?.into_iter().collect(),
            SearchQuery::Email(email, MatchMode::Prefix) => {
                let prefix = oteryn_identity::accounts::normalize_email(email)
                    .replace('!', "!!")
                    .replace('%', "!%")
                    .replace('_', "!_");
                sqlx::query_as("SELECT * FROM accounts WHERE status<>'closed' AND lower(email) LIKE $1 ESCAPE '!' ORDER BY id LIMIT 51")
                    .bind(format!("{prefix}%"))
                    .fetch_all(pool)
                    .await?
            }
            SearchQuery::IdPrefix(prefix) => {
                sqlx::query_as("SELECT * FROM accounts WHERE id::text LIKE $1 ORDER BY id LIMIT 51")
                    .bind(format!("{prefix}%"))
                    .fetch_all(pool)
                    .await?
            }
            SearchQuery::Character(name, mode) => {
                let projection = self.projection.as_ref().ok_or(Error::Unavailable)?;
                let ids = projection::search(projection, name, mode).await?;
                sqlx::query_as("SELECT * FROM accounts WHERE id=ANY($1) AND status='active' AND email_verified_at IS NOT NULL ORDER BY id")
                    .bind(ids)
                    .fetch_all(pool)
                    .await?
            }
        };
        rows.into_iter().map(Self::safe).collect()
    }
}
