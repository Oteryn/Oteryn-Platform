use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
};

use globset::{GlobBuilder, GlobMatcher};
use jsonschema::{Retrieve, Uri, Validator};
use serde_json::Value;
use url::Url;

use crate::{ImportConfig, ImportIssue, ImportMapping, SchemaScope, types::validate_label};

const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_RECORDS: usize = 100_000;

#[derive(Debug)]
pub(crate) struct Document {
    pub kind: String,
    pub key: String,
    pub data: Value,
    pub publishable: bool,
}

struct CompiledMapping {
    mapping: ImportMapping,
    matcher: GlobMatcher,
    validator: Validator,
    matched: bool,
}

/// Only local references within the supplied schema tree can be resolved.
/// HTTP retrieval is disabled in the dependency as well as rejected here.
struct LocalSchemas {
    root: PathBuf,
    resources: std::collections::BTreeMap<String, PathBuf>,
}

impl Retrieve for LocalSchemas {
    fn retrieve(
        &self,
        uri: &Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let path = if let Some(path) = self.resources.get(uri.as_str()) {
            path.clone()
        } else {
            let url = Url::parse(uri.as_str())?;
            url.to_file_path()
                .map_err(|()| "unregistered schema URI; references must resolve locally")?
        };
        let path = path.canonicalize()?;
        if !path.starts_with(&self.root) {
            return Err("schema reference leaves tools/content-schema".into());
        }
        read_json(&path).map_err(Into::into)
    }
}

fn issue(
    file: impl Into<String>,
    pointer: impl Into<String>,
    message: impl ToString,
) -> ImportIssue {
    ImportIssue {
        file: file.into(),
        pointer: pointer.into(),
        message: message.to_string(),
    }
}

fn read_json(path: &Path) -> Result<Value, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err("JSON file exceeds 16 MiB".to_owned());
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

fn normal_relative(path: &str) -> bool {
    !path.is_empty()
        && Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}

fn collect_json(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), ImportIssue> {
    let relative = |path: &Path| {
        path.strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned()
    };
    for entry in fs::read_dir(dir).map_err(|e| issue(relative(dir), "", e))? {
        let entry = entry.map_err(|e| issue(relative(dir), "", e))?;
        let path = entry.path();
        let ty = entry
            .file_type()
            .map_err(|e| issue(relative(&path), "", e))?;
        if ty.is_symlink() {
            return Err(issue(
                relative(&path),
                "",
                "symlinks are not accepted in content",
            ));
        }
        if ty.is_dir() {
            collect_json(root, &path, files)?;
        } else if ty.is_file() && path.extension().is_some_and(|ext| ext == "json") {
            files.push(path);
        }
    }
    Ok(())
}

fn compile(root: &Path, config: &ImportConfig) -> Result<Vec<CompiledMapping>, ImportIssue> {
    let mappings = config.mappings.clone();
    if mappings.is_empty() {
        return Err(issue("", "", "at least one mapping is required"));
    }
    let schemas = root
        .join("tools/content-schema")
        .canonicalize()
        .map_err(|e| issue("tools/content-schema", "", e))?;
    if !schemas.starts_with(root) {
        return Err(issue(
            "tools/content-schema",
            "",
            "schema directory leaves source root",
        ));
    }
    let mut compiled = Vec::new();
    let mut resources = std::collections::BTreeMap::new();
    for (uri, file) in &config.schema_resources {
        if !normal_relative(file) || !file.starts_with("tools/content-schema/") {
            return Err(issue(
                file,
                "",
                "schema resource must be under tools/content-schema/",
            ));
        }
        let path = root
            .join(file)
            .canonicalize()
            .map_err(|e| issue(file, "", e))?;
        if !path.starts_with(&schemas) {
            return Err(issue(
                file,
                "",
                "schema resource leaves tools/content-schema/",
            ));
        }
        resources.insert(uri.clone(), path);
    }
    for mapping in mappings {
        let file = mapping.schema_file.clone();
        for (label, name) in [(&mapping.kind, "kind"), (&mapping.key_field, "key_field")] {
            validate_label(label, name).map_err(|e| issue(&file, "", e))?;
        }
        if !normal_relative(&mapping.path_glob) || !mapping.path_glob.starts_with("content/") {
            return Err(issue(
                &file,
                "",
                "glob must be relative to content/ without traversal",
            ));
        }
        if !normal_relative(&file) || !file.starts_with("tools/content-schema/") {
            return Err(issue(
                &file,
                "",
                "schema must be under tools/content-schema/",
            ));
        }
        if !valid_pointer(&mapping.records_pointer) {
            return Err(issue(&file, "", "records_pointer must be a JSON pointer"));
        }
        if mapping.key_field.starts_with('/') && !valid_pointer(&mapping.key_field) {
            return Err(issue(&file, "", "key_field must be a valid JSON pointer"));
        }
        if mapping.hidden_when_true.iter().any(|p| !valid_pointer(p)) {
            return Err(issue(
                &file,
                "",
                "hidden_when_true entries must be JSON pointers",
            ));
        }
        let schema_path = root
            .join(&file)
            .canonicalize()
            .map_err(|e| issue(&file, "", e))?;
        if !schema_path.starts_with(&schemas) {
            return Err(issue(&file, "", "schema leaves tools/content-schema/"));
        }
        let schema = read_json(&schema_path).map_err(|e| issue(&file, "", e))?;
        let base = Url::from_file_path(&schema_path)
            .map_err(|()| issue(&file, "", "invalid schema file path"))?;
        let validator = jsonschema::options()
            .with_base_uri(base.as_str())
            .with_retriever(LocalSchemas {
                root: schemas.clone(),
                resources: resources.clone(),
            })
            .build(&schema)
            .map_err(|e| issue(&file, e.instance_path().to_string(), e.to_string()))?;
        let matcher = GlobBuilder::new(&mapping.path_glob)
            .literal_separator(true)
            .build()
            .map_err(|e| issue(&file, "", e))?
            .compile_matcher();
        compiled.push(CompiledMapping {
            mapping,
            matcher,
            validator,
            matched: false,
        });
    }
    Ok(compiled)
}

fn valid_pointer(pointer: &str) -> bool {
    if !pointer.is_empty() && !pointer.starts_with('/') {
        return false;
    }
    let mut chars = pointer.chars();
    while let Some(ch) = chars.next() {
        if ch == '~' && !matches!(chars.next(), Some('0' | '1')) {
            return false;
        }
    }
    true
}

/// Validation runs on a blocking thread. No database changes occur here.
#[cfg(test)]
fn validate(
    root: PathBuf,
    mappings: Vec<ImportMapping>,
) -> Result<Vec<Document>, Vec<ImportIssue>> {
    validate_config(
        root,
        ImportConfig {
            mappings,
            ..ImportConfig::default()
        },
    )
}

pub(crate) fn validate_config(
    root: PathBuf,
    config: ImportConfig,
) -> Result<Vec<Document>, Vec<ImportIssue>> {
    validate_inner(root, config).map_err(|e| vec![e])
}

fn validate_inner(root: PathBuf, config: ImportConfig) -> Result<Vec<Document>, ImportIssue> {
    let root = root.canonicalize().map_err(|e| issue("", "", e))?;
    let content = root.join("content");
    if fs::symlink_metadata(&content)
        .map_err(|e| issue("content", "", e))?
        .file_type()
        .is_symlink()
    {
        return Err(issue(
            "content",
            "",
            "content directory cannot be a symlink",
        ));
    }
    let mut mappings = compile(&root, &config)?;
    let mut excluded = HashSet::new();
    for file in &config.excluded_files {
        if !normal_relative(file) || !file.starts_with("content/") || !excluded.insert(file.clone())
        {
            return Err(issue(
                file,
                "",
                "exclusions must be unique exact paths under content/",
            ));
        }
    }
    let mut files = Vec::new();
    collect_json(&root, &content, &mut files)?;
    files.sort();
    let mut documents = Vec::new();
    let mut key_sources = HashMap::new();
    for path in files {
        let file = path
            .strip_prefix(&root)
            .map_err(|e| issue("content", "", e))?
            .to_str()
            .ok_or_else(|| issue("content", "", "non-UTF-8 filename"))?
            .to_owned();
        let matches: Vec<_> = mappings
            .iter_mut()
            .filter(|m| m.matcher.is_match(&file))
            .collect();
        if excluded.contains(&file) {
            if !matches.is_empty() {
                return Err(issue(
                    &file,
                    "",
                    "a file cannot be both mapped and excluded",
                ));
            }
            continue;
        }
        if matches.len() != 1 {
            return Err(issue(
                &file,
                "",
                "each JSON file must match exactly one mapping",
            ));
        }
        let mapping = matches
            .into_iter()
            .next()
            .ok_or_else(|| issue(&file, "", "missing mapping"))?;
        mapping.matched = true;
        let value = read_json(&path).map_err(|e| issue(&file, "", e))?;
        if matches!(mapping.mapping.schema_scope, SchemaScope::Document)
            && let Err(e) = mapping.validator.validate(&value)
        {
            return Err(issue(&file, e.instance_path().to_string(), e.to_string()));
        }
        let pointer = &mapping.mapping.records_pointer;
        let records = value
            .pointer(pointer)
            .and_then(Value::as_array)
            .ok_or_else(|| issue(&file, pointer, "records pointer must select an array"))?;
        for (index, record) in records.iter().enumerate() {
            if matches!(mapping.mapping.schema_scope, SchemaScope::Records)
                && let Err(e) = mapping.validator.validate(record)
            {
                return Err(issue(
                    &file,
                    format!("{pointer}/{index}{}", e.instance_path()),
                    e.to_string(),
                ));
            }
            let key_field = &mapping.mapping.key_field;
            let key_path = if key_field.starts_with('/') {
                key_field.clone()
            } else {
                format!("/{}", key_field.replace('~', "~0").replace('/', "~1"))
            };
            let key_pointer = format!("{pointer}/{index}{key_path}");
            let key = match record.pointer(&key_path) {
                Some(Value::String(s)) => s.clone(),
                Some(Value::Number(n)) if n.is_i64() || n.is_u64() => n.to_string(),
                _ => {
                    return Err(issue(
                        &file,
                        &key_pointer,
                        "key must be a string or integer",
                    ));
                }
            };
            validate_label(&key, "key").map_err(|e| issue(&file, &key_pointer, e))?;
            let identity = (mapping.mapping.kind.clone(), key.clone());
            if let Some(first_file) = key_sources.insert(identity, file.clone()) {
                return Err(issue(
                    &file,
                    &key_pointer,
                    format!("duplicate kind/key, first seen in {first_file}"),
                ));
            }
            let mut publishable = true;
            for hidden_pointer in &mapping.mapping.hidden_when_true {
                match record.pointer(hidden_pointer) {
                    Some(Value::Bool(true)) => publishable = false,
                    Some(Value::Bool(false)) | None => {}
                    Some(_) => {
                        return Err(issue(
                            &file,
                            format!("{pointer}/{index}{hidden_pointer}"),
                            "visibility veto must be boolean",
                        ));
                    }
                }
            }
            documents.push(Document {
                kind: mapping.mapping.kind.clone(),
                key,
                data: record.clone(),
                publishable,
            });
            if documents.len() > MAX_RECORDS {
                return Err(issue(&file, pointer, "import exceeds 100000 records"));
            }
        }
    }
    if documents.is_empty() {
        return Err(issue(
            "content",
            "",
            "empty imports cannot replace the active world",
        ));
    }
    if let Some(mapping) = mappings.iter().find(|m| !m.matched) {
        return Err(issue(
            &mapping.mapping.path_glob,
            "",
            "mapping matched no files",
        ));
    }
    Ok(documents)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn mapping() -> ImportMapping {
        ImportMapping {
            path_glob: "content/synthetic/*.json".into(),
            schema_file: "tools/content-schema/envelope.schema.json".into(),
            kind: "synthetic-entry".into(),
            key_field: "code".into(),
            records_pointer: "/payload".into(),
            schema_scope: SchemaScope::Document,
            hidden_when_true: Vec::new(),
        }
    }

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    fn copy_tree(source: &Path, target: &Path) {
        fs::create_dir_all(target).unwrap();
        for entry in fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &target.join(entry.file_name()));
            } else {
                fs::copy(entry.path(), target.join(entry.file_name())).unwrap();
            }
        }
    }

    #[test]
    fn synthetic_mapping_and_local_schema_references_validate() {
        let documents = validate(fixture("valid"), vec![mapping()]).unwrap();
        assert_eq!(documents.len(), 2);
        assert_eq!(documents[0].key, "fixture.alpha");
        assert_eq!(documents[0].kind, "synthetic-entry");
    }

    #[test]
    fn schema_failure_has_source_file_and_json_pointer() {
        let errors = validate(fixture("invalid"), vec![mapping()]).unwrap_err();
        assert_eq!(errors[0].file, "content/synthetic/entries.json");
        assert_eq!(errors[0].pointer, "/payload/0/weight");
    }

    #[test]
    fn malformed_json_duplicate_keys_and_empty_keys_are_refused() {
        for (json, pointer) in [
            ("{", ""),
            (
                r#"{"payload":[{"code":"a","weight":0},{"code":"a","weight":1}]}"#,
                "/payload/1/code",
            ),
            (r#"{"payload":[{"code":"","weight":0}]}"#, "/payload/0/code"),
        ] {
            let temp = tempfile::tempdir().unwrap();
            copy_tree(&fixture("valid"), temp.path());
            fs::write(temp.path().join("content/synthetic/entries.json"), json).unwrap();
            let errors = validate(temp.path().into(), vec![mapping()]).unwrap_err();
            assert_eq!(errors[0].pointer, pointer);
        }
    }

    #[test]
    fn absent_ambiguous_and_unmatched_mappings_are_refused() {
        for mappings in [
            vec![],
            vec![mapping(), mapping()],
            vec![ImportMapping {
                path_glob: "content/absent/*.json".into(),
                ..mapping()
            }],
        ] {
            assert!(validate(fixture("valid"), mappings).is_err());
        }
        let temp = tempfile::tempdir().unwrap();
        copy_tree(&fixture("valid"), temp.path());
        fs::write(temp.path().join("content/unmapped.json"), "{}").unwrap();
        assert!(validate(temp.path().into(), vec![mapping()]).is_err());
    }

    #[test]
    fn traversal_invalid_pointers_and_non_array_extraction_are_refused() {
        for m in [
            ImportMapping {
                schema_file: "../outside".into(),
                ..mapping()
            },
            ImportMapping {
                path_glob: "content/../*.json".into(),
                ..mapping()
            },
            ImportMapping {
                records_pointer: "/broken~pointer".into(),
                ..mapping()
            },
            ImportMapping {
                records_pointer: "".into(),
                ..mapping()
            },
        ] {
            assert!(validate(fixture("valid"), vec![m]).is_err());
        }
    }

    #[test]
    fn network_schema_references_are_refused() {
        let temp = tempfile::tempdir().unwrap();
        copy_tree(&fixture("valid"), temp.path());
        fs::write(
            temp.path()
                .join("tools/content-schema/envelope.schema.json"),
            r#"{"$ref":"https://example.invalid/schema"}"#,
        )
        .unwrap();
        assert!(validate(temp.path().into(), vec![mapping()]).is_err());
    }

    #[test]
    fn copied_game_shard_validates_with_original_schema_and_nested_keys() {
        let config: ImportConfig =
            serde_json::from_str(include_str!("../config/game-857f15f3.json")).unwrap();
        let mapping = config
            .mappings
            .into_iter()
            .find(|m| m.path_glob == "content/world/areas/cities/areas-*.json")
            .unwrap();
        let config = ImportConfig {
            mappings: vec![mapping],
            excluded_files: Vec::new(),
            schema_resources: config
                .schema_resources
                .into_iter()
                .filter(|(uri, _)| uri == "urn:oteryn:monster-authoring:candidate:1")
                .collect(),
        };
        let documents = validate_config(fixture("game"), config).unwrap();
        assert_eq!(documents.len(), 22);
        assert!(documents[0].key.starts_with("oteryn:content.area.city."));
    }

    #[test]
    fn exact_exclusions_are_explicit_and_cannot_override_mappings() {
        let temp = tempfile::tempdir().unwrap();
        copy_tree(&fixture("valid"), temp.path());
        fs::write(temp.path().join("content/sidecar.json"), "{}").unwrap();
        let config = ImportConfig {
            mappings: vec![mapping()],
            excluded_files: vec!["content/sidecar.json".into()],
            ..ImportConfig::default()
        };
        assert_eq!(
            validate_config(temp.path().into(), config.clone())
                .unwrap()
                .len(),
            2
        );
        let mut conflict = config;
        conflict
            .excluded_files
            .push("content/synthetic/entries.json".into());
        assert!(validate_config(temp.path().into(), conflict).is_err());
    }

    #[test]
    fn record_schema_scope_and_source_visibility_veto_are_configurable() {
        let temp = tempfile::tempdir().unwrap();
        copy_tree(&fixture("valid"), temp.path());
        let records_path = temp.path().join("content/synthetic/entries.json");
        fs::write(&records_path, r#"{"payload":[{"code":"hidden","weight":1,"hidden":true},{"code":"shown","weight":2}]}"#).unwrap();
        let schema_path = temp.path().join("tools/content-schema/record.schema.json");
        let mut schema = read_json(&schema_path).unwrap();
        schema["properties"]["hidden"] = serde_json::json!({"type":"boolean"});
        fs::write(&schema_path, serde_json::to_vec(&schema).unwrap()).unwrap();
        let mapping = ImportMapping {
            schema_file: "tools/content-schema/record.schema.json".into(),
            schema_scope: SchemaScope::Records,
            hidden_when_true: vec!["/hidden".into()],
            ..mapping()
        };
        let documents = validate(temp.path().into(), vec![mapping.clone()]).unwrap();
        assert!(!documents[0].publishable);
        assert!(documents[1].publishable);
        fs::write(
            &records_path,
            r#"{"payload":[{"code":"bad","weight":"wrong"}]}"#,
        )
        .unwrap();
        let errors = validate(temp.path().into(), vec![mapping]).unwrap_err();
        assert_eq!(errors[0].pointer, "/payload/0/weight");
    }
}
