//! Closed private journal codecs. No request can establish its own authority.
use crate::canonical::{self, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Unavailable,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "journal schema rejected: {self:?}")
    }
}
impl std::error::Error for Error {}
type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    pub authority: String,
    pub account: String,
    pub world: Option<String>,
    pub character: Option<String>,
    pub character_revision: Option<String>,
    pub projection_revision: Option<String>,
    pub from_projection_revision: Option<String>,
    pub epoch: Option<String>,
    pub recovery: String,
    pub family: String,
}
#[derive(Debug, Clone)]
pub struct Record {
    pub scope: Scope,
    pub revision: u64,
    pub id: String,
    pub entity: String,
    pub kind: String,
    pub occurred_ms: u64,
    pub coverage_ms: u64,
    pub payload: Value,
    pub bytes: Vec<u8>,
    pub progress: bool,
}
#[derive(Debug, Clone)]
pub struct Watermark {
    pub authority: String,
    pub account: String,
    pub epoch: String,
    pub recovery: String,
    pub family: String,
    pub through: u64,
    pub observed_ms: u64,
    pub baseline: bool,
    pub bytes: Vec<u8>,
}
fn require(ok: bool) -> Result<()> {
    if ok { Ok(()) } else { Err(Error::Invalid) }
}
pub fn fields<'a>(v: &'a Value, names: &str) -> Result<&'a [(String, Value)]> {
    let Value::Object(rows) = v else {
        return Err(Error::Invalid);
    };
    require(
        rows.iter()
            .map(|(k, _)| k.as_str())
            .eq(names.split_whitespace()),
    )?;
    Ok(rows)
}
pub fn get<'a>(v: &'a Value, k: &str) -> Result<&'a Value> {
    v.get(k).ok_or(Error::Invalid)
}
pub fn text(v: &Value) -> Result<&str> {
    if let Value::String(s) = v {
        Ok(s)
    } else {
        Err(Error::Invalid)
    }
}
fn s<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    text(get(v, k)?)
}
fn null(v: &Value) -> bool {
    matches!(v, Value::Null)
}
fn boolean(v: &Value) -> Result<bool> {
    if let Value::Bool(b) = v {
        Ok(*b)
    } else {
        Err(Error::Invalid)
    }
}
fn integer(v: &Value, min: u64, max: u64) -> Result<u64> {
    let n = match v {
        Value::Unsigned(n) => *n,
        Value::Signed(n) if *n >= 0 => *n as u64,
        _ => return Err(Error::Invalid),
    };
    require((min..=max).contains(&n))?;
    Ok(n)
}
pub fn decimal(v: &Value, min: u64, max: u64) -> Result<u64> {
    let t = text(v)?;
    require(
        !t.is_empty() && (t == "0" || !t.starts_with('0')) && t.bytes().all(|b| b.is_ascii_digit()),
    )?;
    let n = t.parse::<u64>().map_err(|_| Error::Invalid)?;
    require((min..=max).contains(&n))?;
    Ok(n)
}
fn dec(v: &Value, k: &str, min: u64, max: u64) -> Result<u64> {
    decimal(get(v, k)?, min, max)
}
fn num(v: &Value, k: &str, min: u64, max: u64) -> Result<u64> {
    integer(get(v, k)?, min, max)
}
fn uuid(t: &str) -> Result<()> {
    require(
        t.len() == 36
            && t.bytes().enumerate().all(|(i, b)| {
                if [8, 13, 18, 23].contains(&i) {
                    b == b'-'
                } else {
                    b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
                }
            })
            && &t[14..15] == "7"
            && matches!(t.as_bytes()[19], b'8' | b'9' | b'a' | b'b'),
    )
}
fn id(v: &Value, k: &str) -> Result<String> {
    let t = s(v, k)?;
    uuid(t)?;
    Ok(t.to_owned())
}
fn opt_id(v: &Value, k: &str) -> Result<Option<String>> {
    if null(get(v, k)?) {
        Ok(None)
    } else {
        id(v, k).map(Some)
    }
}
fn opt_dec(v: &Value, k: &str) -> Result<Option<String>> {
    if null(get(v, k)?) {
        Ok(None)
    } else {
        dec(v, k, 1, u64::MAX)?;
        Ok(Some(s(v, k)?.to_owned()))
    }
}
fn key(t: &str, max: usize) -> Result<()> {
    require(
        !t.is_empty()
            && t.len() <= max
            && t.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b)),
    )
}
fn progress_key(t: &str, max: usize) -> Result<()> {
    key(t, max)?;
    require(t.as_bytes()[0].is_ascii_alphanumeric())
}
fn revision(v: &Value, k: &str) -> Result<()> {
    let t = s(v, k)?;
    require(
        !t.is_empty()
            && t.as_bytes()[0].is_ascii_alphanumeric()
            && t.len() <= 128
            && t.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:/+-".contains(&b)),
    )
}
fn hex(t: &str, len: usize) -> Result<()> {
    require(
        t.len() == len
            && t.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
    )
}
fn array(v: &Value, min: usize, max: usize) -> Result<&[Value]> {
    let Value::Array(a) = v else {
        return Err(Error::Invalid);
    };
    require((min..=max).contains(&a.len()))?;
    Ok(a)
}
fn ordered<'a>(a: &'a [Value], k: &str) -> Result<Vec<&'a str>> {
    let keys = a.iter().map(|v| s(v, k)).collect::<Result<Vec<_>>>()?;
    require(keys.windows(2).all(|w| w[0] < w[1]))?;
    Ok(keys)
}
fn one(t: &str, options: &str) -> Result<()> {
    require(options.split_whitespace().any(|x| x == t))
}
fn time(v: &Value, k: &str) -> Result<u64> {
    dec(v, k, 0, 253402300799999)
}
fn maybe_time(v: &Value, k: &str) -> Result<Option<u64>> {
    if null(get(v, k)?) {
        Ok(None)
    } else {
        time(v, k).map(Some)
    }
}
fn decode(raw: &[u8], max: usize) -> Result<Value> {
    let v = canonical::decode(raw, max, 9).map_err(|_| Error::Invalid)?;
    require(!raw.contains(&b'\\') && canonical::encode(&v).map_err(|_| Error::Invalid)? == raw)?;
    Ok(v)
}
pub const PROGRESS_FAMILIES: &str = "bestiary boss_cooldown quest achievement skill_history death_recap pvp skull stamina premium_end game_login";
pub fn progress(raw: &[u8]) -> Result<Record> {
    let v = decode(raw, 262144)?;
    fields(
        &v,
        "contract_version operation source_authority account_id character_id world_id projection_revision projection_epoch recovery_generation family source_revision record_id entity_key character_revision occurred_at_unix_ms record_kind payload",
    )?;
    num(&v, "contract_version", 1, 1)?;
    require(s(&v, "operation")? == "PublishGameJournalProgressV1")?;
    let scope = progress_scope(&v)?;
    let entity = s(&v, "entity_key")?.to_owned();
    progress_key(&entity, 512)?;
    let kind = s(&v, "record_kind")?.to_owned();
    one(&kind, "UPSERT REMOVE")?;
    let payload = get(&v, "payload")?.clone();
    if kind == "REMOVE" {
        require(null(&payload))?;
        match scope.family.as_str() {
            "stamina" | "skull" => require(entity == scope.family)?,
            "premium_end" => require(entity == "premium")?,
            "skill_history" | "death_recap" | "pvp" | "game_login" => uuid(&entity)?,
            _ => (),
        }
    } else {
        progress_payload(&scope, &entity, &payload)?;
    }
    Ok(Record {
        revision: dec(&v, "source_revision", 1, u64::MAX)?,
        id: id(&v, "record_id")?,
        entity,
        kind,
        occurred_ms: time(&v, "occurred_at_unix_ms")?,
        coverage_ms: 0,
        payload,
        bytes: raw.to_vec(),
        scope,
        progress: true,
    })
}
fn progress_scope(v: &Value) -> Result<Scope> {
    let family = s(v, "family")?.to_owned();
    if family == "house_rent" {
        return Err(Error::Unavailable);
    };
    one(&family, PROGRESS_FAMILIES)?;
    let authority = s(v, "source_authority")?.to_owned();
    key(&authority, 128)?;
    let character = opt_id(v, "character_id")?;
    let world = opt_id(v, "world_id")?;
    let cr = opt_dec(v, "character_revision")?;
    let pr = opt_dec(v, "projection_revision")?;
    require(character.is_some() == world.is_some() && character.is_some() == cr.is_some())?;
    if character.is_none() {
        one(&family, "achievement premium_end game_login")?;
        require(pr.is_some() == (family == "premium_end"))?;
    } else {
        require(pr.is_some() && family != "premium_end")?;
    }
    dec(v, "projection_epoch", 1, u64::MAX)?;
    dec(v, "recovery_generation", 1, u64::MAX)?;
    Ok(Scope {
        authority,
        account: id(v, "account_id")?,
        world,
        character,
        character_revision: cr,
        projection_revision: pr,
        from_projection_revision: None,
        epoch: Some(s(v, "projection_epoch")?.to_owned()),
        recovery: s(v, "recovery_generation")?.to_owned(),
        family,
    })
}
pub fn watermark(raw: &[u8]) -> Result<Watermark> {
    let v = decode(raw, 1024)?;
    fields(
        &v,
        "contract_version operation source_authority account_id projection_epoch recovery_generation family complete_through observed_at_unix_ms baseline_complete",
    )?;
    num(&v, "contract_version", 1, 1)?;
    require(s(&v, "operation")? == "PublishGameJournalProgressWatermarkV1")?;
    key(s(&v, "source_authority")?, 128)?;
    one(s(&v, "family")?, PROGRESS_FAMILIES)?;
    dec(&v, "projection_epoch", 1, u64::MAX)?;
    dec(&v, "recovery_generation", 1, u64::MAX)?;
    Ok(Watermark {
        authority: s(&v, "source_authority")?.to_owned(),
        account: id(&v, "account_id")?,
        epoch: s(&v, "projection_epoch")?.to_owned(),
        recovery: s(&v, "recovery_generation")?.to_owned(),
        family: s(&v, "family")?.to_owned(),
        through: dec(&v, "complete_through", 0, u64::MAX)?,
        observed_ms: time(&v, "observed_at_unix_ms")?,
        baseline: boolean(get(&v, "baseline_complete")?)?,
        bytes: raw.to_vec(),
    })
}
fn progress_payload(scope: &Scope, entity: &str, p: &Value) -> Result<()> {
    let subject = scope.character.is_some();
    match scope.family.as_str() {
        "bestiary" => {
            fields(
                p,
                "race_key race_definition_revision kill_count final_kill_threshold charm_points_awarded",
            )?;
            require(s(p, "race_key")? == entity)?;
            progress_key(entity, 512)?;
            revision(p, "race_definition_revision")?;
            num(p, "kill_count", 0, u32::MAX as u64)?;
            num(p, "final_kill_threshold", 1, u32::MAX as u64)?;
            num(p, "charm_points_awarded", 0, u32::MAX as u64)?;
        }
        "boss_cooldown" => {
            fields(
                p,
                "boss_key definition_revision kill_count available_at_unix_ms last_death_occurrence_id",
            )?;
            require(s(p, "boss_key")? == entity)?;
            revision(p, "definition_revision")?;
            dec(p, "kill_count", 0, u64::MAX)?;
            maybe_time(p, "available_at_unix_ms")?;
            opt_id(p, "last_death_occurrence_id")?;
        }
        "quest" => {
            fields(
                p,
                "quest_key pinned_content_revision definition_hash completed tracks",
            )?;
            require(s(p, "quest_key")? == entity)?;
            revision(p, "pinned_content_revision")?;
            hex(s(p, "definition_hash")?, 64)?;
            boolean(get(p, "completed")?)?;
            let a = array(get(p, "tracks")?, 0, 256)?;
            ordered(a, "track_key")?;
            for t in a {
                fields(t, "track_key value")?;
                progress_key(s(t, "track_key")?, 512)?;
                let value = s(t, "value")?;
                let parsed = value.parse::<i64>().map_err(|_| Error::Invalid)?;
                require(parsed.to_string() == value)?;
            }
        }
        "achievement" => {
            fields(
                p,
                "achievement_key definition_revision progress target granted_at_unix_ms",
            )?;
            require(s(p, "achievement_key")? == entity)?;
            let tail = entity
                .strip_prefix("oteryn:achievement/")
                .ok_or(Error::Invalid)?;
            require(
                entity.len() <= 160
                    && !tail.is_empty()
                    && tail.split('_').all(|part| {
                        !part.is_empty()
                            && part
                                .bytes()
                                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
                    }),
            )?;
            revision(p, "definition_revision")?;
            if subject {
                let n = num(p, "progress", 0, 100000)?;
                require(n <= num(p, "target", 1, 100000)? && null(get(p, "granted_at_unix_ms")?))?;
            } else {
                require(null(get(p, "progress")?) && null(get(p, "target")?))?;
                time(p, "granted_at_unix_ms")?;
            }
        }
        "skill_history" => {
            fields(p, "build_occurrence_id cause policy_revision skills")?;
            require(id(p, "build_occurrence_id")? == entity)?;
            one(
                s(p, "cause")?,
                "training vocation_choice promotion offline_settlement logout death",
            )?;
            revision(p, "policy_revision")?;
            let a = array(get(p, "skills")?, 1, 8)?;
            ordered(a, "skill")?;
            for t in a {
                fields(
                    t,
                    "skill level_before progress_before level_after progress_after",
                )?;
                let skill = s(t, "skill")?;
                one(
                    skill,
                    "magic fist club sword axe distance shielding fishing",
                )?;
                let min = if skill == "magic" { 0 } else { 10 };
                num(t, "level_before", min, 1000)?;
                num(t, "level_after", min, 1000)?;
                dec(t, "progress_before", 0, i64::MAX as u64)?;
                dec(t, "progress_after", 0, i64::MAX as u64)?;
            }
        }
        "death_recap" => {
            fields(
                p,
                "death_occurrence_id level_before level_after experience_lost blessings_before blessings_after lost_items recap_state window_ms actions",
            )?;
            require(id(p, "death_occurrence_id")? == entity)?;
            require(
                num(p, "level_after", 1, u32::MAX as u64)?
                    <= num(p, "level_before", 1, u32::MAX as u64)?,
            )?;
            dec(p, "experience_lost", 0, i64::MAX as u64)?;
            let mut sets = Vec::new();
            for name in ["blessings_before", "blessings_after"] {
                let a = array(get(p, name)?, 0, 32)?;
                let values = a.iter().map(text).collect::<Result<Vec<_>>>()?;
                require(values.windows(2).all(|w| w[0] < w[1]))?;
                for t in &values {
                    progress_key(t, 128)?;
                }
                sets.push(values);
            }
            require(sets[1].iter().all(|x| sets[0].contains(x)))?;
            let mut last = None;
            for t in array(get(p, "lost_items")?, 0, 256)? {
                fields(t, "loss_index item_key quantity")?;
                let n = num(t, "loss_index", 0, 255)?;
                require(last.is_none_or(|l| l < n))?;
                last = Some(n);
                progress_key(s(t, "item_key")?, 512)?;
                num(t, "quantity", 1, 65535)?;
            }
            require(s(p, "recap_state")? == "UNAVAILABLE")?;
            num(p, "window_ms", 0, 0)?;
            array(get(p, "actions")?, 0, 0)?;
        }
        "pvp" => {
            fields(
                p,
                "death_occurrence_id role counterpart_character_id justification points_milli",
            )?;
            require(id(p, "death_occurrence_id")? == entity)?;
            one(s(p, "role")?, "KILL ASSIST VICTIM")?;
            opt_id(p, "counterpart_character_id")?;
            let j = s(p, "justification")?;
            one(j, "JUSTIFIED UNJUSTIFIED UNKNOWN")?;
            if j == "UNKNOWN" {
                require(null(get(p, "points_milli")?))?;
            } else {
                num(
                    p,
                    "points_milli",
                    0,
                    if j == "JUSTIFIED" { 0 } else { 65535 },
                )?;
            }
        }
        "skull" => {
            fields(p, "skull skull_until_unix_ms kill_block_until_unix_ms")?;
            require(entity == "skull")?;
            let k = s(p, "skull")?;
            one(k, "NONE RED BLACK")?;
            require(maybe_time(p, "skull_until_unix_ms")?.is_none() == (k == "NONE"))?;
            maybe_time(p, "kill_block_until_unix_ms")?;
        }
        "stamina" => {
            fields(p, "stamina_minutes content_revision build_occurrence_id")?;
            require(entity == "stamina")?;
            num(p, "stamina_minutes", 0, 2520)?;
            dec(p, "content_revision", 1, u64::MAX)?;
            id(p, "build_occurrence_id")?;
        }
        "premium_end" => {
            fields(
                p,
                "observed_state effective_until_unix_us authority_revision",
            )?;
            require(entity == "premium")?;
            one(
                s(p, "observed_state")?,
                "ACTIVE NOT_YET_EFFECTIVE EXPIRED REVOKED NONE",
            )?;
            if !null(get(p, "effective_until_unix_us")?) {
                dec(p, "effective_until_unix_us", 0, 253402300799999999)?;
            }
            dec(p, "authority_revision", 1, u64::MAX)?;
            require(scope.projection_revision.as_deref() == Some(s(p, "authority_revision")?))?;
        }
        "game_login" => {
            fields(p, "login_event_id entered_at left_at end_reason")?;
            require(id(p, "login_event_id")? == entity)?;
            let start = time(p, "entered_at")?;
            if let Some(end) = maybe_time(p, "left_at")? {
                require(end >= start)?;
                one(s(p, "end_reason")?, "LOGOUT DISCONNECT REVOKED UNKNOWN")?;
            } else {
                require(null(get(p, "end_reason")?))?;
            }
        }
        _ => return Err(Error::Unavailable),
    };
    Ok(())
}

pub fn items_scope(v: &Value) -> Result<Scope> {
    let family = s(v, "family")?.to_owned();
    one(&family, "ITEMS BANK MARKET PLAYER_TRADE")?;
    let authority = s(v, "source_authority")?.to_owned();
    key(&authority, 128)?;
    let character = opt_id(v, "character_id")?;
    let cr = opt_dec(v, "character_revision")?;
    let from = opt_dec(v, "from_projection_revision")?;
    let to = opt_dec(v, "to_projection_revision")?;
    let epoch = opt_dec(v, "projection_epoch")?;
    let char_scope = family != "BANK";
    require(
        [
            character.is_some(),
            cr.is_some(),
            from.is_some(),
            to.is_some(),
            epoch.is_some(),
        ]
        .iter()
        .all(|x| *x == char_scope),
    )?;
    dec(v, "recovery_generation", 1, u64::MAX)?;
    Ok(Scope {
        authority,
        account: id(v, "account_id")?,
        world: Some(id(v, "world_id")?),
        character,
        character_revision: cr,
        projection_revision: to,
        from_projection_revision: from,
        epoch,
        recovery: s(v, "recovery_generation")?.to_owned(),
        family,
    })
}
pub fn cursor(raw: &[u8]) -> Result<Scope> {
    let v = decode(raw, 1024)?;
    fields(
        &v,
        "contract_version operation source_authority account_id world_id character_id character_revision from_projection_revision to_projection_revision projection_epoch family recovery_generation",
    )?;
    num(&v, "contract_version", 1, 1)?;
    require(s(&v, "operation")? == "ReadGameJournalCursorV1")?;
    items_scope(&v)
}
pub fn items(raw: &[u8]) -> Result<Record> {
    let v = decode(raw, 65536)?;
    // G-9 revision 2 has not yet supplied a closed replacement schema. Never
    // reinterpret its revision-1 ownership/session fields as revision-2 evidence.
    if v.get("stream_generation").is_some() {
        return Err(Error::Unavailable);
    }
    fields(
        &v,
        "contract_version operation source_authority account_id world_id character_id character_revision from_projection_revision to_projection_revision projection_epoch family recovery_generation source_revision record_id occurred_at coverage_start_at kind payload",
    )?;
    num(&v, "contract_version", 1, 1)?;
    require(s(&v, "operation")? == "PublishGameJournalRecordV1")?;
    let scope = items_scope(&v)?;
    let p = get(&v, "payload")?.clone();
    let kind = s(&v, "kind")?.to_owned();
    let entity = items_payload(&scope, &kind, &p)?;
    let occurred = dec(&v, "occurred_at", 1, 253402300799)? * 1000;
    let coverage = dec(&v, "coverage_start_at", 1, 253402300799)? * 1000;
    require(coverage <= occurred && (scope.family != "ITEMS" || coverage == occurred))?;
    Ok(Record {
        scope,
        revision: dec(&v, "source_revision", 1, u64::MAX)?,
        id: id(&v, "record_id")?,
        entity,
        kind,
        occurred_ms: occurred,
        coverage_ms: coverage,
        payload: p,
        bytes: raw.to_vec(),
        progress: false,
    })
}
fn count_pair(p: &Value) -> Result<(u64, u64)> {
    let pages = num(p, "page_count", 0, 16384)?;
    let items = num(p, "item_count", 0, 524288)?;
    require((pages == 0 && items == 0) || (pages > 0 && pages <= items && items <= pages * 32))?;
    Ok((pages, items))
}
fn item_line(p: &Value, max: u64) -> Result<String> {
    fields(
        p,
        "item_instance_id definition_family definition_production_key definition_revision_ref quantity",
    )?;
    key(s(p, "definition_family")?, 128)?;
    key(s(p, "definition_production_key")?, 512)?;
    key(s(p, "definition_revision_ref")?, 512)?;
    num(p, "quantity", 1, max)?;
    id(p, "item_instance_id")
}
fn items_payload(scope: &Scope, kind: &str, p: &Value) -> Result<String> {
    match (scope.family.as_str(), kind) {
        ("BANK", "BANK_ENTRY") => {
            fields(
                p,
                "operation_occurrence_id event_id entry_id previous_entry_id kind direction amount balance_before balance_after",
            )?;
            id(p, "operation_occurrence_id")?;
            id(p, "event_id")?;
            let entry = id(p, "entry_id")?;
            opt_id(p, "previous_entry_id")?;
            let kind = s(p, "kind")?;
            one(kind, "DEPOSIT WITHDRAW TRANSFER")?;
            let direction = s(p, "direction")?;
            one(direction, "CREDIT DEBIT")?;
            require(
                (kind != "DEPOSIT" || direction == "CREDIT")
                    && (kind != "WITHDRAW" || direction == "DEBIT"),
            )?;
            let max = match kind {
                "DEPOSIT" => 20000000,
                "WITHDRAW" => 1009999,
                _ => 999999999999,
            };
            let amount = dec(p, "amount", 1, max)?;
            let before = dec(p, "balance_before", 0, 999999999999)?;
            let after = dec(p, "balance_after", 0, 999999999999)?;
            require(if direction == "CREDIT" {
                before.checked_add(amount) == Some(after)
            } else {
                before.checked_sub(amount) == Some(after)
            })?;
            Ok(entry)
        }
        ("MARKET", "MARKET_EVENT") => {
            fields(
                p,
                "event_id offer_id side state end_state definition_family definition_production_key definition_revision_ref amount remaining piece_price fee_gold held_credit_gold",
            )?;
            let event = id(p, "event_id")?;
            id(p, "offer_id")?;
            one(s(p, "side")?, "BUY SELL")?;
            let state = s(p, "state")?;
            one(state, "OPEN FILLED CANCELLED EXPIRED CREDIT_HELD")?;
            if state == "CREDIT_HELD" {
                one(s(p, "end_state")?, "FILLED CANCELLED EXPIRED")?;
            } else {
                require(null(get(p, "end_state")?))?;
            }
            key(s(p, "definition_family")?, 128)?;
            key(s(p, "definition_production_key")?, 512)?;
            key(s(p, "definition_revision_ref")?, 512)?;
            let amount = num(p, "amount", 1, 64000)?;
            let remaining = num(p, "remaining", 0, amount)?;
            let price = dec(p, "piece_price", 1, 999999999999)?;
            require(amount.checked_mul(price).is_some_and(|x| x <= 999999999999))?;
            if state == "FILLED" || (state == "CREDIT_HELD" && s(p, "end_state")? == "FILLED") {
                require(remaining == 0)?;
            }
            dec(p, "fee_gold", 0, 2500000)?;
            let held = dec(p, "held_credit_gold", 0, 999999999999)?;
            require(state == "CREDIT_HELD" || held == 0)?;
            Ok(event)
        }
        ("PLAYER_TRADE", "PLAYER_TRADE_EVENT") => {
            fields(p, "occurrence_id transaction_id event_id given received")?;
            id(p, "occurrence_id")?;
            id(p, "transaction_id")?;
            let event = id(p, "event_id")?;
            let given = item_line(&array(get(p, "given")?, 1, 1)?[0], 100)?;
            let received = item_line(&array(get(p, "received")?, 1, 1)?[0], 100)?;
            require(given != received)?;
            Ok(event)
        }
        ("ITEMS", "ITEMS_BEGIN") => {
            fields(
                p,
                "snapshot_id character_revision page_count item_count depots houses",
            )?;
            let snapshot = id(p, "snapshot_id")?;
            dec(p, "character_revision", 1, u64::MAX)?;
            require(scope.character_revision.as_deref() == Some(s(p, "character_revision")?))?;
            count_pair(p)?;
            let mut last = None;
            for v in array(get(p, "depots")?, 0, 256)? {
                let n = integer(v, 0, 65535)?;
                require(last.is_none_or(|l| l < n))?;
                last = Some(n);
            }
            let a = array(get(p, "houses")?, 0, 256)?;
            let mut previous = "";
            for v in a {
                let t = text(v)?;
                house(t)?;
                require(previous < t)?;
                previous = t;
            }
            Ok(snapshot)
        }
        ("ITEMS", "ITEMS_PAGE") => {
            fields(p, "snapshot_id page_index items")?;
            let snapshot = id(p, "snapshot_id")?;
            num(p, "page_index", 0, 16383)?;
            let a = array(get(p, "items")?, 1, 32)?;
            ordered(a, "item_instance_id")?;
            for row in a {
                item_row(row)?;
            }
            Ok(snapshot)
        }
        ("ITEMS", "ITEMS_END") => {
            fields(p, "snapshot_id page_count item_count pages_sha256")?;
            let snapshot = id(p, "snapshot_id")?;
            count_pair(p)?;
            hex(s(p, "pages_sha256")?, 64)?;
            Ok(snapshot)
        }
        _ => Err(Error::Invalid),
    }
}
fn house(t: &str) -> Result<()> {
    let tail = t
        .strip_prefix("oteryn:content.house.")
        .ok_or(Error::Invalid)?;
    require(
        t.len() <= 512
            && !tail.is_empty()
            && tail
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
    )
}
fn item_row(p: &Value) -> Result<()> {
    fields(
        p,
        "item_instance_id definition_family definition_production_key definition_revision_ref quantity root town_id depot_box house_id spatial_position_hex equipment_slot parent_item_instance_id placement_ordinal",
    )?;
    id(p, "item_instance_id")?;
    key(s(p, "definition_family")?, 128)?;
    key(s(p, "definition_production_key")?, 512)?;
    key(s(p, "definition_revision_ref")?, 512)?;
    num(p, "quantity", 1, u32::MAX as u64)?;
    let root = s(p, "root")?;
    one(root, "EQUIPMENT DEPOT INBOX HOUSE")?;
    opt_id(p, "parent_item_instance_id")?;
    dec(p, "placement_ordinal", 1, u64::MAX)?;
    for (field, active, min, max) in [
        ("town_id", root == "DEPOT", 0, 65535),
        ("depot_box", root == "DEPOT", 1, 17),
        ("equipment_slot", root == "EQUIPMENT", 1, 255),
    ] {
        if active {
            num(p, field, min, max)?;
        } else {
            require(null(get(p, field)?))?;
        }
    }
    if root == "HOUSE" {
        house(s(p, "house_id")?)?;
        let t = s(p, "spatial_position_hex")?;
        require((2..=256).contains(&t.len()) && t.len() % 2 == 0)?;
        hex(t, t.len())?;
        require(null(get(p, "parent_item_instance_id")?))?;
    } else {
        require(null(get(p, "house_id")?) && null(get(p, "spatial_position_hex")?))?;
    }
    Ok(())
}
