//! Bounded fixed windows. Refused hits do not change counters or other windows.
use std::{collections::HashMap, time::Duration};
#[derive(Debug)]
pub struct WindowLimiter {
    limit: usize,
    window: Duration,
    max_keys: usize,
    entries: HashMap<Vec<u8>, (Duration, usize)>,
}
impl WindowLimiter {
    pub fn new(limit: usize, window: Duration, max_keys: usize) -> Self {
        Self {
            limit,
            window,
            max_keys,
            entries: HashMap::new(),
        }
    }
    pub fn allow(&mut self, key: &[u8], now: Duration) -> Result<(), u64> {
        if let Some((start, count)) = self.entries.get_mut(key)
            && now.saturating_sub(*start) < self.window
        {
            if *count >= self.limit {
                return Err(retry_seconds(
                    self.window.saturating_sub(now.saturating_sub(*start)),
                ));
            }
            *count += 1;
            return Ok(());
        }
        if !self.entries.contains_key(key) && self.entries.len() >= self.max_keys {
            self.entries
                .retain(|_, (start, _)| now.saturating_sub(*start) < self.window);
            if self.entries.len() >= self.max_keys {
                return Err(retry_seconds(self.window));
            }
        }
        self.entries.insert(key.to_vec(), (now, 1));
        Ok(())
    }
}
fn retry_seconds(d: Duration) -> u64 {
    (d.as_secs() + u64::from(d.subsec_nanos() > 0)).max(1)
}
