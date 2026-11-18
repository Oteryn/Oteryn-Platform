use std::{borrow::Cow, path::Path};

// Inline PostgreSQL fixtures seed catalogue rows, model a completed CC1
// handoff, or build a worker backlog. Exempt only these statements in the
// cfg(test) portion of their source files.
const FIXTURE_WRITES: &[(&str, &str)] = &[
    (
        "game/runtime/browser.rs",
        "INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,state,ends_at,created_at) VALUES($1,$2,'item','auction','coins',100,false,CASE WHEN $3=0 THEN 'active'::bazaar_state ELSE 'completed'::bazaar_state END,now()+interval '1 day',now()+make_interval(secs=>$3))",
    ),
    (
        "game/runtime/browser.rs",
        "UPDATE bazaar_listings SET state='active'",
    ),
    (
        "game/runtime/browser.rs",
        "UPDATE bazaar_listings SET state='awaiting_delivery'",
    ),
    (
        "game/runtime/browser.rs",
        "UPDATE bazaar_listings SET state='completed',kind='character'",
    ),
    (
        "game/runtime/worker.rs",
        "INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,state,ends_at) VALUES($1,$2,'item','auction','coins',100,false,'awaiting_delivery',now())",
    ),
    (
        "game/runtime/worker.rs",
        "INSERT INTO bazaar_authority_gold_bids(request_id,listing_id,buyer,character_id,maximum,hold_operation_id) VALUES($1,$1,$1,$1,100,$1)",
    ),
    (
        "game/runtime/mod.rs",
        "INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,ends_at) VALUES($1,$2,'item','auction','coins',100,false,now()+interval '1 day')",
    ),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Token<'a> {
    Word(&'a str),
    Symbol(char),
    Literal(&'a str),
    RustString(&'a str),
}

impl Token<'_> {
    fn word(self, value: &str) -> bool {
        matches!(self, Self::Word(text) if text.eq_ignore_ascii_case(value))
    }
}

fn quoted(source: &str, quote: u8, escapes: bool) -> (&str, &str) {
    let bytes = source.as_bytes();
    let mut end = 1;
    while end < bytes.len() {
        if escapes && bytes[end] == b'\\' {
            end += 2;
        } else if bytes[end] == quote {
            if !escapes && bytes.get(end + 1) == Some(&quote) {
                end += 2;
            } else {
                return (&source[1..end], &source[end + 1..]);
            }
        } else {
            end += 1;
        }
    }
    (&source[1..], "")
}

fn block_comment(mut source: &str) -> &str {
    let mut depth = 1;
    while let Some(end) = source.find("*/") {
        let nested = source.find("/*").filter(|start| *start < end);
        let index = nested.unwrap_or(end);
        if nested.is_some() {
            depth += 1;
        } else {
            depth -= 1;
        }
        source = &source[index + 2..];
        if depth == 0 {
            return source;
        }
    }
    ""
}

fn raw_string(source: &str) -> Option<(&str, &str)> {
    let after_r = source.strip_prefix('r')?;
    let body_with_quote = after_r.trim_start_matches('#');
    let body = body_with_quote.strip_prefix('"')?;
    let delimiter = format!("\"{}", &after_r[..after_r.len() - body_with_quote.len()]);
    body.split_once(&delimiter)
}

fn dollar_string(source: &str) -> Option<(&str, &str)> {
    let (tag, body) = source.strip_prefix('$')?.split_once('$')?;
    if !tag.is_empty()
        && (!tag.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
            || !tag.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_'))
    {
        return None;
    }
    body.split_once(&source[..tag.len() + 2])
}

// Tokenize comments, strings and identifier boundaries before recognizing
// statement targets or Rust item delimiters.
fn tokens(mut source: &str, rust: bool) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    while let Some(first) = source.chars().next() {
        if first.is_whitespace() {
            source = source.trim_start();
        } else if source.starts_with(if rust { "//" } else { "--" }) {
            source = source.split_once('\n').map_or("", |(_, rest)| rest);
        } else if let Some(comment) = source.strip_prefix("/*") {
            source = block_comment(comment);
        } else if let Some((body, rest)) = if rust {
            raw_string(source)
        } else {
            dollar_string(source)
        } {
            tokens.push(Token::Literal(body));
            source = rest;
        } else if first == '"' || (!rust && first == '\'') {
            let (body, rest) = quoted(source, first as u8, rust);
            tokens.push(if rust {
                Token::RustString(body)
            } else if first == '"' {
                Token::Word(body)
            } else {
                Token::Literal(body)
            });
            source = rest;
        } else if rust
            && first == '\''
            && let Some(rest) = rust_character(source)
        {
            source = rest;
        } else if first.is_alphanumeric() || first == '_' {
            let end = source
                .find(|c: char| !c.is_alphanumeric() && c != '_')
                .unwrap_or(source.len());
            tokens.push(Token::Word(&source[..end]));
            source = &source[end..];
        } else {
            tokens.push(Token::Symbol(first));
            source = &source[first.len_utf8()..];
        }
    }
    tokens
}

fn rust_character(source: &str) -> Option<&str> {
    let body = source.strip_prefix('\'')?;
    if body.starts_with('\\') {
        // Escaped character literals have no newline or whitespace in the
        // escape itself. Skip their punctuation before counting braces.
        let end = body[2..].find('\'')? + 2;
        return Some(&body[end + 1..]);
    }
    let first = body.chars().next()?;
    body[first.len_utf8()..].strip_prefix('\'')
}

fn string_value(token: Token<'_>) -> Option<Cow<'_, str>> {
    let Token::RustString(source) = token else {
        return match token {
            Token::Literal(raw) => Some(Cow::Borrowed(raw)),
            _ => None,
        };
    };
    if !source.contains('\\') {
        return Some(Cow::Borrowed(source));
    }

    let mut decoded = String::new();
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            decoded.push(c);
            continue;
        }
        let ch = match chars.next()? {
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '0' => '\0',
            '\n' => {
                while chars.peek().is_some_and(|c| c.is_whitespace()) {
                    chars.next();
                }
                continue;
            }
            'x' => char::from_u32(chars.next()?.to_digit(16)? * 16 + chars.next()?.to_digit(16)?)?,
            'u' if chars.next()? == '{' => {
                let digits: String = chars
                    .by_ref()
                    .take_while(|c| *c != '}')
                    .filter(|c| *c != '_')
                    .collect();
                char::from_u32(u32::from_str_radix(&digits, 16).ok()?)?
            }
            c => c,
        };
        decoded.push(ch);
    }
    Some(Cow::Owned(decoded))
}

fn target<'a, 'sql>(mut tokens: &'a [Token<'sql>]) -> Option<(&'sql str, &'a [Token<'sql>])> {
    if tokens.first()?.word("only") {
        tokens = &tokens[1..];
    }
    let parenthesized = tokens.first() == Some(&Token::Symbol('('));
    if parenthesized {
        tokens = &tokens[1..];
    }
    let (Token::Word(table), mut rest) = tokens.split_first()? else {
        return None;
    };
    let mut table = *table;
    while let [Token::Symbol('.'), Token::Word(name), remaining @ ..] = rest {
        table = name;
        rest = remaining;
    }
    if parenthesized {
        rest = rest.strip_prefix(&[Token::Symbol(')')])?;
    }
    Some((table, rest))
}

fn assignment_to(tokens: &[Token<'_>], column: &str) -> bool {
    let rest = tokens.strip_prefix(&[Token::Symbol('*')]).unwrap_or(tokens);
    let rest = match rest {
        [set, ..] if set.word("set") => rest,
        [as_token, Token::Word(_), rest @ ..] if as_token.word("as") => rest,
        [Token::Word(_), rest @ ..] => rest,
        _ => return false,
    };
    let [set, assignments @ ..] = rest else {
        return false;
    };
    if !set.word("set") {
        return false;
    }
    let mut depth = 0;
    let mut lhs = true;
    for token in assignments {
        if depth == 0
            && (matches!(token, Token::Symbol(')' | ';'))
                || ["where", "from", "returning"]
                    .iter()
                    .any(|word| token.word(word)))
        {
            break;
        }
        match token {
            Token::Symbol('(' | '[') => depth += 1,
            Token::Symbol(')' | ']') => depth -= 1,
            Token::Symbol('=') if depth == 0 => lhs = false,
            Token::Symbol(',') if depth == 0 => lhs = true,
            _ if lhs && token.word(column) => return true,
            _ => {}
        }
    }
    false
}

fn forbidden_state_write(sql: &str) -> bool {
    const INSERT_TARGETS: &[&str] = &[
        "bazaar_listings",
        "bazaar_game_outbox",
        "bazaar_game_projections",
        "bazaar_authority_dispatch",
        "bazaar_authority_receipts",
        "bazaar_game_quarantine",
        "bazaar_authority_gold_bids",
        "bazaar_authority_transfers",
        "bazaar_authority_transfer_retries",
        "bazaar_authority_transfer_retirements",
        "character_commands",
    ];
    let tokens = tokens(sql, false);
    tokens.iter().enumerate().any(|(index, token)| {
        let rest = &tokens[index + 1..];
        if (token.word("insert") || token.word("merge"))
            && let [into, rest @ ..] = rest
            && into.word("into")
        {
            return target(rest).is_some_and(|(table, _)| {
                INSERT_TARGETS
                    .iter()
                    .any(|protected| table.eq_ignore_ascii_case(protected))
            });
        }
        if token.word("update")
            && let Some((table, rest)) = target(rest)
        {
            return ["bazaar_game_outbox", "bazaar_game_projections"]
                .iter()
                .any(|protected| table.eq_ignore_ascii_case(protected))
                || (table.eq_ignore_ascii_case("bazaar_listings") && assignment_to(rest, "state"))
                || (table.eq_ignore_ascii_case("bazaar_authority_gold_bids")
                    && assignment_to(rest, "phase"));
        }
        false
    })
}

fn normalize_sql(sql: &str) -> String {
    sql.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[derive(Default)]
struct ItemAttributes<'a> {
    test: bool,
    path: Option<&'a str>,
}

fn inspect<'a>(
    path: &Path,
    source: &'a [Token<'a>],
    inside_test: bool,
    modules: &mut Vec<(&'a str, Option<&'a str>)>,
    offenders: &mut Vec<String>,
) {
    use Token::{Literal, RustString, Symbol, Word};
    let mut remaining = source;
    let mut attributes = ItemAttributes::default();
    while let Some((token, rest)) = remaining.split_first() {
        remaining = rest;
        match token {
            Symbol('#') if rest.first() == Some(&Symbol('[')) => {
                let (args, after) = group(&rest[1..], ']');
                remaining = after;
                match args {
                    [Word("cfg"), Symbol('('), Word("test"), Symbol(')')] => attributes.test = true,
                    [Word("path"), Symbol('='), Literal(file) | RustString(file)] => {
                        attributes.path = Some(file)
                    }
                    _ => {}
                }
            }
            Word("mod") if let [Word(name), Symbol(';'), after @ ..] = rest => {
                if !inside_test && !attributes.test {
                    modules.push((name, attributes.path));
                }
                remaining = after;
                attributes = ItemAttributes::default();
            }
            Symbol(open @ ('{' | '(' | '[')) => {
                let close = match open {
                    '{' => '}',
                    '(' => ')',
                    _ => ']',
                };
                let (body, after) = group(rest, close);
                inspect(
                    path,
                    body,
                    inside_test || attributes.test,
                    modules,
                    offenders,
                );
                remaining = after;
                if *open == '{' {
                    attributes = ItemAttributes::default();
                }
            }
            Symbol(';') => attributes = ItemAttributes::default(),
            Literal(_) | RustString(_)
                if let Some(sql) = string_value(*token)
                    && forbidden_state_write(&sql) =>
            {
                let normalized = normalize_sql(&sql);
                if !(inside_test || attributes.test)
                    || !FIXTURE_WRITES
                        .iter()
                        .any(|(file, known)| path == Path::new(file) && normalized == *known)
                {
                    offenders.push(format!("{}: {normalized}", path.display()));
                }
            }
            _ => {}
        }
    }
}

fn group<'a>(source: &'a [Token<'a>], close: char) -> (&'a [Token<'a>], &'a [Token<'a>]) {
    let mut depth = 0;
    for (index, token) in source.iter().enumerate() {
        match token {
            Token::Symbol('{' | '(' | '[') => depth += 1,
            Token::Symbol(c) if *c == close && depth == 0 => {
                return (&source[..index], &source[index + 1..]);
            }
            Token::Symbol('}' | ')' | ']') => depth -= 1,
            _ => {}
        }
    }
    (source, &[])
}

pub(super) fn scan_source(path: &Path, source: &str, offenders: &mut Vec<String>) {
    if path != Path::new("lifecycle.rs") {
        inspect(
            path,
            &tokens(source, true),
            false,
            &mut Vec::new(),
            offenders,
        );
    }
}

pub(super) fn scan(root: &Path) -> std::io::Result<Vec<String>> {
    let mut pending = vec![Path::new("lib.rs").to_path_buf()];
    let mut offenders = Vec::new();
    while let Some(path) = pending.pop() {
        if path == Path::new("lifecycle.rs") {
            continue;
        }
        let source = std::fs::read_to_string(root.join(&path))?;
        let mut modules = Vec::new();
        let source_tokens = tokens(&source, true);
        inspect(&path, &source_tokens, false, &mut modules, &mut offenders);
        let parent = path.parent().unwrap_or(Path::new(""));
        let directory = if matches!(
            path.file_name().and_then(std::ffi::OsStr::to_str),
            Some("lib.rs" | "mod.rs")
        ) {
            parent.to_path_buf()
        } else {
            path.with_extension("")
        };
        for (module, explicit) in modules {
            if let Some(file) = explicit {
                pending.push(parent.join(file));
            } else {
                let default = directory.join(format!("{module}.rs"));
                pending.push(if root.join(&default).is_file() {
                    default
                } else {
                    directory.join(module).join("mod.rs")
                });
            }
        }
    }
    Ok(offenders)
}
