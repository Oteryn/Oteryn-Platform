#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use fluent_bundle::FluentResource;
use fluent_syntax::ast::Entry;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

fn collect_catalogs(directory: &Path, pairs: &mut BTreeMap<PathBuf, [Option<PathBuf>; 2]>) {
    for entry in fs::read_dir(directory).expect("read catalog directory") {
        let path = entry.expect("read catalog entry").path();
        if path.is_dir() {
            collect_catalogs(&path, pairs);
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "ftl") {
            continue;
        }

        let stem = path.file_stem().unwrap().to_str().unwrap();
        let (prefix, locale) = stem.rsplit_once(['_', '.']).unwrap_or(("", stem));
        let index = match locale {
            "pl" => 0,
            "en" => 1,
            _ => panic!("unrecognized locale in {}", path.display()),
        };
        let pair = pairs.entry(path.with_file_name(prefix)).or_default();
        assert!(
            pair[index].is_none(),
            "duplicate catalog {}",
            path.display()
        );
        pair[index] = Some(path);
    }
}

fn keys(path: &Path) -> BTreeSet<String> {
    let source = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("could not read {}: {error}", path.display());
    });
    let resource = FluentResource::try_new(source).unwrap_or_else(|(_, errors)| {
        panic!("invalid Fluent in {}: {errors:?}", path.display());
    });
    let mut keys = BTreeSet::new();
    for entry in resource.entries() {
        let (id, value, attributes) = match entry {
            Entry::Message(message) => (
                message.id.name.to_owned(),
                message.value.is_some(),
                &message.attributes,
            ),
            Entry::Term(term) => (format!("-{}", term.id.name), true, &term.attributes),
            _ => continue,
        };
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
    keys
}

#[test]
fn polish_and_english_catalogs_have_identical_keys() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut pairs = BTreeMap::new();
    for directory in ["crates", "templates"] {
        collect_catalogs(&workspace.join(directory), &mut pairs);
    }
    assert!(!pairs.is_empty(), "no Fluent catalogs found");
    let mut failures = Vec::new();
    for (name, pair) in pairs {
        let [Some(pl), Some(en)] = pair else {
            failures.push(format!("missing locale catalog in {}", name.display()));
            continue;
        };
        let (pl_keys, en_keys) = (keys(&pl), keys(&en));
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
}
