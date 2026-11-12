//! Gateway configuration is independent of portal and database configuration.
use std::{fmt, net::IpAddr, path::PathBuf, time::Duration};
#[derive(Clone)]
pub struct Config {
    pub listen: String,
    pub base_url: String,
    pub token_file: PathBuf,
    pub enabled: bool,
    /// `GATEWAY_NATIVE_PROTOCOL3_ENABLED` (contract §19.1), default off.
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
        validate_url(&base_url)?;
        let token_file = value("OTERYN_PLATFORM_SERVICE_TOKEN_FILE", "");
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
            "true" => true,
            _ => return Err("invalid GATEWAY_NATIVE_PROTOCOL3_ENABLED".into()),
        };
        let listen = value("GATEWAY_LISTEN_ADDR", "0.0.0.0:8080");
        if listen.trim().is_empty() {
            return Err("GATEWAY_LISTEN_ADDR is empty".into());
        }
        Ok(Self {
            listen,
            base_url,
            token_file: token_file.into(),
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
        std::fs::File::open(&self.token_file)?
            .take(8193)
            .read_to_string(&mut b)?;
        let token = b.trim_end_matches(['\r', '\n']);
        if token.is_empty()
            || token.len() > 8192
            || !token.bytes().all(|b| (0x21..=0x7e).contains(&b))
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid service token file",
            ));
        }
        Ok(token.to_owned())
    }
}
pub fn validate_url(raw: &str) -> Result<(), String> {
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
