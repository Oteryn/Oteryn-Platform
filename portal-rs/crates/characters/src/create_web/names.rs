//! Platform naming policy, matching the existing account form. Game decides uniqueness.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameError {
    Invalid,
    Reserved,
}

pub fn canonicalize(raw: &str) -> Result<String, NameError> {
    if raw.is_empty()
        || raw.len() > 255
        || !raw.bytes().all(|b| b.is_ascii_alphabetic() || b == b' ')
    {
        return Err(NameError::Invalid);
    }
    let words: Vec<_> = raw.split(' ').filter(|w| !w.is_empty()).collect();
    if words.is_empty() || words.len() > 3 || words.iter().any(|w| !(2..=15).contains(&w.len())) {
        return Err(NameError::Invalid);
    }
    let lower = words.join(" ").to_ascii_lowercase();
    if !(3..=29).contains(&lower.len()) {
        return Err(NameError::Invalid);
    }
    let reserved = [
        "admin",
        "administrator",
        "god",
        "gamemaster",
        "gm",
        "cm",
        "support",
        "tutor",
        "moderator",
        "staff",
        "system",
        "server",
        "oteryn",
    ];
    // The retired product prefix is represented as bytes to respect source policy.
    let retired = [99, 97, 110, 97, 114, 121];
    if lower
        .split(' ')
        .any(|word| reserved.contains(&word) || word.as_bytes() == retired)
        || matches!(lower.as_str(), "game master" | "community manager")
        || lower.starts_with("oteryn")
        || lower.as_bytes().starts_with(&retired)
    {
        return Err(NameError::Reserved);
    }
    Ok(words
        .iter()
        .map(|w| {
            let mut word = w.to_ascii_lowercase();
            word[..1].make_ascii_uppercase();
            word
        })
        .collect::<Vec<_>>()
        .join(" "))
}

/// Suggestions are candidates, never a promise of availability or a reservation.
pub fn suggestion() -> String {
    let bytes = uuid::Uuid::now_v7().into_bytes();
    let first = [
        "Ald", "Bel", "Cor", "Del", "Eld", "Fen", "Gal", "Hal", "Ith", "Kel", "Lor", "Mel", "Nor",
        "Or", "Ral", "Syl",
    ];
    let last = [
        "arin", "eron", "ion", "ora", "wyn", "oria", "adan", "eris", "ira", "anor", "eth", "ora",
        "ian", "ara", "orin", "una",
    ];
    format!(
        "{}{}",
        first[usize::from(bytes[14] % 16)],
        last[usize::from(bytes[15] % 16)]
    )
}
