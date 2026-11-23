use axum::http::{HeaderMap, header};

/// Does the request allow an HTML representation? The most specific media range wins.
pub fn accepts_html(headers: &HeaderMap) -> bool {
    !headers.contains_key(header::ACCEPT)
        || headers
            .get_all(header::ACCEPT)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .filter_map(|value| split_quoted(value, b','))
            .flatten()
            .filter_map(|value| {
                let mut parts = split_quoted(value, b';')?.into_iter().map(str::trim);
                let media = parts.next()?;
                let specificity = if media.eq_ignore_ascii_case("text/html") {
                    2
                } else if media.eq_ignore_ascii_case("text/*") {
                    1
                } else if media == "*/*" {
                    0
                } else {
                    return None;
                };
                // Portal HTML is text/html; charset=utf-8. Only matching media
                // parameters before the quality weight increase specificity.
                let mut charset = false;
                for part in parts.filter(|part| !part.is_empty()) {
                    let (name, value) = part.split_once('=')?;
                    let (name, value) = (name.trim(), value.trim());
                    if name.eq_ignore_ascii_case("q") {
                        let acceptable = value.parse::<f32>().is_ok_and(|q| q > 0.0 && q <= 1.0);
                        return Some((specificity, charset, acceptable));
                    }
                    if charset || !name.eq_ignore_ascii_case("charset") || !utf8_charset(value) {
                        return None;
                    }
                    charset = true;
                }
                Some((specificity, charset, true))
            })
            .max()
            .is_some_and(|(_, _, acceptable)| acceptable)
}

fn utf8_charset(value: &str) -> bool {
    let Some(quoted) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return value.eq_ignore_ascii_case("utf-8");
    };
    let mut bytes = quoted.bytes();
    std::iter::from_fn(|| match bytes.next()? {
        b'\\' => bytes.next(),
        byte => Some(byte),
    })
    .map(|byte| byte.to_ascii_lowercase())
    .eq(b"utf-8".iter().copied())
}

// A quoted media parameter may contain separators and escaped quotes.
fn split_quoted(value: &str, separator: u8) -> Option<Vec<&str>> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    let mut escaped = false;
    for (index, byte) in value.bytes().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        match byte {
            b'\\' if quoted => escaped = true,
            b'"' => quoted = !quoted,
            byte if byte == separator && !quoted => {
                parts.push(&value[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    if quoted {
        return None;
    }
    parts.push(&value[start..]);
    Some(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_negotiation_respects_quoted_media_parameters() {
        for (accept, expected) in [
            (r#"application/json; profile="one,text/html,two""#, false),
            (r#"application/json; profile="one\",text/html,two""#, false),
            (r#"application/json; profile="unclosed, text/html"#, false),
            (
                r#"application/json; profile="one,text/html,two", text/html;q=1"#,
                true,
            ),
            (r#"text/html; q=1; profile="one;q=0;two""#, true),
            (r#"text/html; q=0; profile="one;two", */*; q=1"#, false),
        ] {
            let mut headers = HeaderMap::new();
            headers.insert(header::ACCEPT, axum::http::HeaderValue::from_static(accept));
            assert_eq!(accepts_html(&headers), expected, "{accept}");
        }
    }

    #[test]
    fn html_negotiation_ranks_matching_charset_before_quality() {
        for (accept, expected) in [
            ("text/html; charset=utf-8; q=0, text/html; q=1", false),
            (r#"text/html;q=0, text/html; CHARSET="UTF-8"; q=0.5"#, true),
            (r#"text/html; charset="U\TF-8"; q=0, text/html"#, false),
            ("text/html; charset=iso-8859-1; q=0, text/html", true),
            ("text/html; charset=iso-8859-1", false),
            (r#"text/html; profile="one;two"; q=0, */*; q=1"#, true),
            (r#"text/html; profile="one;q=0;two"; q=1"#, false),
            ("text/*; charset=utf-8; q=0, text/html; q=1", true),
        ] {
            let mut headers = HeaderMap::new();
            headers.insert(header::ACCEPT, axum::http::HeaderValue::from_static(accept));
            assert_eq!(accepts_html(&headers), expected, "{accept}");
        }
    }
}
