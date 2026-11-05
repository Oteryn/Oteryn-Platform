//! SOURCE: Oteryn/Oteryn-Atlas, farm-intelligence.mjs and numeric contract,
//! revision 0bbd188; deterministic algorithms ported without renderer dependencies.
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstimateError {
    InvalidInput,
    Unavailable,
    Unreachable,
    NumericEnvelope,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Estimate {
    /// Generated drops, not guaranteed personal acquisition. All values are estimates.
    pub expected_kills: f64,
    pub p50_kills: u64,
    pub p80_kills: u64,
    pub p95_kills: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct Outcome {
    pub quantity: u32,
    pub probability_ppm: u32,
}

const MAX_KILLS: u64 = 1_000_000_000;

pub fn fixed(target: u32, quantity: u32, ppm: u32) -> Result<Estimate, EstimateError> {
    fixed_rational(target, quantity, u64::from(ppm), 1_000_000)
}

/// Exact rational input, within the original safe-integer publication envelope.
pub fn fixed_rational(
    target: u32,
    quantity: u32,
    numerator: u64,
    denominator: u64,
) -> Result<Estimate, EstimateError> {
    if target == 0
        || target > 100_000
        || quantity == 0
        || quantity > 100_000
        || denominator == 0
        || numerator > denominator
        || denominator > 9_007_199_254_740_991
    {
        return Err(EstimateError::InvalidInput);
    }
    if numerator == 0 {
        return Err(EstimateError::Unreachable);
    }
    let required = u64::from(target.div_ceil(quantity));
    let p = numerator as f64 / denominator as f64;
    let expected = required as f64 / p;
    if expected > MAX_KILLS as f64 {
        return Err(EstimateError::NumericEnvelope);
    }
    let threshold = |confidence| minimum(required, p, confidence);
    Ok(Estimate {
        expected_kills: expected,
        p50_kills: threshold(0.5)?,
        p80_kills: threshold(0.8)?,
        p95_kills: threshold(0.95)?,
    })
}

fn tail(kills: u64, p: f64, required: u64) -> f64 {
    if kills < required {
        return 0.0;
    }
    if p == 1.0 {
        return 1.0;
    }
    if required == 1 {
        return -(kills as f64 * (-p).ln_1p()).exp_m1();
    }
    let log_p = p.ln();
    let log_q = (-p).ln_1p();
    let mut term = kills as f64 * log_q;
    let mut max = f64::NEG_INFINITY;
    let mut sum = 0.0;
    for s in 0..required {
        if s > 0 {
            term += ((kills - s + 1) as f64).ln() - (s as f64).ln() + log_p - log_q;
        }
        if term > max {
            sum = if max == f64::NEG_INFINITY {
                1.0
            } else {
                sum * (max - term).exp() + 1.0
            };
            max = term;
        } else {
            sum += (term - max).exp();
        }
    }
    let cdf = max + sum.ln();
    if cdf < -40.0 {
        1.0
    } else {
        (-cdf.min(0.0).exp_m1()).clamp(0.0, 1.0)
    }
}

fn minimum(required: u64, p: f64, confidence: f64) -> Result<u64, EstimateError> {
    if p == 1.0 {
        return Ok(required);
    }
    let mut low = required - 1;
    let mut high = ((required as f64 / p).ceil() as u64)
        .max(required)
        .min(MAX_KILLS);
    while tail(high, p, required) < confidence {
        if high == MAX_KILLS {
            return Err(EstimateError::NumericEnvelope);
        }
        low = high;
        high = (high * 2).min(MAX_KILLS);
    }
    while high - low > 1 {
        let mid = low + (high - low) / 2;
        if tail(mid, p, required) >= confidence {
            high = mid;
        } else {
            low = mid;
        }
    }
    Ok(high)
}

/// Exact ppm PMF; duplicates and non-unit total mass are invalid. No guessed distribution.
pub fn exact_pmf(target: u32, outcomes: &[Outcome]) -> Result<Estimate, EstimateError> {
    if target == 0 || target > 5_000 || outcomes.is_empty() || outcomes.len() > 128 {
        return Err(EstimateError::InvalidInput);
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut total = 0_u64;
    for o in outcomes {
        if !seen.insert(o.quantity) || o.probability_ppm > 1_000_000 {
            return Err(EstimateError::InvalidInput);
        }
        total += u64::from(o.probability_ppm);
    }
    if total != 1_000_000 {
        return Err(EstimateError::InvalidInput);
    }
    let p0 = outcomes
        .iter()
        .find(|o| o.quantity == 0)
        .map_or(0.0, |o| f64::from(o.probability_ppm) / 1_000_000.0);
    if p0 == 1.0 {
        return Err(EstimateError::Unreachable);
    }
    let target = target as usize;
    let mut expected = vec![0.0; target + 1];
    for remaining in 1..=target {
        let continuation: f64 = outcomes
            .iter()
            .filter(|o| o.quantity > 0)
            .map(|o| {
                f64::from(o.probability_ppm) / 1_000_000.0
                    * expected[remaining.saturating_sub(o.quantity as usize)]
            })
            .sum();
        expected[remaining] = (1.0 + continuation) / (1.0 - p0);
    }
    let mut distribution = vec![0.0; target + 1];
    distribution[0] = 1.0;
    let mut thresholds = [0_u64; 3];
    let mut work = 0;
    for kills in 1..=200_000 {
        let mut next = vec![0.0; target + 1];
        for (quantity, mass) in distribution.iter().enumerate().filter(|(_, m)| **m != 0.0) {
            for o in outcomes {
                work += 1;
                if work > 20_000_000 {
                    return Err(EstimateError::NumericEnvelope);
                }
                next[(quantity + o.quantity as usize).min(target)] +=
                    mass * f64::from(o.probability_ppm) / 1_000_000.0;
            }
        }
        distribution = next;
        for (i, c) in [0.5, 0.8, 0.95].iter().enumerate() {
            if thresholds[i] == 0 && distribution[target] + 1e-14 >= *c {
                thresholds[i] = kills;
            }
        }
        if thresholds.iter().all(|k| *k > 0) {
            return Ok(Estimate {
                expected_kills: expected[target],
                p50_kills: thresholds[0],
                p80_kills: thresholds[1],
                p95_kills: thresholds[2],
            });
        }
    }
    Err(EstimateError::NumericEnvelope)
}
