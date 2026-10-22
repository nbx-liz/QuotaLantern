use crate::{AdapterError, ContextUsage, Observation, QuotaWindow, Reading};

/// Compact status line text: remaining quota per window and context usage.
///
/// Remaining shares are rounded down so that the display never overstates
/// what is left. Built only from typed values and fixed labels, never from
/// input text.
pub fn render_status(result: &Result<Observation, AdapterError>) -> String {
    let Ok(observation) = result else {
        return "QuotaLantern: 入力エラー".to_owned();
    };
    let quota = match (observation.five_hour, observation.seven_day) {
        (Reading::Absent, Reading::Absent) => "利用枠 —".to_owned(),
        (five_hour, seven_day) => format!(
            "5h残 {} · 7d残 {}",
            remaining(five_hour),
            remaining(seven_day)
        ),
    };
    format!("{quota} · ctx {}", context(observation.context))
}

fn remaining(reading: Reading<QuotaWindow>) -> String {
    match reading {
        // `floor(100 - used)` can round up in f64 (`used = 1e-15` gives
        // 100); `ceil` and the integer subtraction are both exact.
        Reading::Available(window) => format!("{}%", 100.0 - window.used.get().ceil()),
        Reading::Absent => "—".to_owned(),
        Reading::Invalid => "?".to_owned(),
    }
}

fn context(reading: Reading<ContextUsage>) -> String {
    match reading {
        // Adding 0.0 turns a received `-0` into 0, which would print as "-0".
        Reading::Available(usage) => format!("{}%", usage.used.get().floor() + 0.0),
        Reading::Absent => "—".to_owned(),
        Reading::Invalid => "?".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quotalantern_protocol::Percent;

    fn window(used: f64) -> Reading<QuotaWindow> {
        Reading::Available(QuotaWindow {
            used: Percent::new(used).unwrap(),
            resets_at: None,
        })
    }

    fn observation(
        five_hour: Reading<QuotaWindow>,
        seven_day: Reading<QuotaWindow>,
        context: Reading<ContextUsage>,
    ) -> Observation {
        Observation {
            five_hour,
            seven_day,
            spend_limit_present: false,
            context,
        }
    }

    #[test]
    fn rounds_remaining_down() {
        let text = render_status(&Ok(observation(window(23.5), window(0.0), Reading::Absent)));
        assert_eq!(text, "5h残 76% · 7d残 100% · ctx —");
    }

    #[test]
    fn never_overstates_remaining_near_integers() {
        let text = render_status(&Ok(observation(
            window(1e-15),
            window(1.0 + f64::EPSILON),
            Reading::Absent,
        )));
        assert_eq!(text, "5h残 99% · 7d残 98% · ctx —");
    }

    #[test]
    fn negative_zero_renders_as_zero() {
        let context = Reading::Available(ContextUsage {
            used: Percent::new(-0.0).unwrap(),
            capacity_tokens: None,
        });
        let text = render_status(&Ok(observation(window(-0.0), Reading::Absent, context)));
        assert_eq!(text, "5h残 100% · 7d残 — · ctx 0%");
    }

    #[test]
    fn distinguishes_absent_and_invalid() {
        let text = render_status(&Ok(observation(
            Reading::Invalid,
            Reading::Absent,
            Reading::Invalid,
        )));
        assert_eq!(text, "5h残 ? · 7d残 — · ctx ?");
    }

    #[test]
    fn absent_quota_is_not_zero() {
        let text = render_status(&Ok(observation(
            Reading::Absent,
            Reading::Absent,
            Reading::Absent,
        )));
        assert_eq!(text, "利用枠 — · ctx —");
    }

    #[test]
    fn errors_render_fixed_text() {
        assert_eq!(
            render_status(&Err(AdapterError::Malformed)),
            "QuotaLantern: 入力エラー"
        );
    }
}
