//! Allowlisted subset of the status line input. Every other field is skipped
//! by serde without being copied. These types deliberately have no `Debug`.

use quotalantern_protocol::{Percent, PositiveSafeUint, UtcTimestamp};
use serde::Deserialize;
use serde::de::IgnoredAny;

use crate::{ContextUsage, Observation, QuotaWindow, Reading};

#[derive(Deserialize)]
pub(crate) struct StatusInput {
    #[serde(default)]
    rate_limits: Option<RateLimits>,
    #[serde(default)]
    context_window: Option<ContextWindow>,
}

#[derive(Deserialize)]
struct RateLimits {
    #[serde(default)]
    five_hour: Option<Window>,
    #[serde(default)]
    seven_day: Option<Window>,
    #[serde(default)]
    spend_limit: Option<IgnoredAny>,
}

#[derive(Deserialize)]
struct Window {
    #[serde(default)]
    used_percentage: Option<f64>,
    #[serde(default)]
    resets_at: Option<i64>,
}

#[derive(Deserialize)]
struct ContextWindow {
    #[serde(default)]
    used_percentage: Option<f64>,
    #[serde(default)]
    context_window_size: Option<u64>,
}

impl StatusInput {
    pub(crate) fn into_observation(self) -> Observation {
        let (five_hour, seven_day, spend_limit_present) = match self.rate_limits {
            Some(limits) => (
                quota(limits.five_hour),
                quota(limits.seven_day),
                limits.spend_limit.is_some(),
            ),
            None => (Reading::Absent, Reading::Absent, false),
        };
        Observation {
            five_hour,
            seven_day,
            spend_limit_present,
            context: context(self.context_window),
        }
    }
}

fn quota(window: Option<Window>) -> Reading<QuotaWindow> {
    let Some(Window {
        used_percentage: Some(used),
        resets_at,
    }) = window
    else {
        return Reading::Absent;
    };
    let Some(used) = Percent::new(used) else {
        return Reading::Invalid;
    };
    let resets_at = match resets_at.map(UtcTimestamp::from_unix_seconds) {
        None => None,
        Some(Some(instant)) => Some(instant),
        Some(None) => return Reading::Invalid,
    };
    Reading::Available(QuotaWindow { used, resets_at })
}

fn context(window: Option<ContextWindow>) -> Reading<ContextUsage> {
    let Some(ContextWindow {
        used_percentage: Some(used),
        context_window_size,
    }) = window
    else {
        return Reading::Absent;
    };
    let Some(used) = Percent::new(used) else {
        return Reading::Invalid;
    };
    let capacity_tokens = match context_window_size.map(PositiveSafeUint::new) {
        None => None,
        Some(Some(capacity)) => Some(capacity),
        Some(None) => return Reading::Invalid,
    };
    Reading::Available(ContextUsage {
        used,
        capacity_tokens,
    })
}
