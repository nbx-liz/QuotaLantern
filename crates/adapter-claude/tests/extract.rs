//! Synthetic status line inputs (ADR-0002, #5). No real session data.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use quotalantern_adapter_claude::{
    AdapterError, MAX_INPUT_BYTES, Observation, Reading, describe_shape, extract,
};
use quotalantern_protocol::UtcTimestamp;
use serde_json::{Value, json};

const CANARIES: [&str; 4] = [
    "synthetic-canary-session-0001",
    "synthetic-canary-user",
    "synthetic-canary@example.invalid",
    "synthetic-canary-secret",
];

fn full() -> Value {
    serde_json::from_str(include_str!("fixtures/full.json")).unwrap()
}

fn run(value: &Value) -> Result<Observation, AdapterError> {
    extract(&serde_json::to_vec(value).unwrap())
}

#[test]
fn extracts_allowlisted_values() {
    let observation = run(&full()).unwrap();

    let Reading::Available(five_hour) = observation.five_hour else {
        panic!("five_hour not available");
    };
    assert_eq!(five_hour.used.get(), 23.5);
    assert_eq!(
        five_hour.resets_at,
        Some(UtcTimestamp::parse("2025-02-01T16:00:00Z").unwrap())
    );

    let Reading::Available(seven_day) = observation.seven_day else {
        panic!("seven_day not available");
    };
    assert_eq!(seven_day.used.get(), 41.2);

    let Reading::Available(context) = observation.context else {
        panic!("context not available");
    };
    assert_eq!(context.used.get(), 8.0);
    assert_eq!(context.capacity_tokens.map(|c| c.get()), Some(200_000));
    assert!(!observation.spend_limit_present);
}

#[test]
fn absent_rate_limits_are_absent_not_zero() {
    let mut input = full();
    input.as_object_mut().unwrap().remove("rate_limits");
    let observation = run(&input).unwrap();
    assert_eq!(observation.five_hour, Reading::Absent);
    assert_eq!(observation.seven_day, Reading::Absent);

    input["rate_limits"] = Value::Null;
    assert_eq!(run(&input).unwrap().five_hour, Reading::Absent);
}

#[test]
fn windows_are_independent() {
    let mut input = full();
    input["rate_limits"]
        .as_object_mut()
        .unwrap()
        .remove("five_hour");
    let observation = run(&input).unwrap();
    assert_eq!(observation.five_hour, Reading::Absent);
    assert!(matches!(observation.seven_day, Reading::Available(_)));
}

#[test]
fn early_session_null_context_is_absent() {
    let mut input = full();
    input["context_window"]["used_percentage"] = Value::Null;
    assert_eq!(run(&input).unwrap().context, Reading::Absent);
}

#[test]
fn spend_limit_is_recorded_but_never_converted() {
    let mut input = full();
    input["rate_limits"] = json!({
        "spend_limit": {"used_percentage": 120.0, "resets_at": 1740787200, "used_usd": 600.0}
    });
    let observation = run(&input).unwrap();
    assert!(observation.spend_limit_present);
    assert_eq!(observation.five_hour, Reading::Absent);
    assert_eq!(observation.seven_day, Reading::Absent);
}

#[test]
fn out_of_range_percent_is_invalid_not_clamped() {
    for used in [100.5, -1.0] {
        let mut input = full();
        input["rate_limits"]["five_hour"]["used_percentage"] = json!(used);
        input["context_window"]["used_percentage"] = json!(used);
        let observation = run(&input).unwrap();
        assert_eq!(observation.five_hour, Reading::Invalid, "{used}");
        assert_eq!(observation.context, Reading::Invalid, "{used}");
        assert!(matches!(observation.seven_day, Reading::Available(_)));
    }
}

#[test]
fn invalid_reset_time_invalidates_the_window() {
    let mut input = full();
    input["rate_limits"]["five_hour"]["resets_at"] = json!(i64::MAX);
    assert_eq!(run(&input).unwrap().five_hour, Reading::Invalid);
}

// ADR-0002: a wrong type in an allowlisted field invalidates only its own
// reading. This test pinned whole-input rejection before the review fix.
#[test]
fn wrong_type_invalidates_only_its_reading() {
    let cases = [
        ("used_percentage", json!("23.5")),
        ("used_percentage", json!(true)),
        ("used_percentage", json!([23.5])),
        ("resets_at", json!(1738425600.0)),
        ("resets_at", json!(1738425600.5)),
        ("resets_at", json!(u64::MAX)),
        ("resets_at", json!({"seconds": 1738425600})),
    ];
    for (field, value) in cases {
        let mut input = full();
        input["rate_limits"]["five_hour"][field] = value.clone();
        let observation = run(&input).unwrap();
        assert_eq!(observation.five_hour, Reading::Invalid, "{field} = {value}");
        assert!(matches!(observation.seven_day, Reading::Available(_)));
        assert!(matches!(observation.context, Reading::Available(_)));
    }

    for value in [json!(200000.0), json!(-1), json!("200000")] {
        let mut input = full();
        input["context_window"]["context_window_size"] = value;
        let observation = run(&input).unwrap();
        assert_eq!(observation.context, Reading::Invalid);
        assert!(matches!(observation.five_hour, Reading::Available(_)));
    }
}

// Derived structs also accept arrays and fill fields by position.
#[test]
fn arrays_never_fill_allowlisted_fields_by_position() {
    let raw = br#"{"rate_limits":[[5,1738425600],[7]],"context_window":[3]}"#;
    let observation = extract(raw).unwrap();
    assert_eq!(observation.five_hour, Reading::Invalid);
    assert_eq!(observation.seven_day, Reading::Invalid);
    assert_eq!(observation.context, Reading::Invalid);

    let raw = br#"{"rate_limits":{"five_hour":[5,1738425600]}}"#;
    let observation = extract(raw).unwrap();
    assert_eq!(observation.five_hour, Reading::Invalid);
    assert_eq!(observation.seven_day, Reading::Absent);

    let shape = describe_shape(br#"{"rate_limits":[[5]]}"#).unwrap();
    assert!(shape.contains("# five_hour\tinvalid\n"), "{shape}");
}

#[test]
fn non_object_sections_are_invalid() {
    for value in [json!(5), json!("x"), json!(true)] {
        let mut input = full();
        input["rate_limits"]["seven_day"] = value;
        let observation = run(&input).unwrap();
        assert_eq!(observation.seven_day, Reading::Invalid);
        assert!(matches!(observation.five_hour, Reading::Available(_)));
    }
}

#[test]
fn null_window_alone_is_absent() {
    let mut input = full();
    input["rate_limits"]["five_hour"] = Value::Null;
    let observation = run(&input).unwrap();
    assert_eq!(observation.five_hour, Reading::Absent);
    assert!(matches!(observation.seven_day, Reading::Available(_)));
}

#[test]
fn repeated_allowlisted_key_is_malformed() {
    let raw = br#"{"context_window":{"used_percentage":1,"used_percentage":2}}"#;
    assert_eq!(extract(raw), Err(AdapterError::Malformed));
}

#[test]
fn unknown_fields_and_arrays_are_skipped() {
    let mut input = full();
    input["future_field"] = json!([{"a": [1, 2, {"b": null}]}]);
    assert!(run(&input).is_ok());
}

#[test]
fn rejects_malformed_json() {
    let inputs = [
        &b"not json"[..],
        b"",
        b"{",
        b"[1,2]",
        b"[]",
        b"[[5,1738425600]]",
        b"null",
        b"5",
        b"\"text\"",
        b"{} {}",
    ];
    for raw in inputs {
        assert_eq!(extract(raw), Err(AdapterError::Malformed), "{raw:?}");
    }
}

#[test]
fn rejects_oversized_input() {
    let mut raw = serde_json::to_vec(&full()).unwrap();
    raw.resize(MAX_INPUT_BYTES, b' ');
    assert!(extract(&raw).is_ok());
    raw.push(b' ');
    assert_eq!(extract(&raw), Err(AdapterError::TooLarge));
}

#[test]
fn rejects_excess_depth() {
    let nest = |levels: usize| {
        let mut deep = json!(0);
        for _ in 0..levels {
            deep = json!([deep]);
        }
        let mut input = full();
        input["future_field"] = deep;
        run(&input)
    };
    // The top-level object is one level.
    assert!(nest(15).is_ok());
    assert_eq!(nest(16), Err(AdapterError::TooDeep));
}

#[test]
fn identifying_metadata_reaches_no_output() {
    let raw = serde_json::to_vec(&full()).unwrap();
    let observation = extract(&raw).unwrap();
    let shape = describe_shape(&raw).unwrap();
    let rendered = quotalantern_adapter_claude::render_status(&Ok(observation.clone()));
    let debug = format!("{observation:?}");
    for canary in CANARIES {
        for (what, text) in [("debug", &debug), ("shape", &shape), ("render", &rendered)] {
            assert!(!text.contains(canary), "{canary} leaked into {what}");
        }
    }
}

#[test]
fn shape_records_keys_and_types_only() {
    let shape = describe_shape(&serde_json::to_vec(&full()).unwrap()).unwrap();
    for expected in [
        "$\tobject",
        "$.cwd\tstring",
        "$.rate_limits.five_hour.used_percentage\tnumber",
        "$.workspace.added_dirs\tarray",
        "$.workspace.added_dirs[]\tstring",
        "$.<dynamic>\tobject",
        "$.<dynamic>.nested\tstring",
        "# five_hour\tavailable",
        "# spend_limit\tabsent",
        "# context\tavailable",
    ] {
        assert!(
            shape.lines().any(|line| line == expected),
            "missing {expected:?}"
        );
    }
    // Numbers from the input never appear.
    for value in ["23.5", "1738425600", "200000", "0.01234"] {
        assert!(!shape.contains(value), "value {value} leaked");
    }
}

#[test]
fn errors_carry_no_input_content() {
    let err = extract(b"{\"rate_limits\": synthetic-canary-secret}").unwrap_err();
    assert!(!format!("{err} {err:?}").contains("synthetic-canary"));
}
