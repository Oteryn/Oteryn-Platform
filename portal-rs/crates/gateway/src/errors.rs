use crate::wire::{check_json, exact};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    RequestMalformed,
    UnsupportedVersion,
    OfferUnsupported,
    AuthenticationRequired,
    AttemptConflict,
    CharacterConflict,
    RouteUnavailable,
    RateLimited,
    ReconciliationRequired,
    GrantExpired,
    Unavailable,
}
impl Code {
    pub const ALL: [Self; 11] = [
        Self::RequestMalformed,
        Self::UnsupportedVersion,
        Self::OfferUnsupported,
        Self::AuthenticationRequired,
        Self::AttemptConflict,
        Self::CharacterConflict,
        Self::RouteUnavailable,
        Self::RateLimited,
        Self::ReconciliationRequired,
        Self::GrantExpired,
        Self::Unavailable,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::RequestMalformed => "NATIVE_LOGIN_REQUEST_MALFORMED",
            Self::UnsupportedVersion => "NATIVE_LOGIN_UNSUPPORTED_VERSION",
            Self::OfferUnsupported => "NATIVE_LOGIN_OFFER_UNSUPPORTED",
            Self::AuthenticationRequired => "NATIVE_LOGIN_AUTHENTICATION_REQUIRED",
            Self::AttemptConflict => "NATIVE_LOGIN_ATTEMPT_CONFLICT",
            Self::CharacterConflict => "NATIVE_LOGIN_CHARACTER_CONFLICT",
            Self::RouteUnavailable => "NATIVE_LOGIN_ROUTE_UNAVAILABLE",
            Self::RateLimited => "NATIVE_LOGIN_RATE_LIMITED",
            Self::ReconciliationRequired => "ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED",
            Self::GrantExpired => "NATIVE_LOGIN_GRANT_EXPIRED",
            Self::Unavailable => "NATIVE_LOGIN_UNAVAILABLE",
        }
    }
    pub fn status(self) -> u16 {
        match self {
            Self::RequestMalformed | Self::UnsupportedVersion => 400,
            Self::AuthenticationRequired => 401,
            Self::OfferUnsupported
            | Self::AttemptConflict
            | Self::CharacterConflict
            | Self::GrantExpired => 409,
            Self::RateLimited => 429,
            _ => 503,
        }
    }
    pub fn class(self) -> &'static str {
        match self {
            Self::RequestMalformed | Self::AttemptConflict | Self::GrantExpired => "RETRY_LOGIN",
            Self::UnsupportedVersion | Self::OfferUnsupported => "CLIENT_UPDATE_REQUIRED",
            Self::AuthenticationRequired => "AUTHENTICATION_REQUIRED",
            Self::CharacterConflict => "SESSION_UNAVAILABLE",
            _ => "TEMPORARILY_UNAVAILABLE",
        }
    }
    pub fn body(self, attempt_ref: Option<&str>) -> ErrorBody<'_> {
        ErrorBody {
            protocol_version: 2,
            error: ErrorDetail {
                code: self.name(),
                public_class: self.class(),
            },
            attempt_ref,
        }
    }
}
#[derive(Debug, Serialize)]
pub struct ErrorBody<'a> {
    protocol_version: u8,
    error: ErrorDetail,
    attempt_ref: Option<&'a str>,
}
#[derive(Debug, Serialize)]
struct ErrorDetail {
    code: &'static str,
    public_class: &'static str,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Refusal {
    pub code: Code,
    pub retry_after: u64,
}
impl From<Code> for Refusal {
    fn from(code: Code) -> Self {
        Self {
            code,
            retry_after: 0,
        }
    }
}
pub fn parse_issuer_error(status: u16, b: &[u8], attempt: &str) -> Option<Refusal> {
    if !check_json(b, 2) {
        return None;
    }
    let v: Value = serde_json::from_slice(b).ok()?;
    if !exact(
        &v,
        &["protocol_version", "error", "attempt_ref"],
        &["attempt_ref"],
    ) || v["protocol_version"].as_i64()? != 2
        || (!v["attempt_ref"].is_null() && v["attempt_ref"].as_str()? != attempt)
        || !exact(&v["error"], &["code", "public_class"], &[])
    {
        return None;
    }
    let name = v["error"]["code"].as_str()?;
    let name = match name {
        "NATIVE_LOGIN_TICKET_REJECTED" | "NATIVE_LOGIN_ACCOUNT_SECURITY_DENIED" => {
            Code::AuthenticationRequired.name()
        }
        _ => name,
    };
    let code = Code::ALL.into_iter().find(|c| c.name() == name)?;
    (code.status() == status && v["error"]["public_class"].as_str()? == code.class())
        .then(|| code.into())
}
pub fn retry_after(raw: &str) -> u64 {
    // strconv.Atoi permits a leading plus, but no whitespace.
    raw.strip_prefix('+')
        .unwrap_or(raw)
        .parse::<u64>()
        .ok()
        .filter(|s| (1..=3600).contains(s))
        .unwrap_or(60)
}
