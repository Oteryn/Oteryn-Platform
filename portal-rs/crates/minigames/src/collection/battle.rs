use super::*;
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Strike,
    Guard,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Active,
    Victory,
    Defeat,
    Draw,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Fighter {
    pub card: Uuid,
    pub health: u16,
}
/// Kept server-side. The browser receives a view without RNG state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Battle {
    pub ruleset: u8,
    pub turn: u32,
    pub player: Vec<Fighter>,
    pub enemy: Vec<Fighter>,
    pub outcome: Outcome,
    seed: u64,
    last_action: Option<Action>,
}
impl Battle {
    pub fn new(player: Vec<Fighter>, enemy: Vec<Fighter>, seed: u64) -> Result<Self> {
        if player.is_empty()
            || player.len() > 3
            || enemy.is_empty()
            || enemy.len() > 3
            || player.iter().chain(&enemy).any(|f| f.health != 30)
        {
            return Err(Error::BadInput);
        }
        Ok(Self {
            ruleset: 1,
            turn: 0,
            player,
            enemy,
            outcome: Outcome::Active,
            seed,
            last_action: None,
        })
    }
    fn roll(&mut self) -> u16 {
        // SplitMix64, fixed ruleset v1; arithmetic intentionally wraps.
        self.seed = self.seed.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.seed;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        (6 + ((z ^ (z >> 31)) % 5)) as u16
    }
    /// Optimistic turn identity makes same-action retries idempotent and refuses
    /// stale/different turns, including after the terminal round.
    pub fn advance(&mut self, expected: u32, action: Action) -> Result<()> {
        if self.turn > 0 && expected == self.turn - 1 && self.last_action == Some(action) {
            return Ok(());
        }
        if self.ruleset != 1 || self.outcome != Outcome::Active || expected != self.turn {
            return Err(Error::Conflict);
        }
        let p = self
            .player
            .iter()
            .position(|f| f.health > 0)
            .ok_or(Error::Unavailable)?;
        let e = self
            .enemy
            .iter()
            .position(|f| f.health > 0)
            .ok_or(Error::Unavailable)?;
        let strike = self.roll();
        let retaliation = self.roll();
        let (outgoing, incoming) = match action {
            Action::Strike => (strike, retaliation),
            Action::Guard => (3, retaliation.saturating_sub(6)),
        };
        // Simultaneous strikes prevent first-player advantage; reserves enter next turn.
        self.enemy[e].health = self.enemy[e].health.saturating_sub(outgoing);
        self.player[p].health = self.player[p].health.saturating_sub(incoming);
        self.turn += 1;
        self.last_action = Some(action);
        let alive = (
            self.player.iter().any(|f| f.health > 0),
            self.enemy.iter().any(|f| f.health > 0),
        );
        self.outcome = match alive {
            (true, false) => Outcome::Victory,
            (false, true) => Outcome::Defeat,
            (false, false) => Outcome::Draw,
            _ if self.turn >= 100 => Outcome::Draw,
            _ => Outcome::Active,
        };
        Ok(())
    }
}
