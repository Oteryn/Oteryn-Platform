use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use fluent_bundle::FluentResource;

type Entries = BTreeMap<String, String>;
type TemplateLiteral<'a> = (usize, bool, &'a str, Option<&'a str>);

fn files(directory: &Path, result: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            files(&entry.path(), result)?;
        } else if kind.is_file() {
            result.push(entry.path());
        }
    }
    Ok(())
}

fn catalogue_name(path: &Path) -> Result<(&str, &str, &str), String> {
    let invalid_name = || format!("invalid catalogue name {path:?}");
    let file = path
        .file_stem()
        .and_then(|part| part.to_str())
        .ok_or_else(invalid_name)?;
    let (prefix, lang) = file
        .split_at_checked(file.len().saturating_sub(2))
        .ok_or_else(invalid_name)?;
    assert!(
        matches!(lang, "pl" | "en"),
        "unclassified catalogue {path:?}"
    );
    let extension = path
        .extension()
        .and_then(|part| part.to_str())
        .ok_or_else(invalid_name)?;
    Ok((prefix, lang, extension))
}

fn catalogue(path: &Path, source: &str, fluent: bool) -> Entries {
    let mut entries = Entries::new();
    if fluent {
        assert!(
            FluentResource::try_new(source.to_owned()).is_ok(),
            "invalid Fluent syntax in {path:?}"
        );
    }
    // Keep source text (including select variants) so the checks do not need
    // arbitrary argument values to render Fluent patterns.
    let separator = if fluent { '=' } else { '\t' };
    let mut current_key = String::new();
    let mut last_key = String::new();
    for line in source.lines().filter(|line| !line.trim().is_empty()) {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once(separator)
            && (!line.starts_with(char::is_whitespace) || trimmed.starts_with('.'))
        {
            let key = if let Some(attribute) = key.trim().strip_prefix('.') {
                format!("{current_key}.{attribute}")
            } else {
                current_key = key.trim().to_owned();
                current_key.clone()
            };
            assert!(
                entries
                    .insert(key.clone(), value.trim().to_owned())
                    .is_none(),
                "duplicate key {key} in {path:?}"
            );
            last_key = key;
        } else if let Some(value) = entries.get_mut(&last_key) {
            value.push(' ');
            value.push_str(trimmed);
        }
    }
    entries
}

fn words(value: &str) -> Vec<&str> {
    value
        .split(|character: char| !character.is_alphabetic())
        .filter(|word| !word.is_empty())
        .collect()
}

fn english_sentence(value: &str) -> bool {
    // Two distinct English function words plus four words total catches prose
    // while avoiding product names, borrowed vocabulary and short labels.
    let lowercase = value.to_lowercase();
    let words = words(&lowercase);
    let markers = words
        .iter()
        .filter(|word| {
            matches!(
                **word,
                "the"
                    | "your"
                    | "you"
                    | "this"
                    | "with"
                    | "from"
                    | "for"
                    | "and"
                    | "are"
                    | "will"
                    | "must"
            )
        })
        .collect::<BTreeSet<_>>();
    words.len() >= 4 && markers.len() >= 2
}

fn check_pair(location: &str, pl: Option<&str>, en: Option<&str>, errors: &mut Vec<String>) {
    let (Some(pl), Some(en)) = (pl, en) else {
        errors.push(format!(
            "{location}: missing {}",
            if pl.is_none() { "pl" } else { "en" }
        ));
        return;
    };
    if pl.trim().is_empty() || en.trim().is_empty() {
        errors.push(format!("{location}: empty translation"));
    }
    // A comma-separated identifier inventory (such as the reserved character
    // names) has no sentence to translate. The classifier needs at least four
    // comma-delimited entries, each a single identifier.
    let identifier_list = en.split(';').next().is_some_and(|head| {
        let entries: Vec<_> = head.split(',').map(str::trim).collect();
        entries.len() >= 4
            && entries.iter().all(|entry| {
                !entry.is_empty() && entry.bytes().all(|byte| byte.is_ascii_lowercase())
            })
    });
    if pl == en && words(en).len() >= 3 && !identifier_list {
        errors.push(format!("{location}: identical sentence-like translations"));
    }
    if en.contains(|c| "ąćęłńóśźżĄĆĘŁŃÓŚŹŻ".contains(c)) {
        errors.push(format!("{location}: Polish diacritics in en"));
    }
    if english_sentence(pl) {
        errors.push(format!("{location}: English sentence in pl"));
    }
}

fn check_catalogues(location: &str, pl: &Entries, en: &Entries, errors: &mut Vec<String>) -> usize {
    let keys = pl.keys().chain(en.keys()).collect::<BTreeSet<_>>();
    for key in &keys {
        check_pair(
            &format!("{location}:{key}"),
            pl.get(*key).map(String::as_str),
            en.get(*key).map(String::as_str),
            errors,
        );
    }
    keys.len()
}

fn is_embedded_catalogue(value: &str) -> bool {
    value.contains("\\n")
        && value.split_once(" = ").is_some_and(|(key, _)| {
            !key.is_empty()
                && key
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte == b'-')
        })
}

fn template_literals(source: &str) -> Result<Vec<TemplateLiteral<'_>>, String> {
    let mut literals = Vec::new();
    let mut offset = 0;
    while let Some(start) = source[offset..].find("{%") {
        let start = offset + start;
        let rest = &source[start + 2..];
        let Some((directive, body)) = rest.split_once("%}") else {
            break;
        };
        offset = start + 2 + directive.len() + 2;
        let lang = match directive.split_whitespace().collect::<Vec<_>>().as_slice() {
            [
                "if",
                "lang" | "locale" | "language" | "self.lang" | "chrome.lang",
                "==",
                "\"pl\"",
            ] => true,
            [
                "if",
                "lang" | "locale" | "language" | "self.lang" | "chrome.lang",
                "==",
                "\"en\"",
            ] => false,
            _ => continue,
        };
        // The current inline locale switches have literal arms. Refuse any new
        // shape so nested or more complex switches cannot silently escape the check.
        let (first, rest) = body.split_once("{%").ok_or("locale switch without endif")?;
        // Language-switcher URLs and active-link attributes carry no copy.
        if first.trim().starts_with('#') || first.trim().starts_with("aria-current=") {
            continue;
        }
        let (branch, rest) = rest
            .split_once("%}")
            .ok_or("unfinished template directive")?;
        let second = match branch.trim() {
            "endif" => None,
            "else" => {
                let (second, end) = rest.split_once("{%").ok_or("locale switch without endif")?;
                assert!(
                    end.split_once("%}")
                        .is_some_and(|(tag, _)| tag.trim() == "endif"),
                    "unsupported nested locale switch after {second:?}"
                );
                Some(second.trim())
            }
            _ => return Err(format!("unsupported locale switch directive: {branch}")),
        };
        literals.push((source[..start].lines().count(), lang, first.trim(), second));
    }
    Ok(literals)
}

// Simple inline Rust copy is expressed as a pair of string literals (or
// tuples of literals). Catalogue selectors and other executable branches are
// checked at their own source, so they have no literal pair here.
fn literal_values(body: &str) -> Option<Vec<&str>> {
    let body = body.trim();
    let body = body
        .strip_prefix('(')
        .and_then(|body| body.strip_suffix(')'))
        .unwrap_or(body);
    let mut values = Vec::new();
    let mut remaining = body.trim();
    while !remaining.is_empty() {
        let value_start = remaining.strip_prefix('"')?;
        let (value, tail) = value_start.split_once('"')?;
        // Localized URLs and locale codes are routing data.
        if value.starts_with('/') || matches!(value, "pl" | "en") {
            return None;
        }
        values.push(value);
        remaining = tail.trim_start();
        if !remaining.is_empty() {
            remaining = remaining.strip_prefix(',')?.trim_start();
        }
    }
    (!values.is_empty()).then_some(values)
}

fn rust_literals(source: &str) -> Vec<(String, Option<&str>, Option<&str>)> {
    let mut pairs = Vec::new();
    for (index, _) in source.match_indices("if ") {
        let Some((condition, arms)) = source[index + 3..].split_once('{') else {
            continue;
        };
        let polish_first = match condition.trim() {
            "lang == \"pl\"" | "locale == \"pl\"" | "language == \"pl\"" => true,
            "english" | "en" | "lang == \"en\"" | "locale == \"en\"" | "language == \"en\"" => {
                false
            }
            _ => continue,
        };
        let Some((first, tail)) = arms.split_once('}') else {
            continue;
        };
        let second = tail
            .trim_start()
            .strip_prefix("else")
            .and_then(|tail| tail.trim_start().strip_prefix('{'))
            .and_then(|tail| tail.split_once('}'))
            .and_then(|(second, _)| literal_values(second));
        let first = literal_values(first);
        let (pl, en) = if polish_first {
            (first, second)
        } else {
            (second, first)
        };
        let count = pl
            .as_ref()
            .map_or(0, Vec::len)
            .max(en.as_ref().map_or(0, Vec::len));
        for entry in 0..count {
            pairs.push((
                format!("{}:{}", source[..index].lines().count(), entry + 1),
                pl.as_ref().and_then(|values| values.get(entry)).copied(),
                en.as_ref().and_then(|values| values.get(entry)).copied(),
            ));
        }
    }

    // Scan locale/key match arms by their key, independently of arm order.
    // Current tables use either language enum variants or an English flag.
    let mut table = BTreeMap::<&str, (Option<&str>, Option<&str>)>::new();
    let mut start = 0;
    let mut english_flag = false;
    let lines: Vec<_> = source.lines().collect();
    for (line, code) in lines.iter().enumerate() {
        let code = code.trim();
        if code.contains("match ") || code.starts_with('}') {
            for (key, (pl, en)) in std::mem::take(&mut table) {
                pairs.push((format!("{start}:{key}"), pl, en));
            }
            start = line + 1;
            english_flag = code.contains(", english)");
        }
        let Some((pattern, value)) = code.split_once("=>") else {
            continue;
        };
        fn literal(line: &str) -> Option<&str> {
            line.trim()
                .strip_prefix('"')
                .and_then(|value| value.split('"').next())
        }
        let value = if value.trim() == "(" {
            // Match arms can return a locale identifier together with inline
            // Fluent. The identifier is routing data, so select the resource.
            lines[line + 1..]
                .iter()
                .take_while(|line| !line.trim_start().starts_with(')'))
                .copied()
                .filter_map(literal)
                .find(|value| is_embedded_catalogue(value))
        } else {
            literal(value)
        };
        let Some(value) = value else { continue };
        let pattern = pattern.trim().trim_start_matches('(').trim_end_matches(')');
        let parts = pattern.split(',').map(str::trim).collect::<Vec<_>>();
        let locale = |part| match part {
            "Language::Pl" | "Locale::Pl" | "\"pl\"" => Some(true),
            "Language::En" | "Locale::En" | "\"en\"" => Some(false),
            "false" if english_flag => Some(true),
            "true" if english_flag => Some(false),
            _ => None,
        };
        let (key, polish) = match parts.as_slice() {
            [part] => ("literal", locale(part)),
            [first, second] if locale(first).is_some() => (*second, locale(first)),
            [first, second] => (*first, locale(second)),
            _ => continue,
        };
        if let Some(polish) = polish {
            let (pl, en) = table.entry(key).or_default();
            let slot = if polish { pl } else { en };
            assert!(
                slot.replace(value).is_none(),
                "duplicate locale arm at line {}",
                line + 1
            );
        }
    }
    pairs
}

fn owner(path: &Path) -> Result<&str, String> {
    let mut components = path.iter();
    let unclassified = || format!("classify translation owner for {path:?}");
    let mut next = || {
        components
            .next()
            .and_then(|part| part.to_str())
            .ok_or_else(unclassified)
    };
    let owner = match next()? {
        "crates" | "bin" => next()?,
        "templates" => match next()? {
            "components" => "ui",
            "account" => match next()? {
                "overview.pl.ftl" | "overview.en.ftl" => "characters",
                _ => "identity",
            },
            "admin" if next().is_ok_and(|part| part == "world") => "world",
            "atlas" => "atlas",
            "bazaar" => "bazaar",
            "badges" | "profile" | "services" => "characters",
            "guilds" | "houses" | "profiles" | "today" => "community",
            "events" | "pages" => "content",
            "oauth" => "gameauth",
            "pass" => "minigames",
            "playerdata" => "playerdata",
            "stats" => "stats",
            "wiki" => "wiki",
            "admin" => "admin",
            _ => return Err(unclassified()),
        },
        "static" => "design gallery",
        _ => return Err(unclassified()),
    };
    Ok(owner)
}

#[test]
fn polish_and_english_translation_sources_stay_in_parity() -> Result<(), Box<dyn std::error::Error>>
{
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut paths = Vec::new();
    for directory in ["crates", "bin", "templates", "static/design/locales"] {
        files(&root.join(directory), &mut paths)?;
    }
    paths.sort();
    let (mut errors, mut counts) = (Vec::new(), BTreeMap::new());
    for path in &paths {
        let relative = path.strip_prefix(&root)?;
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("ftl" | "tsv") => {
                let (prefix, lang, extension) = catalogue_name(path)?;
                let other = if lang == "pl" { "en" } else { "pl" };
                let counterpart = path.with_file_name(format!("{prefix}{other}.{extension}"));
                if !counterpart.exists() {
                    errors.push(format!("{relative:?}: missing {other} catalogue"));
                    continue;
                }
                if lang == "en" {
                    continue;
                }
                let fluent = extension == "ftl";
                let pl = catalogue(path, &fs::read_to_string(path)?, fluent);
                let en = catalogue(&counterpart, &fs::read_to_string(&counterpart)?, fluent);
                *counts.entry(owner(relative)?).or_insert(0) +=
                    check_catalogues(&relative.display().to_string(), &pl, &en, &mut errors);
            }
            Some(extension @ ("html" | "rs")) => {
                let source = fs::read_to_string(path)?;
                if extension == "rs"
                    && !relative
                        .components()
                        .any(|part| part.as_os_str() == "tests")
                    && !relative
                        .file_name()
                        .and_then(|part| part.to_str())
                        .ok_or_else(|| format!("invalid Rust source file name {relative:?}"))?
                        .ends_with("_tests.rs")
                {
                    for (key, pl, en) in rust_literals(&source) {
                        let location = format!("{}:{key}", relative.display());
                        let count = if pl.is_some_and(is_embedded_catalogue)
                            || en.is_some_and(is_embedded_catalogue)
                        {
                            let pl = catalogue(
                                relative,
                                &pl.unwrap_or_default().replace("\\n", "\n"),
                                true,
                            );
                            let en = catalogue(
                                relative,
                                &en.unwrap_or_default().replace("\\n", "\n"),
                                true,
                            );
                            check_catalogues(&location, &pl, &en, &mut errors)
                        } else {
                            check_pair(&location, pl, en, &mut errors);
                            1
                        };
                        *counts.entry(owner(relative)?).or_insert(0) += count;
                    }
                }
                for (line, polish_first, first, second) in
                    template_literals(&source).map_err(|error| format!("{relative:?}: {error}"))?
                {
                    *counts.entry(owner(relative)?).or_insert(0) += 1;
                    let (pl, en) = if polish_first {
                        (Some(first), second)
                    } else {
                        (second, Some(first))
                    };
                    check_pair(
                        &format!("{}:{line}", relative.display()),
                        pl,
                        en,
                        &mut errors,
                    );
                }
            }
            _ => {}
        }
    }
    for (owner, count) in counts {
        println!("{owner}: {count}");
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    Ok(())
}
