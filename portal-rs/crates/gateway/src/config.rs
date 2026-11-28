//! Gateway configuration is independent of portal and database configuration.
use oteryn_core::Environment;
use std::{fmt, net::IpAddr, path::PathBuf, time::Duration};

const SERVICE_TOKEN_FILE: &str = "OTERYN_PLATFORM_SERVICE_TOKEN_FILE";
pub(crate) const ISSUER_CA_FILE: &str = "OTERYN_PLATFORM_ISSUER_CA_FILE";
#[derive(Clone)]
pub struct Config {
    pub listen: String,
    pub trusted_proxies: Vec<IpAddr>,
    pub base_url: String,
    pub token_file: PathBuf,
    pub issuer_ca_file: Option<PathBuf>,
    pub enabled: bool,
    /// `GATEWAY_NATIVE_PROTOCOL3_ENABLED` (contract §19.1), default off and limited to testing/preprod.
    pub protocol3: bool,
    pub timeout: Duration,
    pub ready_timeout: Duration,
    pub version: String,
}
impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("GatewayConfig{redacted}")
    }
}
impl Config {
    pub fn from_env() -> Result<Self, String> {
        if std::env::var_os(ISSUER_CA_FILE).is_some_and(|value| value.to_str().is_none()) {
            return Err(format!("invalid {ISSUER_CA_FILE}"));
        }
        Self::from_lookup(|key| std::env::var(key).ok())
    }
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        let value = |key, fallback: &str| {
            get(key)
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| fallback.to_owned())
        };
        let base_url = value("OTERYN_PLATFORM_BASE_URL", "")
            .trim_end_matches('/')
            .to_owned();
        let issuer_ca_file = get(ISSUER_CA_FILE);
        if issuer_ca_file
            .as_ref()
            .is_some_and(|path| path.trim().is_empty())
        {
            return Err(format!("{ISSUER_CA_FILE} is empty"));
        }
        validate_issuer_url(&base_url, issuer_ca_file.is_some())?;
        let token_file = value(SERVICE_TOKEN_FILE, "");
        if token_file.is_empty() {
            return Err("OTERYN_PLATFORM_SERVICE_TOKEN_FILE is required".into());
        }
        let enabled = match value("GATEWAY_NATIVE_LOGIN_ENABLED", "false").as_str() {
            "false" => false,
            "true" => true,
            _ => return Err("invalid GATEWAY_NATIVE_LOGIN_ENABLED".into()),
        };
        let protocol3 = match value("GATEWAY_NATIVE_PROTOCOL3_ENABLED", "false").as_str() {
            "false" => false,
            "true" => get("OTERYN_ENV").is_some_and(|environment| {
                matches!(
                    Environment::parse(&environment),
                    Ok(Environment::Testing | Environment::Preprod)
                )
            }),
            _ => return Err("invalid GATEWAY_NATIVE_PROTOCOL3_ENABLED".into()),
        };
        let listen = value("GATEWAY_LISTEN_ADDR", "0.0.0.0:8080");
        if listen.trim().is_empty() {
            return Err("GATEWAY_LISTEN_ADDR is empty".into());
        }
        Ok(Self {
            trusted_proxies: value("GATEWAY_TRUSTED_PROXIES", "")
                .split(',')
                .filter(|v| !v.trim().is_empty())
                .map(|v| {
                    v.trim()
                        .parse()
                        .map_err(|_| "invalid GATEWAY_TRUSTED_PROXIES".to_owned())
                })
                .collect::<Result<Vec<IpAddr>, String>>()?,
            listen,
            base_url,
            token_file: token_file.into(),
            issuer_ca_file: issuer_ca_file.map(PathBuf::from),
            enabled,
            protocol3,
            timeout: duration(&value("GATEWAY_NATIVE_ADMISSION_TIMEOUT", "5s"), 8)?,
            ready_timeout: duration(&value("GATEWAY_REQUEST_TIMEOUT", "5s"), 30)?,
            version: value("GATEWAY_VERSION", "dev"),
        })
    }
    pub fn read_token(&self) -> Result<String, std::io::Error> {
        use std::io::Read;
        let mut b = String::new();
        oteryn_core::secret_file::open(SERVICE_TOKEN_FILE, &self.token_file)
            .map_err(|error| std::io::Error::new(error.kind(), error))?
            .take(8195)
            .read_to_string(&mut b)
            .map_err(|error| std::io::Error::new(error.kind(), SERVICE_TOKEN_FILE))?;
        let token = b.trim_end_matches(['\r', '\n']);
        if b.len() == 8195
            || token.is_empty()
            || token.len() > 8192
            || !token.bytes().all(|b| (0x21..=0x7e).contains(&b))
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid OTERYN_PLATFORM_SERVICE_TOKEN_FILE",
            ));
        }
        Ok(token.to_owned())
    }
}
pub fn validate_url(raw: &str) -> Result<(), String> {
    validate_issuer_url(raw, false)
}

pub(crate) fn validate_issuer_url(raw: &str, require_https: bool) -> Result<(), String> {
    let url = url::Url::parse(raw).map_err(|_| "invalid OTERYN_PLATFORM_BASE_URL")?;
    let host = url.host_str().ok_or("upstream host is required")?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("upstream URL must contain only scheme, host and optional path".into());
    }
    if require_https && url.scheme() != "https" {
        return Err(format!("{ISSUER_CA_FILE} requires an HTTPS upstream"));
    }
    let host = host.trim_matches(['[', ']']);
    if url.scheme() == "http"
        && !host.eq_ignore_ascii_case("localhost")
        && !host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
    {
        return Err("non-loopback upstream requires https".into());
    }
    Ok(())
}
fn duration(raw: &str, max: u64) -> Result<Duration, String> {
    let units = [
        ("ns", 1e-9),
        ("us", 1e-6),
        ("µs", 1e-6),
        ("μs", 1e-6),
        ("ms", 1e-3),
        ("s", 1.0),
        ("m", 60.0),
        ("h", 3600.0),
    ];
    let mut rest = raw.strip_prefix('+').unwrap_or(raw);
    let mut seconds = 0.0;
    while !rest.is_empty() {
        let end = rest
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .ok_or("invalid gateway timeout")?;
        let n: f64 = rest[..end].parse().map_err(|_| "invalid gateway timeout")?;
        rest = &rest[end..];
        let (unit, scale) = units
            .iter()
            .find(|(u, _)| rest.starts_with(u))
            .ok_or("invalid gateway timeout")?;
        seconds += n * scale;
        rest = &rest[unit.len()..];
    }
    if !seconds.is_finite() || seconds <= 0.0 || seconds > max as f64 {
        return Err("gateway timeout out of range".into());
    }
    Duration::try_from_secs_f64(seconds).map_err(|_| "invalid gateway timeout".into())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt};

    #[test]
    fn service_token_file_admission() -> Result<(), Box<dyn std::error::Error>> {
        let file = tempfile::NamedTempFile::new()?;
        fs::write(file.path(), b"synthetic\n")?;
        let config = Config::from_lookup(|key| match key {
            "OTERYN_PLATFORM_BASE_URL" => Some("http://127.0.0.1:8000".into()),
            "OTERYN_PLATFORM_SERVICE_TOKEN_FILE" => Some(file.path().display().to_string()),
            _ => None,
        })?;
        fs::set_permissions(file.path(), fs::Permissions::from_mode(0o644))?;
        let error = config.read_token().err().ok_or("mode was admitted")?;
        assert!(
            error
                .to_string()
                .contains("OTERYN_PLATFORM_SERVICE_TOKEN_FILE")
        );
        fs::set_permissions(file.path(), fs::Permissions::from_mode(0o600))?;
        assert_eq!(config.read_token()?, "synthetic");
        fs::write(file.path(), b"two words")?;
        assert!(config.read_token().is_err());
        Ok(())
    }
}
