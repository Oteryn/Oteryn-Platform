use super::*;
use aes::cipher::{BlockDecryptMut, KeyIvInit, block_padding::Pkcs7};
use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use hmac::{Hmac, Mac};
use sha2::Sha256;

pub(super) const TABLES: &[&str] = &[
    "identities",
    "identity_recovery_keys",
    "admin_roles",
    "admin_permissions",
    "admin_role_permissions",
    "identity_admin_roles",
];

pub(super) fn map(ctx: &Context, table: &str, row: &Record) -> Result<Vec<Insert>> {
    let mut records = Vec::new();
    match table {
        "identities" => {
            let id = mapped_id(ctx,table,row)?;
            let email=oteryn_identity::accounts::normalize_email(&text(row,"email")?);
            if !(oteryn_identity::login::Credentials { email:email.clone(), password:"import validation".into() }).valid() { return Err("invalid email address"); }
            let password = text(row,"password")?;
            if !password.starts_with("$argon2id$") && !password.starts_with("$2y$") && !password.starts_with("$2b$") { return Err("unsupported password hash"); }
            if password.starts_with("$2") { return Err("bcrypt hashes require destination login support before cutover"); }
            let status = if row.get("terminated_at").is_some_and(|v| !v.is_null()) { "closed" }
                else if row.get("disabled_at").is_some_and(|v| !v.is_null()) { "suspended" } else { "active" };
            let mut record = standard(ctx,table,row,"accounts",&["email","native_security_generation"])?;
            record.values.extend(json!({"password_hash":password,"email_verified_at":row.get("email_verified_at").filter(|v| !v.is_null()).cloned().unwrap_or(json!(text(row,"created_at")?)),"locale":row.get("locale").cloned().unwrap_or(json!("en")),"status":status,
                "public_account_association":flag(row,"public_account_association",false)?,"public_status_visible":flag(row,"public_status_visible",false)?,
                "login_mfa_required":row.get("two_factor_confirmed_at").is_some_and(|v| !v.is_null())}).as_object().cloned().ok_or("invalid account")?);
            record.values.insert("email".into(),json!(email));
            records.push(record);
            if let Some(Value::String(secret)) = row.get("two_factor_secret") {
                let plain = decrypt(secret)?;
                let secret = totp_rs::Secret::Encoded(String::from_utf8(plain).map_err(|_| "invalid TOTP secret")?).to_bytes().map_err(|_| "invalid TOTP secret")?;
                let account = Uuid::parse_str(id.as_str().ok_or("invalid account identifier")?).map_err(|_| "invalid account identifier")?;
                let raw = std::env::var("OTERYN_MFA_KEY").map_err(|_| "destination MFA key missing")?;
                let encrypted=encrypt_secret(account,&secret,&raw)?;
                records.push(insert("account_mfa_totp",json!({"account_id":id,"secret":bytes(&encrypted),"confirmed":row.get("two_factor_confirmed_at").is_some_and(|v| !v.is_null()),"last_step":row.get("two_factor_last_used_timestep").filter(|v| !v.is_null()).cloned().unwrap_or(json!(-1))}))?);
                if let Some(Value::String(codes)) = row.get("two_factor_recovery_codes") {
                    let hashes: Vec<String> = serde_json::from_slice(&decrypt(codes)?).map_err(|_| "invalid recovery codes")?;
                    for hash in hashes {
                        let hash = hex(&hash).map_err(|_| "legacy recovery-code hashes require compatible destination storage")?;
                        if hash.len() != 32 { return Err("invalid recovery code hash"); }
                        records.push(insert("account_recovery_codes",json!({"account_id":id,"code_hash":bytes(&hash)}))?);
                    }
                }
            } else if row.get("two_factor_confirmed_at").is_some_and(|v| !v.is_null()) || row.get("two_factor_recovery_codes").is_some_and(|v| !v.is_null()) { return Err("MFA secret missing for retained security state"); }
            if let Some(scheduled) = row.get("termination_scheduled_for").filter(|v| !v.is_null()) {
                records.push(insert("account_terminations",json!({"id":Uuid::now_v7().to_string(),"account_id":id,"scheduled_for":scheduled,"created_at":row.get("termination_requested_at").ok_or("termination request time missing")?,"finalized_at":row.get("terminated_at")}))?);
            }
            if row.get("email_change_available_at").is_some_and(|v| !v.is_null()) { return Err("email change cooldown has no compatible target"); }
        }
        "identity_recovery_keys" => return Err("recovery keys have no destination table; identity area must remain unapplied"),
        "admin_roles" => { if !ctx.reused_roles.contains(&text(row,"id")?) { records.push(standard(ctx,table,row,"roles",&["key"])?); } },
        "admin_permissions" => { text(row,"key")?; }
        "admin_role_permissions" => records.push(insert("role_permissions",json!({"role_id":ctx.reference(row,"role_id","admin_roles")?,"permission":ctx.permissions.get(&text(row,"permission_id")?).ok_or("unknown permission")?}))?),
        "identity_admin_roles" => records.push(insert("account_roles",json!({"account_id":ctx.reference(row,"identity_id","identities")?,"role_id":ctx.reference(row,"role_id","admin_roles")?}))?),
        _ => return Err("unsupported identity table"),
    }
    Ok(records)
}

// Laravel's default AES-256-CBC envelope authenticates the encoded IV and value.
// Neither the transport key nor the decrypted credentials are persisted in the import map.
fn decrypt(encoded: &str) -> Result<Vec<u8>> {
    let raw =
        std::env::var("PORTAL_IMPORT_LARAVEL_KEY").map_err(|_| "source encryption key missing")?;
    decrypt_with_key(encoded, &raw)
}
pub(super) fn decrypt_with_key(encoded: &str, raw: &str) -> Result<Vec<u8>> {
    let key = if let Some(raw) = raw.strip_prefix("base64:") {
        STANDARD
            .decode(raw)
            .map_err(|_| "invalid source encryption key")?
    } else {
        raw.as_bytes().to_vec()
    };
    if key.len() != 32 {
        return Err("invalid source encryption key");
    }
    let envelope: Record = serde_json::from_slice(
        &STANDARD
            .decode(encoded)
            .map_err(|_| "invalid encrypted credential")?,
    )
    .map_err(|_| "invalid encrypted credential")?;
    let iv = text(&envelope, "iv")?;
    let value = text(&envelope, "value")?;
    let mut mac =
        <Hmac<Sha256> as Mac>::new_from_slice(&key).map_err(|_| "invalid source encryption key")?;
    mac.update(iv.as_bytes());
    mac.update(value.as_bytes());
    mac.verify_slice(&hex(&text(&envelope, "mac")?)?)
        .map_err(|_| "credential authentication failed")?;
    let iv = STANDARD.decode(iv).map_err(|_| "invalid credential IV")?;
    let value = STANDARD
        .decode(value)
        .map_err(|_| "invalid encrypted credential")?;
    cbc::Decryptor::<aes::Aes256>::new_from_slices(&key, &iv)
        .map_err(|_| "invalid credential IV")?
        .decrypt_padded_vec_mut::<Pkcs7>(&value)
        .map_err(|_| "credential decryption failed")
}

pub(super) fn encrypt_secret(account: Uuid, secret: &[u8], raw: &str) -> Result<Vec<u8>> {
    let key = hex(raw)?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| "invalid destination MFA key")?;
    let mut nonce = [0; 12];
    getrandom::fill(&mut nonce).map_err(|_| "randomness unavailable")?;
    let encrypted = cipher
        .encrypt(
            &Nonce::from(nonce),
            Payload {
                msg: secret,
                aad: account.as_bytes(),
            },
        )
        .map_err(|_| "TOTP encryption failed")?;
    Ok([nonce.to_vec(), encrypted].concat())
}
