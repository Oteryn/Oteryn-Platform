//! Accepted G-9 revision 3. A separate closed codec; G-10 bytes are unchanged.
use super::*;

pub const ISSUER: &str = "OTERYN_PLATFORM_GAME_JOURNAL";
pub const AUDIENCE: &str = "OTERYN_GAME_GAME_JOURNAL";
#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub envelope: Value,
    pub authority: String,
    pub generation: String,
    pub account: String,
    pub revision: u64,
    pub kind: String,
    pub character: String,
    pub character_revision: u64,
    pub session: String,
    pub segment: String,
    pub business_id: String,
    pub bytes: Vec<u8>,
}
fn time(v: &Value, k: &str) -> Result<u64> {
    dec(v, k, 1, 253402300799)
}
fn catalog_key(t: &str) -> Result<()> {
    key(t, 512)?;
    let (namespace, name) = t.split_once(':').ok_or(Error::Invalid)?;
    require(
        !namespace.is_empty()
            && !name.is_empty()
            && namespace.as_bytes()[0].is_ascii_alphanumeric()
            && namespace
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_./-".contains(&b)),
    )
}
fn ck(v: &Value, k: &str) -> Result<()> {
    catalog_key(s(v, k)?)
}
fn counter(v: &Value, k: &str) -> Result<u64> {
    dec(v, k, 0, i64::MAX as u64)
}
fn positive_counter(v: &Value, k: &str) -> Result<u64> {
    dec(v, k, 1, i64::MAX as u64)
}
fn sorted(rows: &[Value], keys: &[&str]) -> Result<()> {
    let mut previous: Option<Vec<Option<String>>> = None;
    for row in rows {
        let tuple = keys
            .iter()
            .map(|k| {
                let value = get(row, k)?;
                if null(value) {
                    Ok(None)
                } else {
                    text(value).map(|s| Some(s.to_owned()))
                }
            })
            .collect::<Result<Vec<_>>>()?;
        require(previous.as_ref().is_none_or(|p| p < &tuple))?;
        previous = Some(tuple);
    }
    Ok(())
}
fn items(v: &Value) -> Result<()> {
    let rows = array(v, 0, 4096)?;
    sorted(rows, &["item_key"])?;
    for r in rows {
        fields(r, "item_key quantity")?;
        ck(r, "item_key")?;
        positive_counter(r, "quantity")?;
    }
    Ok(())
}
fn damage(v: &Value) -> Result<()> {
    let rows = array(v, 0, 8192)?;
    sorted(rows, &["damage_type_key", "source_kind", "source_key"])?;
    for r in rows {
        fields(r, "damage_type_key source_kind source_key amount")?;
        ck(r, "damage_type_key")?;
        one(
            s(r, "source_kind")?,
            "SELF MONSTER PLAYER ENVIRONMENT UNKNOWN",
        )?;
        if s(r, "source_kind")? == "MONSTER" {
            ck(r, "source_key")?;
        } else {
            require(null(get(r, "source_key")?))?;
        }
        positive_counter(r, "amount")?;
    }
    Ok(())
}
fn segment(r: &Value, party_accepted: bool) -> Result<()> {
    fields(
        r,
        "session_id segment_id segment_index character_id world_id channel_id character_revision catalog_revision started_at ended_at duration_seconds end_reason final_segment experience_gained experience_lost level vocation_key places loot supplies damage_dealt damage_taken healing_done healing_received monsters_killed rare_drop_count party",
    )?;
    id(r, "world_id")?;
    id(r, "channel_id")?;
    num(r, "segment_index", 0, 65535)?;
    let start = time(r, "started_at")?;
    let end = time(r, "ended_at")?;
    require(end >= start)?;
    let duration = counter(r, "duration_seconds")?;
    require(duration <= end - start)?;
    one(
        s(r, "end_reason")?,
        "AREA_EXIT KILL_IDLE_TIMEOUT CONTEXT_CHANGE SESSION_END RECOVERY_END",
    )?;
    require(boolean(get(r, "final_segment")?)? == (s(r, "end_reason")? == "SESSION_END"))?;
    for k in [
        "experience_gained",
        "experience_lost",
        "healing_done",
        "healing_received",
    ] {
        counter(r, k)?;
    }
    num(r, "level", 1, 65535)?;
    ck(r, "vocation_key")?;
    let places = array(get(r, "places")?, 0, 256)?;
    sorted(places, &["place_key"])?;
    let mut sum = 0u64;
    for p in places {
        fields(p, "place_key duration_seconds")?;
        if !null(get(p, "place_key")?) {
            ck(p, "place_key")?;
        }
        let n = counter(p, "duration_seconds")?;
        require(n <= duration)?;
        sum = sum.checked_add(n).ok_or(Error::Invalid)?;
    }
    require(sum == duration)?;
    items(get(r, "loot")?)?;
    items(get(r, "supplies")?)?;
    damage(get(r, "damage_dealt")?)?;
    damage(get(r, "damage_taken")?)?;
    let kills = array(get(r, "monsters_killed")?, 0, 4096)?;
    sorted(kills, &["monster_key"])?;
    for k in kills {
        fields(k, "monster_key count")?;
        ck(k, "monster_key")?;
        positive_counter(k, "count")?;
    }
    num(r, "rare_drop_count", 0, 65535)?;
    if start == end && s(r, "end_reason")? == "SESSION_END" {
        for k in [
            "experience_gained",
            "experience_lost",
            "healing_done",
            "healing_received",
        ] {
            require(counter(r, k)? == 0)?;
        }
        for k in [
            "places",
            "loot",
            "supplies",
            "damage_dealt",
            "damage_taken",
            "monsters_killed",
        ] {
            require(array(get(r, k)?, 0, 8192)?.is_empty())?;
        }
        require(num(r, "rare_drop_count", 0, 65535)? == 0)?;
    }

    let party = get(r, "party")?;
    if !null(party) {
        if !party_accepted {
            return Err(Error::Unavailable);
        }
        require(start != end || s(r, "end_reason")? != "SESSION_END")?;
        validate_party(party, get(r, "loot")?, duration)?;
    }
    Ok(())
}

fn validate_party(party: &Value, owner_loot: &Value, duration: u64) -> Result<()> {
    use std::collections::BTreeSet;

    fields(party, "party_id members loot")?;
    id(party, "party_id")?;
    let mut slots = BTreeSet::new();
    let mut subject = None;
    for member in array(get(party, "members")?, 1, 64)? {
        fields(member, "member_slot is_subject active_seconds")?;
        let slot = num(member, "member_slot", 0, 63)?;
        require(slots.last().is_none_or(|prior| *prior < slot))?;
        slots.insert(slot);
        require(counter(member, "active_seconds")? <= duration)?;
        if boolean(get(member, "is_subject")?)? {
            require(subject.replace(slot).is_none())?;
        }
    }
    let subject = subject.ok_or(Error::Invalid)?;
    let mut previous = None;
    let mut personal = Vec::new();
    for item in array(get(party, "loot")?, 0, 4096)? {
        fields(item, "member_slot item_key quantity")?;
        let slot = num(item, "member_slot", 0, 63)?;
        require(slots.contains(&slot))?;
        ck(item, "item_key")?;
        let key = s(item, "item_key")?;
        let quantity = positive_counter(item, "quantity")?;
        require(previous.is_none_or(|prior| prior < (slot, key)))?;
        previous = Some((slot, key));
        if slot == subject {
            personal.push((key, quantity));
        }
    }
    let expected = array(owner_loot, 0, 4096)?
        .iter()
        .map(|item| Ok((s(item, "item_key")?, positive_counter(item, "quantity")?)))
        .collect::<Result<Vec<_>>>()?;
    require(personal == expected)
}

fn drop_record(r: &Value) -> Result<()> {
    fields(
        r,
        "session_id segment_id event_id character_id character_revision occurred_at catalog_revision item_key quantity monster_key place_key rarity_rule_key",
    )?;
    id(r, "event_id")?;
    time(r, "occurred_at")?;
    ck(r, "item_key")?;
    positive_counter(r, "quantity")?;
    for k in ["monster_key", "place_key"] {
        if !null(get(r, k)?) {
            ck(r, k)?;
        }
    }
    ck(r, "rarity_rule_key")
}
pub fn record(raw: &[u8]) -> Result<SessionRecord> {
    record_with_party(raw, false)
}

/// Only the caller's accepted Game privacy configuration can admit party ledgers.
pub fn record_with_party(raw: &[u8], party_accepted: bool) -> Result<SessionRecord> {
    let v = decode(raw, 1048576)?;
    fields(
        &v,
        "contract_version operation source_authority stream_generation account_id source_revision record_kind record",
    )?;
    num(&v, "contract_version", 1, 1)?;
    require(s(&v, "operation")? == "PublishGameJournalRecordV1")?;
    key(s(&v, "source_authority")?, 128)?;
    let r = get(&v, "record")?;
    let kind = s(&v, "record_kind")?;
    match kind {
        "HUNT_SEGMENT" => segment(r, party_accepted)?,
        "RARE_DROP" => drop_record(r)?,
        _ => return Err(Error::Invalid),
    }
    key(s(r, "catalog_revision")?, 128)?;
    Ok(SessionRecord {
        authority: s(&v, "source_authority")?.to_owned(),
        generation: id(&v, "stream_generation")?,
        account: id(&v, "account_id")?,
        revision: dec(&v, "source_revision", 1, u64::MAX)?,
        kind: kind.to_owned(),
        character: id(r, "character_id")?,
        character_revision: dec(r, "character_revision", 1, u64::MAX)?,
        session: id(r, "session_id")?,
        segment: id(r, "segment_id")?,
        business_id: if kind == "HUNT_SEGMENT" {
            id(r, "segment_id")?
        } else {
            id(r, "event_id")?
        },
        envelope: v,
        bytes: raw.to_vec(),
    })
}
#[derive(Debug, Clone, Copy)]
pub enum Exchange {
    Pending,
    Read,
    Receipt,
}
pub fn exchange(raw: &[u8], kind: Exchange) -> Result<Value> {
    let (max, operation, tail) = match kind {
        Exchange::Pending => (
            512,
            "ListPendingGameJournalCommandsV1",
            "after_source_revision after_operation_id max_entries",
        ),
        Exchange::Read => (
            512,
            "ReadGameJournalCommandIntentV1",
            "issuer_authority operation_id",
        ),
        Exchange::Receipt => (
            1024,
            "PublishGameJournalCommandReceiptV1",
            "issuer_authority operation_id binding_digest outcome result_code decided_at",
        ),
    };
    let v = decode(raw, max)?;
    fields(
        &v,
        &format!("contract_version operation source_authority {tail}"),
    )?;
    num(&v, "contract_version", 1, 1)?;
    require(s(&v, "operation")? == operation)?;
    key(s(&v, "source_authority")?, 128)?;
    match kind {
        Exchange::Pending => {
            let cursor = dec(&v, "after_source_revision", 0, u64::MAX)?;
            let after = s(&v, "after_operation_id")?;
            require((cursor == 0) == (after == "00000000-0000-0000-0000-000000000000"))?;
            if cursor > 0 {
                uuid(after)?;
            }
            num(&v, "max_entries", 1, 32)?;
        }
        Exchange::Read | Exchange::Receipt => {
            require(s(&v, "issuer_authority")? == ISSUER)?;
            id(&v, "operation_id")?;
            if matches!(kind, Exchange::Receipt) {
                let hash = s(&v, "binding_digest")?;
                require(
                    hash.len() == 64
                        && hash
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                )?;
                match s(&v, "outcome")? {
                    "COMMITTED" => require(s(&v, "result_code")? == "JOURNAL_REPLAY_QUEUED")?,
                    "REJECTED" => one(
                        s(&v, "result_code")?,
                        "JOURNAL_RANGE_UNAVAILABLE JOURNAL_OWNER_MISMATCH JOURNAL_GENERATION_MISMATCH JOURNAL_INTENT_EXPIRED JOURNAL_UNSUPPORTED",
                    )?,
                    _ => return Err(Error::Invalid),
                }
                time(&v, "decided_at")?;
            }
        }
    }
    Ok(v)
}
pub fn intent(raw: &[u8]) -> Result<Value> {
    let v = decode(raw, 4096)?;
    fields(
        &v,
        "contract_version command issuer_authority audience source_revision operation_id account_id stream_generation from_revision through_revision issued_at expires_at",
    )?;
    num(&v, "contract_version", 1, 1)?;
    require(
        s(&v, "command")? == "ReplayGameJournalRecords"
            && s(&v, "issuer_authority")? == ISSUER
            && s(&v, "audience")? == AUDIENCE,
    )?;
    dec(&v, "source_revision", 1, u64::MAX)?;
    for k in ["operation_id", "account_id", "stream_generation"] {
        id(&v, k)?;
    }
    let from = dec(&v, "from_revision", 1, u64::MAX)?;
    let through = dec(&v, "through_revision", from, u64::MAX)?;
    require(
        through - from < 10000
            && time(&v, "expires_at")?
                == time(&v, "issued_at")?
                    .checked_add(300)
                    .ok_or(Error::Invalid)?,
    )?;
    Ok(v)
}
