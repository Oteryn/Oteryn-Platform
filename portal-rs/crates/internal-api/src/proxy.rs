//! Client addresses across explicitly trusted reverse proxies.
use axum::http::HeaderMap;
use std::net::{IpAddr, SocketAddr};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyError {
    BadInput,
}
impl std::fmt::Display for ProxyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid proxy address configuration")
    }
}
impl std::error::Error for ProxyError {}
/// Exact deployment-configured proxy addresses; empty means no trust.
/// Walk X-Forwarded-For right to left, stopping at the first untrusted address.
/// Trusted proxies must append/overwrite the header correctly.
#[derive(Clone, Debug, Default)]
pub struct TrustedProxies(pub Vec<IpAddr>);
impl TrustedProxies {
    pub fn parse(value: &str) -> Result<Self, ProxyError> {
        if value.trim().is_empty() {
            return Ok(Self::default());
        }
        let addresses = value
            .split(',')
            .map(|ip| {
                ip.trim()
                    .parse::<IpAddr>()
                    .map_err(|_| ProxyError::BadInput)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if addresses.len() > 32 {
            return Err(ProxyError::BadInput);
        }
        Ok(Self(addresses))
    }
    pub fn configured() -> Result<Self, ProxyError> {
        match std::env::var("TRUSTED_PROXIES") {
            Ok(value) => Self::parse(&value),
            Err(std::env::VarError::NotPresent) => Ok(Self::default()),
            Err(_) => Err(ProxyError::BadInput),
        }
    }

    pub fn client_ip(&self, peer: SocketAddr, headers: &HeaderMap) -> Result<IpAddr, ProxyError> {
        if !self.0.contains(&peer.ip()) {
            return Ok(peer.ip());
        }
        let mut chain = Vec::new();
        for value in headers.get_all("x-forwarded-for") {
            let value = value.to_str().map_err(|_| ProxyError::BadInput)?;
            if value.len() > 4096 {
                return Err(ProxyError::BadInput);
            }
            for ip in value.split(',') {
                if chain.len() >= 32 {
                    return Err(ProxyError::BadInput);
                }
                chain.push(
                    ip.trim()
                        .parse::<IpAddr>()
                        .map_err(|_| ProxyError::BadInput)?,
                );
            }
        }
        let mut client = peer.ip();
        for ip in chain.into_iter().rev() {
            if !self.0.contains(&client) {
                break;
            }
            client = ip;
        }
        Ok(client)
    }
}
