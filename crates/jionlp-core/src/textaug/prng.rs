//! Tiny SplitMix64 PRNG — deterministic, seedable, no extra deps.
//!
//! Shared across all textaug modules so they produce reproducible output
//! for a given `(seed, input)` pair.

use std::time::{SystemTime, UNIX_EPOCH};

/// Seed the PRNG from the system clock. Use `seed = 0` in any module that
/// wants "non-deterministic but still seedable" behavior.
pub fn clock_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x1234_5678_9abc_def0)
}

/// SplitMix64 — small, fast, good statistical properties.
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(0x9E3779B97F4A7C15),
        }
    }

    pub fn from_opt(seed: u64) -> Self {
        Self::new(if seed == 0 { clock_seed() } else { seed })
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1) with 53-bit precision.
    pub fn uniform01(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform integer in [0, bound).
    pub fn uniform_int(&mut self, bound: usize) -> usize {
        if bound == 0 {
            return 0;
        }
        (self.next_u64() as usize) % bound
    }

    /// Standard-normal sample via Box-Muller.
    pub fn normal(&mut self) -> f64 {
        let u1 = self.uniform01().max(1e-12);
        let u2 = self.uniform01();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }

    /// Weighted choice: pick an index proportional to `weights`.
    pub fn weighted_choice(&mut self, weights: &[f64]) -> usize {
        let total: f64 = weights.iter().sum();
        if total <= 0.0 {
            return 0;
        }
        let r = self.uniform01() * total;
        let mut acc = 0.0;
        for (i, &w) in weights.iter().enumerate() {
            acc += w;
            if r < acc {
                return i;
            }
        }
        weights.len() - 1
    }
}
