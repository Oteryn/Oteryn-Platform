use super::Error;
use oteryn_internal_api::canonical::{self, Shape, Value};
use uuid::Uuid;
pub(super) static SHAPE: Shape = Shape::Object(&[
    ("contract_version", Shape::Scalar),
    ("operation", Shape::Scalar),
    ("source_authority", Shape::Scalar),
    ("restore_event_id", Shape::Scalar),
    ("restored_generation", Shape::Scalar),
    ("new_recovery_generation", Shape::Scalar),
    ("restore_point", Shape::Scalar),
    ("backup_label", Shape::Scalar),
]);
pub(super) struct Notice {
    pub id: Uuid,
    pub source: String,
    pub restored: String,
    pub new_generation: String,
}
pub(super) fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str, Error> {
    match v.get(key) {
        Some(Value::String(s)) => Ok(s),
        _ => Err(Error::BadInput),
    }
}
pub(super) fn counter(s: &str) -> Result<u64, Error> {
    if s.is_empty() || s.len() > 20 || s.starts_with('0') || !s.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(Error::BadInput);
    }
    s.parse().map_err(|_| Error::BadInput)
}
pub(super) fn authority(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}
pub(super) fn decode(raw: &[u8]) -> Result<Notice, Error> {
    let v = canonical::decode_canonical(raw, 512, 4, &SHAPE).map_err(|_| Error::BadInput)?;
    if v.get("contract_version") != Some(&Value::Unsigned(1))
        || text(&v, "operation")? != "PublishGameRestoreNoticeV1"
    {
        return Err(Error::BadInput);
    }
    let source = text(&v, "source_authority")?;
    if !authority(source) {
        return Err(Error::BadInput);
    }
    let id_raw = text(&v, "restore_event_id")?;
    let id = Uuid::parse_str(id_raw).map_err(|_| Error::BadInput)?;
    if id.get_version_num() != 7
        || id.get_variant() != uuid::Variant::RFC4122
        || id.to_string() != id_raw
    {
        return Err(Error::BadInput);
    }
    let restored = counter(text(&v, "restored_generation")?)?;
    let new_generation = counter(text(&v, "new_recovery_generation")?)?;
    counter(text(&v, "restore_point")?)?;
    let label = text(&v, "backup_label")?;
    if new_generation <= restored
        || label.is_empty()
        || label.len() > 128
        || !label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
    {
        return Err(Error::BadInput);
    }
    Ok(Notice {
        id,
        source: source.into(),
        restored: format!("{restored:020}"),
        new_generation: format!("{new_generation:020}"),
    })
}
pub(super) fn status(id: Uuid, reopened: i64, outstanding: i64) -> String {
    let status = if outstanding == 0 {
        "ACKNOWLEDGED"
    } else {
        "RECONCILING"
    };
    format!(
        "{{\"contract_version\":1,\"restore_event_id\":\"{id}\",\"status\":\"{status}\",\"reopened\":{reopened},\"outstanding\":{outstanding}}}"
    )
}
