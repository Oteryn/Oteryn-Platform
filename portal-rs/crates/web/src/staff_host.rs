//! The HTTPS authority used by staff navigation, host guards and CSRF.

use axum::http::{HeaderMap, header, uri::Authority};

/// Accept only a hostname or IP address with an optional numeric port.
pub fn canonical_authority(input: &str) -> Option<String> {
    let authority = input.parse::<Authority>().ok()?;
    let host = authority.host();
    // The HTTP authority parser accepts userinfo. The entire value must be host[:port].
    match input.strip_prefix(host)? {
        "" => {}
        suffix => {
            let port = suffix.strip_prefix(':')?;
            if !port.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            port.parse::<u16>().ok()?;
        }
    }
    if !host.starts_with('[') {
        let name = host.strip_suffix('.').unwrap_or(host);
        if name.len() > 253
            || !name.split('.').all(|label| {
                let label = label.as_bytes();
                label.len() <= 63
                    && label.first().is_some_and(u8::is_ascii_alphanumeric)
                    && label.last().is_some_and(u8::is_ascii_alphanumeric)
                    && label
                        .iter()
                        .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
            })
        {
            return None;
        }
    }
    // Serialize like the browser, including omission of HTTPS's default port.
    let url = url::Url::parse(&format!("https://{input}/")).ok()?;
    let host = url.host_str()?;
    Some(match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_owned(),
    })
}

pub fn matches(headers: &HeaderMap, canonical: &str) -> bool {
    let mut hosts = headers.get_all(header::HOST).iter();
    hosts
        .next()
        .and_then(|host| host.to_str().ok())
        .and_then(canonical_authority)
        .is_some_and(|host| host.eq_ignore_ascii_case(canonical))
        && hosts.next().is_none()
}
