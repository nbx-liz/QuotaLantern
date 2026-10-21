//! Schema/parser agreement (ADR-0009): every fixture is judged by the JSON
//! Schema validator and by `parse_event`, and both verdicts must match the
//! manifest.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use jsonschema::Validator;
use quotalantern_protocol::{
    MAX_EVENT_BYTES, Metric, ParseError, SCHEMA_VERSION, State, Target, parse_event,
};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    file: String,
    expected_valid: bool,
    reject_layer: Option<RejectLayer>,
    purpose: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RejectLayer {
    Schema,
    Parser,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn events_dir() -> PathBuf {
    repo_root().join("tests/fixtures/events")
}

fn validator() -> Validator {
    let schema: Value =
        serde_json::from_slice(&fs::read(repo_root().join("schemas/event.schema.json")).unwrap())
            .unwrap();
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .should_ignore_unknown_formats(false)
        .build(&schema)
        .unwrap()
}

fn cases() -> Vec<Case> {
    serde_json::from_slice(&fs::read(events_dir().join("manifest.json")).unwrap()).unwrap()
}

/// Guards against the v0.1 failure where `format` was silently not enforced.
#[test]
fn schema_validator_enforces_formats() {
    let probe = jsonschema::draft202012::options()
        .should_validate_formats(true)
        .should_ignore_unknown_formats(false)
        .build(&json!({"format": "date-time"}))
        .unwrap();
    assert!(probe.is_valid(&json!("2026-10-21T09:00:00Z")));
    assert!(!probe.is_valid(&json!("yesterday")));
    assert!(!probe.is_valid(&json!("2026-02-30T00:00:00Z")));
}

#[test]
fn manifest_lists_every_fixture_exactly_once() {
    let listed: Vec<String> = cases().into_iter().map(|case| case.file).collect();
    let unique: BTreeSet<&String> = listed.iter().collect();
    assert_eq!(unique.len(), listed.len(), "duplicate manifest entry");

    let on_disk: BTreeSet<String> = fs::read_dir(events_dir())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name != "manifest.json")
        .collect();
    let listed: BTreeSet<String> = listed.into_iter().collect();
    assert_eq!(on_disk, listed, "fixtures and manifest differ");
}

#[test]
fn fixtures_match_manifest_in_schema_and_parser() {
    let validator = validator();
    let cases = cases();
    assert!(!cases.is_empty());
    for case in cases {
        let Case {
            file,
            expected_valid,
            reject_layer,
            purpose,
        } = case;
        assert!(!purpose.is_empty(), "{file}: missing purpose");
        assert_eq!(
            expected_valid,
            reject_layer.is_none(),
            "{file}: inconsistent manifest entry"
        );

        let raw = fs::read(events_dir().join(&file)).unwrap();
        assert_eq!(
            parse_event(&raw).is_ok(),
            expected_valid,
            "{file}: parser verdict"
        );

        match serde_json::from_slice::<Value>(&raw) {
            Ok(instance) => {
                let schema_valid = validator.is_valid(&instance);
                let expected_schema_valid = reject_layer != Some(RejectLayer::Schema);
                assert_eq!(
                    schema_valid, expected_schema_valid,
                    "{file}: schema verdict"
                );
            }
            // Not JSON at all: only the parser layer can own the rejection.
            Err(_) => assert_eq!(reject_layer, Some(RejectLayer::Parser), "{file}: not JSON"),
        }
    }
}

#[test]
fn counter_sequence_is_valid_in_both_layers() {
    let validator = validator();
    let raw =
        fs::read(repo_root().join("tests/fixtures/sequences/counter_sequence.jsonl")).unwrap();
    let mut values = Vec::new();
    for line in raw.split(|&b| b == b'\n').filter(|line| !line.is_empty()) {
        assert!(validator.is_valid(&serde_json::from_slice::<Value>(line).unwrap()));
        match parse_event(line).unwrap().metric {
            Metric::TokenCounter(counter) => values.push(counter.value.get()),
            other => panic!("unexpected metric {other:?}"),
        }
    }
    assert_eq!(values, [100, 130, 130, 160]);
}

#[test]
fn parsed_values_are_exposed() {
    let raw = fs::read(events_dir().join("valid_source_state_bucket.json")).unwrap();
    let Metric::SourceState(state) = parse_event(&raw).unwrap().metric else {
        panic!("expected source_state");
    };
    assert_eq!(state.target, Target::Quota);
    assert_eq!(state.state, State::Stale);
    assert!(state.subject_id.is_some());
}

fn sample_event() -> Value {
    serde_json::from_slice(&fs::read(events_dir().join("valid_quota.json")).unwrap()).unwrap()
}

#[test]
fn rejects_oversized_event_before_parsing() {
    let mut raw = serde_json::to_vec(&sample_event()).unwrap();
    raw.resize(MAX_EVENT_BYTES + 1, b' ');
    assert_eq!(parse_event(&raw), Err(ParseError::TooLarge));

    raw.truncate(MAX_EVENT_BYTES);
    assert!(
        parse_event(&raw).is_ok(),
        "trailing whitespace up to the limit is fine"
    );
}

#[test]
fn rejects_invalid_utf8() {
    let mut raw = serde_json::to_vec(&sample_event()).unwrap();
    raw.push(0xff);
    assert_eq!(parse_event(&raw), Err(ParseError::InvalidUtf8));
}

#[test]
fn rejects_excess_depth_before_parsing() {
    // The envelope and the metric are two levels; `levels` more makes the total.
    let with_extra_levels = |levels: usize| {
        let mut event = sample_event();
        let mut deep = json!(0);
        for _ in 0..levels {
            deep = json!({ "x": deep });
        }
        event["metric"]["extra"] = deep;
        parse_event(&serde_json::to_vec(&event).unwrap())
    };
    // Depth 16 passes the depth check and fails only on the unknown key.
    assert_eq!(with_extra_levels(14), Err(ParseError::Malformed));
    assert_eq!(with_extra_levels(15), Err(ParseError::TooDeep));
}

#[test]
fn unsupported_version_is_distinct() {
    let mut event = sample_event();
    event["schema_version"] = json!(SCHEMA_VERSION + 1);
    assert_eq!(
        parse_event(&serde_json::to_vec(&event).unwrap()),
        Err(ParseError::UnsupportedVersion)
    );

    event["schema_version"] = json!("1");
    assert_eq!(
        parse_event(&serde_json::to_vec(&event).unwrap()),
        Err(ParseError::Malformed)
    );
}

#[test]
fn errors_carry_no_input_content() {
    const CANARY: &str = "synthetic-canary@example.invalid";
    let mut event = sample_event();
    event["user_email"] = json!(CANARY);
    let err = parse_event(&serde_json::to_vec(&event).unwrap()).unwrap_err();
    assert_eq!(err, ParseError::Malformed);
    assert!(!format!("{err}").contains(CANARY));
    assert!(!format!("{err:?}").contains(CANARY));
}
