//! Strict compatibility aliases. Every explicitly supplied flag must enable its gate.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Flags {
    pub viewer: bool,
    pub delivery: bool,
    pub world: bool,
}
fn gate(primary: Option<&str>, legacy: Option<&str>) -> bool {
    (primary.is_some() || legacy.is_some())
        && primary.is_none_or(|v| v == "true")
        && legacy.is_none_or(|v| v == "1")
}
impl Flags {
    pub fn configured() -> Self {
        let read = |primary: &str, legacy: &str| {
            gate(
                std::env::var(primary).ok().as_deref(),
                std::env::var(legacy).ok().as_deref(),
            )
        };
        Self {
            viewer: std::env::var("OTERYN_ATLAS_ENABLED").as_deref() == Ok("true"),
            delivery: read(
                "OTERYN_ATLAS_DELIVERY_ENABLED",
                "PORTAL_ATLAS_DELIVERY_ENABLED",
            ),
            world: read("OTERYN_WORLD_ENABLED", "PORTAL_WORLD_ENABLED"),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_explicit_flags_must_enable_the_gate() {
        for (p, l, expected) in [
            (None, None, false),
            (Some("true"), None, true),
            (None, Some("1"), true),
            (Some("true"), Some("1"), true),
            (Some("false"), Some("1"), false),
            (Some("true"), Some("0"), false),
            (Some("1"), None, false),
            (None, Some("true"), false),
        ] {
            assert_eq!(gate(p, l), expected);
        }
    }
}
