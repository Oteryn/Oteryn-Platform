use fluent_bundle::FluentResource;
use fluent_syntax::ast::Entry;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn collect_catalogs(
    directory: &Path,
    pairs: &mut BTreeMap<PathBuf, [Option<PathBuf>; 2]>,
) -> TestResult {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_catalogs(&path, pairs)?;
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "ftl") {
            continue;
        }

        let stem = path
            .file_stem()
            .and_then(|name| name.to_str())
            .ok_or("invalid catalog filename")?;
        let (prefix, locale) = stem.rsplit_once(['_', '.']).unwrap_or(("", stem));
        let index = match locale {
            "pl" => 0,
            "en" => 1,
            _ => return Err(format!("unrecognized locale in {}", path.display()).into()),
        };
        let pair = pairs.entry(path.with_file_name(prefix)).or_default();
        assert!(
            pair[index].is_none(),
            "duplicate catalog {}",
            path.display()
        );
        pair[index] = Some(path);
    }
    Ok(())
}

fn keys(path: &Path) -> TestResult<BTreeSet<String>> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let resource = FluentResource::try_new(source)
        .map_err(|(_, errors)| format!("invalid Fluent in {}: {errors:?}", path.display()))?;
    Ok(resource_keys(&resource, path)?)
}

fn resource_keys(resource: &FluentResource, path: &Path) -> Result<BTreeSet<String>, String> {
    let mut ids = BTreeSet::new();
    let mut keys = BTreeSet::new();
    for entry in resource.entries() {
        let (id, prefix, value, attributes) = match entry {
            Entry::Message(message) => (
                message.id.name,
                "",
                message.value.is_some(),
                &message.attributes,
            ),
            Entry::Term(term) => (term.id.name, "-", true, &term.attributes),
            _ => continue,
        };
        if !ids.insert(id) {
            return Err(format!("duplicate entry ID {id} in {}", path.display()));
        }
        let id = format!("{prefix}{id}");
        if value {
            assert!(
                keys.insert(id.clone()),
                "duplicate key {id} in {}",
                path.display()
            );
        }
        for attribute in attributes {
            let key = format!("{id}.{}", attribute.id.name);
            assert!(
                keys.insert(key.clone()),
                "duplicate key {key} in {}",
                path.display()
            );
        }
    }
    Ok(keys)
}

#[test]
fn rejects_duplicate_attribute_only_messages() -> TestResult {
    let resource = FluentResource::try_new(
        "button =\n    .label = Label\nbutton =\n    .title = Title\n".into(),
    )
    .map_err(|(_, errors)| format!("invalid Fluent fixture: {errors:?}"))?;
    assert_eq!(
        resource_keys(&resource, Path::new("duplicate.ftl")),
        Err("duplicate entry ID button in duplicate.ftl".into())
    );
    Ok(())
}

#[test]
fn rejects_message_and_term_with_the_same_id() -> TestResult {
    let resource = FluentResource::try_new("label = Message\n-label = Term\n".into())
        .map_err(|(_, errors)| format!("invalid Fluent fixture: {errors:?}"))?;
    assert_eq!(
        resource_keys(&resource, Path::new("message-term.ftl")),
        Err("duplicate entry ID label in message-term.ftl".into())
    );
    Ok(())
}

#[test]
fn polish_and_english_catalogs_have_identical_keys() -> TestResult {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut pairs = BTreeMap::new();
    for directory in ["crates", "templates"] {
        collect_catalogs(&workspace.join(directory), &mut pairs)?;
    }
    assert!(!pairs.is_empty(), "no Fluent catalogs found");
    let mut failures = Vec::new();
    for (name, pair) in pairs {
        let [Some(pl), Some(en)] = pair else {
            failures.push(format!("missing locale catalog in {}", name.display()));
            continue;
        };
        let (pl_keys, en_keys) = (keys(&pl)?, keys(&en)?);
        if pl_keys != en_keys {
            failures.push(format!(
                "{} / {}: missing in pl {:?}, missing in en {:?}",
                pl.display(),
                en.display(),
                en_keys.difference(&pl_keys).collect::<Vec<_>>(),
                pl_keys.difference(&en_keys).collect::<Vec<_>>()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    Ok(())
}
