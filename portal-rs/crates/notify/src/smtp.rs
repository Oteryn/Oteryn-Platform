//! Shared, certificate-validating SMTP transport configuration for every sender.
use lettre::{AsyncSmtpTransport, Tokio1Executor, transport::smtp::authentication::Credentials};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TlsMode {
    StartTls,
    Implicit,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransportConfig {
    pub mode: TlsMode,
    pub port: u16,
}
impl TransportConfig {
    pub fn parse(mode: Option<&str>, port: Option<&str>) -> Result<Self, &'static str> {
        let mode = match mode.unwrap_or("starttls") {
            "starttls" => TlsMode::StartTls,
            "implicit" => TlsMode::Implicit,
            _ => return Err("invalid SMTP TLS mode"),
        };
        let port = match port {
            Some(value) => value
                .parse::<u16>()
                .ok()
                .filter(|v| *v != 0)
                .ok_or("invalid SMTP port")?,
            None => {
                if mode == TlsMode::StartTls {
                    587
                } else {
                    465
                }
            }
        };
        Ok(Self { mode, port })
    }
    pub fn from_env() -> Result<Self, &'static str> {
        fn optional(key: &str) -> Result<Option<String>, &'static str> {
            match std::env::var(key) {
                Ok(value) => Ok(Some(value)),
                Err(std::env::VarError::NotPresent) => Ok(None),
                Err(_) => Err("invalid SMTP configuration encoding"),
            }
        }
        Self::parse(
            optional("OTERYN_SMTP_TLS_MODE")?.as_deref(),
            optional("OTERYN_SMTP_PORT")?.as_deref(),
        )
    }
    pub fn transport(
        self,
        host: &str,
        credentials: Credentials,
    ) -> Result<AsyncSmtpTransport<Tokio1Executor>, lettre::transport::smtp::Error> {
        let builder = match self.mode {
            TlsMode::StartTls => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)?,
            TlsMode::Implicit => AsyncSmtpTransport::<Tokio1Executor>::relay(host)?,
        };
        Ok(builder.port(self.port).credentials(credentials).build())
    }
}
pub fn transport(
    host: &str,
    username: String,
    password: String,
) -> crate::Result<AsyncSmtpTransport<Tokio1Executor>> {
    if username.is_empty() || password.is_empty() {
        return Err("SMTP username and password must be nonempty".into());
    }
    Ok(TransportConfig::from_env()?.transport(host, Credentials::new(username, password))?)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_transport_rejects_empty_credentials() {
        for (username, password) in [("", "fixture"), ("local", "")] {
            let error = transport("localhost", username.into(), password.into())
                .err()
                .map(|error| error.to_string());
            assert_eq!(
                error.as_deref(),
                Some("SMTP username and password must be nonempty")
            );
        }
    }

    #[test]
    fn shared_tls_mode_and_port_are_explicit_and_validated() {
        assert_eq!(
            TransportConfig::parse(None, None),
            Ok(TransportConfig {
                mode: TlsMode::StartTls,
                port: 587
            })
        );
        assert_eq!(
            TransportConfig::parse(Some("implicit"), Some("2465")),
            Ok(TransportConfig {
                mode: TlsMode::Implicit,
                port: 2465
            })
        );
        for mode in ["plain", "", "tls"] {
            assert!(TransportConfig::parse(Some(mode), None).is_err());
        }
        for port in ["0", "65536", "bad"] {
            assert!(TransportConfig::parse(None, Some(port)).is_err());
        }
    }
}
