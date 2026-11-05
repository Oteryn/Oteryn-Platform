//! Platform-local MFA. Callers must charge the public T04a policies before mutations.
//! SOURCE: app/Identity/Mfa/MfaCodeConsumer.php and PendingMfaLogin.php (behavior).
use crate::{AccountId, accounts::Accounts, login::Credentials, password};
use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
use subtle::ConstantTimeEq;
use totp_rs::{Algorithm, TOTP};
use uuid::Uuid;

type Tx<'a> = Transaction<'a, Postgres>;
/// Session adapters use this key with their typed server-side session API.
pub const SESSION_GRANT_KEY: &str = "mfa_grant";
/// An authorization snapshot, never a checked-out connection or an open transaction.
/// Revalidate after rotating the session to close concurrent credential-change races.
pub struct SessionAuthorization {
    pub account: AccountId,
    password_hash: String,
    grant: Option<String>,
}
impl SessionAuthorization {
    pub async fn current(&self, mfa: &Mfa) -> Result<bool, sqlx::Error> {
        if let Some(grant) = &self.grant {
            return mfa.satisfied(self.account, grant).await;
        }
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM accounts WHERE id=$1 AND password_hash=$2 AND status='active' AND email_verified_at IS NOT NULL AND login_mfa_required=false)")
            .bind(self.account).bind(&self.password_hash).fetch_one(&mfa.pool).await
    }
}

fn error() -> sqlx::Error {
    sqlx::Error::Protocol("MFA unavailable".into())
}
pub fn digest(value: &str) -> Vec<u8> {
    Sha256::digest(value.as_bytes()).to_vec()
}
pub fn random_token() -> Result<String, sqlx::Error> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(|_| error())?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
fn valid_token(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
/// Debug always redacts encryption keys and provisioning material.
#[derive(Clone)]
pub struct Mfa {
    pool: PgPool,
    key: [u8; 32],
}
pub struct Enrollment {
    pub secret: String,
}
pub struct Challenge {
    pub token: String,
    pub account: AccountId,
}
pub struct Authenticated {
    pub account: AccountId,
    pub grant: String,
    pub trusted: Option<String>,
}
#[derive(sqlx::FromRow)]
struct State {
    secret: Vec<u8>,
    confirmed: bool,
    last_step: i64,
    generation: Uuid,
}
impl Mfa {
    pub fn new(pool: PgPool, key: [u8; 32]) -> Self {
        Self { pool, key }
    }
    pub async fn authorize_session(
        &self,
        auth: &Authenticated,
    ) -> Result<Option<SessionAuthorization>, sqlx::Error> {
        let mut transaction = self.pool.begin().await?;
        let Some((account, _, password_hash)) = Self::valid(&mut transaction, &auth.grant, "grant")
            .await?
            .filter(|v| v.0 == auth.account)
        else {
            Self::refuse(transaction, Some(auth.account)).await?;
            return Ok(None);
        };
        transaction.commit().await?;
        Ok(Some(SessionAuthorization {
            account,
            password_hash,
            grant: Some(auth.grant.clone()),
        }))
    }

    pub async fn password_session(
        &self,
        input: Credentials,
    ) -> Result<Option<SessionAuthorization>, sqlx::Error> {
        let password = input.password.clone();
        let Some(id) = crate::login::authenticate(&self.pool, input).await? else {
            return Ok(None);
        };
        let mut transaction = self.pool.begin().await?;
        let current = Self::lock(&mut transaction, id).await?;
        let required: bool =
            sqlx::query_scalar("SELECT login_mfa_required FROM accounts WHERE id=$1")
                .bind(id)
                .fetch_one(&mut *transaction)
                .await?;
        if required
            || current.is_none()
            || !Self::password(&password, current.as_deref().unwrap_or("")).await?
        {
            Self::refuse(transaction, Some(id)).await?;
            return Ok(None);
        }
        Self::audit(&mut transaction, Some(id), "mfa.password_login").await?;
        let password_hash = current.ok_or_else(error)?;
        transaction.commit().await?;
        Ok(Some(SessionAuthorization {
            account: id,
            password_hash,
            grant: None,
        }))
    }
    pub async fn record_http_refusal(&self) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        Self::audit(&mut tx, None, "mfa.http_refused").await?;
        tx.commit().await
    }
    pub async fn enrolled(&self, id: AccountId) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM account_mfa_totp WHERE account_id=$1 AND confirmed)",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
    }
    /// Configuration uses exactly 64 hexadecimal characters; malformed/missing keys deny startup.
    pub fn configured(pool: PgPool) -> Result<Self, sqlx::Error> {
        let raw = std::env::var("OTERYN_MFA_KEY").map_err(|_| error())?;
        if !valid_token(&raw) {
            return Err(error());
        }
        let mut key = [0; 32];
        for (i, b) in key.iter_mut().enumerate() {
            *b = u8::from_str_radix(&raw[i * 2..i * 2 + 2], 16).map_err(|_| error())?;
        }
        Ok(Self::new(pool, key))
    }
    fn encrypt(&self, id: AccountId, plain: &[u8]) -> Result<Vec<u8>, sqlx::Error> {
        let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|_| error())?;
        let mut nonce = [0; 12];
        getrandom::fill(&mut nonce).map_err(|_| error())?;
        let body = cipher
            .encrypt(
                &Nonce::from(nonce),
                Payload {
                    msg: plain,
                    aad: id.as_bytes(),
                },
            )
            .map_err(|_| error())?;
        Ok([nonce.to_vec(), body].concat())
    }
    fn decrypt(&self, id: AccountId, data: &[u8]) -> Result<Vec<u8>, sqlx::Error> {
        if data.len() < 28 {
            return Err(error());
        }
        Aes256Gcm::new_from_slice(&self.key)
            .map_err(|_| error())?
            .decrypt(
                &Nonce::from(<[u8; 12]>::try_from(&data[..12]).map_err(|_| error())?),
                Payload {
                    msg: &data[12..],
                    aad: id.as_bytes(),
                },
            )
            .map_err(|_| error())
    }
    async fn lock(tx: &mut Tx<'_>, id: AccountId) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar("SELECT password_hash FROM accounts WHERE id=$1 AND status='active' AND email_verified_at IS NOT NULL FOR UPDATE").bind(id).fetch_optional(&mut **tx).await
    }
    async fn state(tx: &mut Tx<'_>, id: AccountId) -> Result<Option<State>, sqlx::Error> {
        sqlx::query_as("SELECT secret,confirmed,last_step,generation FROM account_mfa_totp WHERE account_id=$1").bind(id).fetch_optional(&mut **tx).await
    }
    async fn audit(
        tx: &mut Tx<'_>,
        id: Option<AccountId>,
        action: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO audit_log(actor,action,target,before,after,request_id) VALUES($1,$2,'mfa','{}','{}',$3)").bind(id).bind(action).bind(Uuid::now_v7()).execute(&mut **tx).await?;
        Ok(())
    }
    async fn refuse(tx: Tx<'_>, id: Option<AccountId>) -> Result<(), sqlx::Error> {
        let mut tx = tx;
        Self::audit(&mut tx, id, "mfa.refused").await?;
        tx.commit().await
    }
    async fn password(plain: &str, hash: &str) -> Result<bool, sqlx::Error> {
        if plain.len() > 1024 {
            return Ok(false);
        }
        let plain = plain.to_owned();
        let hash = hash.to_owned();
        tokio::task::spawn_blocking(move || password::verify(&plain, &hash))
            .await
            .map_err(|_| error())
    }
    /// Starts enrollment only for active verified accounts after current-password confirmation.
    pub async fn enroll(
        &self,
        id: AccountId,
        password: &str,
    ) -> Result<Option<Enrollment>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let hash = Self::lock(&mut tx, id).await?;
        let state = Self::state(&mut tx, id).await?;
        if hash.is_none()
            || state.is_some_and(|s| s.confirmed)
            || !Self::password(password, hash.as_deref().unwrap_or("")).await?
        {
            Self::refuse(tx, Some(id)).await?;
            return Ok(None);
        }
        let mut secret = vec![0; 20];
        getrandom::fill(&mut secret).map_err(|_| error())?;
        let encoded = totp_rs::Secret::Raw(secret.clone())
            .to_encoded()
            .to_string();
        let encrypted = self.encrypt(id, &secret)?;
        sqlx::query("INSERT INTO account_mfa_totp(account_id,secret) VALUES($1,$2) ON CONFLICT(account_id) DO UPDATE SET secret=$2,last_step=-1,generation=uuidv7(),updated_at=now()").bind(id).bind(encrypted).execute(&mut *tx).await?;
        Self::audit(&mut tx, Some(id), "mfa.enrollment_started").await?;
        tx.commit().await?;
        Ok(Some(Enrollment { secret: encoded }))
    }
    fn step(
        &self,
        id: AccountId,
        state: &State,
        code: &str,
        now: u64,
    ) -> Result<Option<i64>, sqlx::Error> {
        let code = code.trim();
        if code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()) {
            return Ok(None);
        }
        let totp = TOTP::new(Algorithm::SHA1, 6, 1, 30, self.decrypt(id, &state.secret)?)
            .map_err(|_| error())?;
        let current = now / 30;
        for step in [
            current.saturating_sub(1),
            current,
            current.saturating_add(1),
        ] {
            let step_i = i64::try_from(step).map_err(|_| error())?;
            if step_i > state.last_step
                && bool::from(totp.generate(step * 30).as_bytes().ct_eq(code.as_bytes()))
            {
                return Ok(Some(step_i));
            }
        }
        Ok(None)
    }
    pub async fn confirm(
        &self,
        id: AccountId,
        password: &str,
        code: &str,
    ) -> Result<Option<Vec<String>>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let hash = Self::lock(&mut tx, id).await?;
        let state = Self::state(&mut tx, id).await?;
        let step = match state.as_ref() {
            Some(s) if !s.confirmed => self.step(id, s, code, now()?)?,
            _ => None,
        };
        if hash.is_none()
            || step.is_none()
            || !Self::password(password, hash.as_deref().unwrap_or("")).await?
        {
            Self::refuse(tx, Some(id)).await?;
            return Ok(None);
        }
        sqlx::query("UPDATE account_mfa_totp SET confirmed=true,last_step=$2,updated_at=now() WHERE account_id=$1").bind(id).bind(step).execute(&mut *tx).await?;
        // The import/login guard remains the authoritative password-only denial.
        sqlx::query("UPDATE accounts SET login_mfa_required=true,updated_at=now() WHERE id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        let mut codes = Vec::new();
        sqlx::query("DELETE FROM account_recovery_codes WHERE account_id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        for _ in 0..8 {
            let code = random_token()?;
            sqlx::query("INSERT INTO account_recovery_codes(account_id,code_hash) VALUES($1,$2)")
                .bind(id)
                .bind(digest(&code))
                .execute(&mut *tx)
                .await?;
            codes.push(code);
        }
        Self::revoke(&mut tx, id).await?;
        Self::audit(&mut tx, Some(id), "mfa.enrolled").await?;
        tx.commit().await?;
        Ok(Some(codes))
    }
    async fn revoke(tx: &mut Tx<'_>, id: AccountId) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM account_mfa_tokens WHERE account_id=$1")
            .bind(id)
            .execute(&mut **tx)
            .await?;
        sqlx::query("DELETE FROM web_sessions WHERE account_id=$1")
            .bind(id)
            .execute(&mut **tx)
            .await?;
        sqlx::query("DELETE FROM native_admissions WHERE account_id=$1")
            .bind(id)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }
    async fn token(
        tx: &mut Tx<'_>,
        id: AccountId,
        state: &State,
        hash: &str,
        purpose: &str,
        parent: Option<&str>,
        seconds: i32,
    ) -> Result<String, sqlx::Error> {
        let token = random_token()?;
        sqlx::query("INSERT INTO account_mfa_tokens(token_hash,account_id,generation,credential_hash,purpose,parent_hash,expires_at) VALUES($1,$2,$3,$4,$5,$6,now()+make_interval(secs=>$7))").bind(digest(&token)).bind(id).bind(state.generation).bind(digest(hash)).bind(purpose).bind(parent.map(digest)).bind(f64::from(seconds)).execute(&mut **tx).await?;
        Ok(token)
    }
    /// Password validation never creates an authenticated session. Unknown accounts do equal-cost hashing.
    pub async fn begin_login(&self, input: &Credentials) -> Result<Option<Challenge>, sqlx::Error> {
        if !input.valid() {
            let tx = self.pool.begin().await?;
            Self::refuse(tx, None).await?;
            return Ok(None);
        }
        let account = Accounts {
            pool: self.pool.clone(),
        }
        .find_by_email(&input.email)
        .await?;
        let dummy = "$argon2id$v=19$m=19456,t=2,p=1$M0Iyek83amFsS1J0cWNnLg$5wDTmQ1PJJorXDWlZfJnbjZkEATuPz4DMxaPpZ9azV0";
        let hash = account
            .as_ref()
            .map_or(dummy, |a| a.password_hash.as_str())
            .to_owned();
        let verified = Self::password(&input.password, &hash).await?;
        let mut tx = self.pool.begin().await?;
        let Some(account) = account.filter(|_| verified) else {
            Self::refuse(tx, None).await?;
            return Ok(None);
        };
        let current = Self::lock(&mut tx, account.id).await?;
        let state = Self::state(&mut tx, account.id).await?;
        let Some(state) =
            state.filter(|s| s.confirmed && current.as_deref() == Some(hash.as_str()))
        else {
            Self::refuse(tx, None).await?;
            return Ok(None);
        };
        let token = Self::token(&mut tx, account.id, &state, &hash, "challenge", None, 300).await?;
        Self::audit(&mut tx, Some(account.id), "mfa.challenge_started").await?;
        tx.commit().await?;
        Ok(Some(Challenge {
            token,
            account: account.id,
        }))
    }
    async fn valid(
        tx: &mut Tx<'_>,
        token: &str,
        purpose: &str,
    ) -> Result<Option<(AccountId, State, String)>, sqlx::Error> {
        if !valid_token(token) {
            return Ok(None);
        }
        let id: Option<AccountId> = sqlx::query_scalar(
            "SELECT account_id FROM account_mfa_tokens WHERE token_hash=$1 AND purpose=$2",
        )
        .bind(digest(token))
        .bind(purpose)
        .fetch_optional(&mut **tx)
        .await?;
        let Some(id) = id else {
            return Ok(None);
        };
        let hash = Self::lock(tx, id).await?;
        let state = Self::state(tx, id).await?;
        let (Some(hash), Some(state)) = (hash, state) else {
            return Ok(None);
        };
        let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account_mfa_tokens WHERE token_hash=$1 AND purpose=$2 AND expires_at>now() AND generation=$3 AND credential_hash=$4)").bind(digest(token)).bind(purpose).bind(state.generation).bind(digest(&hash)).fetch_one(&mut **tx).await?;
        Ok((valid && state.confirmed).then_some((id, state, hash)))
    }
    pub async fn pending_account(&self, token: &str) -> Result<Option<AccountId>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        Ok(Self::valid(&mut tx, token, "challenge").await?.map(|v| v.0))
    }
    /// Returns plaintext only to the mail adapter, never to the browser or an audit payload.
    pub async fn email_code(
        &self,
        challenge: &str,
    ) -> Result<Option<(String, String)>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let Some((id, state, hash)) = Self::valid(&mut tx, challenge, "challenge").await? else {
            Self::refuse(tx, None).await?;
            return Ok(None);
        };
        sqlx::query("DELETE FROM account_mfa_tokens WHERE account_id=$1 AND purpose='email'")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        let token = Self::token(&mut tx, id, &state, &hash, "email", Some(challenge), 300).await?;
        let email: String = sqlx::query_scalar("SELECT email FROM accounts WHERE id=$1")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
        Self::audit(&mut tx, Some(id), "mfa.email_issued").await?;
        tx.commit().await?;
        Ok(Some((email, token)))
    }
    pub async fn complete(
        &self,
        challenge: &str,
        code: &str,
        remember: bool,
    ) -> Result<Option<Authenticated>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let Some((id, state, hash)) = Self::valid(&mut tx, challenge, "challenge").await? else {
            Self::refuse(tx, None).await?;
            return Ok(None);
        };
        let code = code.trim();
        let mut factor = "totp";
        let accepted = if let Some(step) = self.step(id, &state, code, now()?)? {
            sqlx::query(
                "UPDATE account_mfa_totp SET last_step=$2,updated_at=now() WHERE account_id=$1",
            )
            .bind(id)
            .bind(step)
            .execute(&mut *tx)
            .await?;
            true
        } else if valid_token(code) {
            let used = sqlx::query(
                "DELETE FROM account_recovery_codes WHERE account_id=$1 AND code_hash=$2",
            )
            .bind(id)
            .bind(digest(&code.to_ascii_lowercase()))
            .execute(&mut *tx)
            .await?
            .rows_affected();
            if used == 1 {
                factor = "recovery";
                true
            } else {
                factor = "email";
                sqlx::query("DELETE FROM account_mfa_tokens WHERE token_hash=$1 AND parent_hash=$2 AND purpose='email' AND account_id=$3 AND generation=$4 AND credential_hash=$5 AND expires_at>now()").bind(digest(code)).bind(digest(challenge)).bind(id).bind(state.generation).bind(digest(&hash)).execute(&mut *tx).await?.rows_affected()==1
            }
        } else {
            false
        };
        if !accepted {
            Self::refuse(tx, Some(id)).await?;
            return Ok(None);
        }
        sqlx::query("DELETE FROM account_mfa_tokens WHERE token_hash=$1 OR parent_hash=$1")
            .bind(digest(challenge))
            .execute(&mut *tx)
            .await?;
        let grant = Self::token(&mut tx, id, &state, &hash, "grant", None, 86400).await?;
        let trusted = if remember {
            Some(Self::token(&mut tx, id, &state, &hash, "trusted", None, 30 * 86400).await?)
        } else {
            None
        };
        Self::audit(&mut tx, Some(id), &format!("mfa.login.{factor}")).await?;
        tx.commit().await?;
        Ok(Some(Authenticated {
            account: id,
            grant,
            trusted,
        }))
    }
    /// Trusted devices still require the password and a live, single-use login challenge.
    pub async fn trust(
        &self,
        challenge: &str,
        device: &str,
    ) -> Result<Option<Authenticated>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let Some((id, state, hash)) = Self::valid(&mut tx, challenge, "challenge").await? else {
            Self::refuse(tx, None).await?;
            return Ok(None);
        };
        // Lock order remains account first; never acquire a second account lock for a foreign device.
        let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account_mfa_tokens WHERE token_hash=$1 AND account_id=$2 AND purpose='trusted' AND expires_at>now() AND generation=$3 AND credential_hash=$4)").bind(digest(device)).bind(id).bind(state.generation).bind(digest(&hash)).fetch_one(&mut *tx).await?;
        if !valid {
            Self::refuse(tx, Some(id)).await?;
            return Ok(None);
        }
        sqlx::query("DELETE FROM account_mfa_tokens WHERE token_hash=$1")
            .bind(digest(challenge))
            .execute(&mut *tx)
            .await?;
        let expires: sqlx::types::time::OffsetDateTime = sqlx::query_scalar(
            "DELETE FROM account_mfa_tokens WHERE token_hash=$1 RETURNING expires_at",
        )
        .bind(digest(device))
        .fetch_one(&mut *tx)
        .await?;
        let rotated = Self::token(&mut tx, id, &state, &hash, "trusted", None, 30 * 86400).await?;
        sqlx::query("UPDATE account_mfa_tokens SET expires_at=$2 WHERE token_hash=$1")
            .bind(digest(&rotated))
            .bind(expires)
            .execute(&mut *tx)
            .await?;
        let trusted = Some(rotated);
        let grant = Self::token(&mut tx, id, &state, &hash, "grant", None, 86400).await?;
        Self::audit(&mut tx, Some(id), "mfa.login.trusted").await?;
        tx.commit().await?;
        Ok(Some(Authenticated {
            account: id,
            grant,
            trusted,
        }))
    }
    /// Denies absent, revoked, expired, foreign and stale credential proofs.
    pub async fn satisfied(&self, id: AccountId, grant: &str) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        self.satisfied_in_transaction(&mut tx, id, grant).await
    }
    /// Read-only grant verification under the caller's transaction. The account
    /// lock is retained through domain/audit commit to serialize revocation.
    pub async fn satisfied_in_transaction(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        id: AccountId,
        grant: &str,
    ) -> Result<bool, sqlx::Error> {
        let valid = Self::valid(tx, grant, "grant")
            .await?
            .is_some_and(|v| v.0 == id);
        if !valid {
            return Ok(false);
        }
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account_mfa_tokens WHERE token_hash=$1 AND expires_at>clock_timestamp() AND created_at<=clock_timestamp())")
            .bind(digest(grant)).fetch_one(&mut **tx).await
    }
    /// Consumes a fresh TOTP counter for an active, verified, enrolled account.
    /// Recovery/email/device proofs are deliberately not accepted for staff elevation.
    pub async fn verify_totp(&self, id: AccountId, code: &str) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let active = Self::lock(&mut tx, id).await?.is_some();
        let state = Self::state(&mut tx, id).await?;
        let step = match state.as_ref() {
            Some(state) if active && state.confirmed => self.step(id, state, code, now()?)?,
            _ => None,
        };
        let Some(step) = step else {
            Self::refuse(tx, Some(id)).await?;
            return Ok(false);
        };
        sqlx::query(
            "UPDATE account_mfa_totp SET last_step=$2,updated_at=now() WHERE account_id=$1",
        )
        .bind(id)
        .bind(step)
        .execute(&mut *tx)
        .await?;
        Self::audit(&mut tx, Some(id), "mfa.step_up.totp").await?;
        tx.commit().await?;
        Ok(true)
    }
    /// Safe identifiers only; bearer material is never listed.
    pub async fn devices(&self, id: AccountId) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar("SELECT encode(token_hash,'hex') FROM account_mfa_tokens WHERE account_id=$1 AND purpose='trusted' AND expires_at>now() ORDER BY created_at").bind(id).fetch_all(&self.pool).await
    }
    pub async fn remove_device(
        &self,
        id: AccountId,
        identifier: &str,
    ) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let active = Self::lock(&mut tx, id).await?.is_some();
        let removed = if active && valid_token(identifier) {
            sqlx::query("DELETE FROM account_mfa_tokens WHERE account_id=$1 AND purpose='trusted' AND encode(token_hash,'hex')=$2").bind(id).bind(identifier).execute(&mut *tx).await?.rows_affected()==1
        } else {
            false
        };
        Self::audit(
            &mut tx,
            Some(id),
            if removed {
                "mfa.device_removed"
            } else {
                "mfa.refused"
            },
        )
        .await?;
        tx.commit().await?;
        Ok(removed)
    }
}
fn now() -> Result<u64, sqlx::Error> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|t| t.as_secs())
        .map_err(|_| error())
}

#[cfg(test)]
#[path = "mfa_tests.rs"]
mod tests;

macro_rules! redacted_debug {
    ($($type:ty),+) => {$ (
        impl std::fmt::Debug for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("Mfa([redacted])")
            }
        }
    )+};
}
redacted_debug!(
    Mfa,
    Enrollment,
    Challenge,
    Authenticated,
    SessionAuthorization
);

/// Bounded expiry cleanup, safe to schedule while account features are disabled.
#[derive(Debug, Clone)]
pub struct MfaCleanup {
    pub pool: PgPool,
}
impl oteryn_db::outbox::Cleanup for MfaCleanup {
    fn clean(&self) -> oteryn_db::outbox::CleanupFuture<'_> {
        Box::pin(async move {
            sqlx::query("DELETE FROM account_mfa_tokens WHERE token_hash IN (SELECT token_hash FROM account_mfa_tokens WHERE expires_at<=now() LIMIT 1000 FOR UPDATE SKIP LOCKED)")
                .execute(&self.pool).await.map_err(|e| e.to_string())?;
            Ok(())
        })
    }
}
