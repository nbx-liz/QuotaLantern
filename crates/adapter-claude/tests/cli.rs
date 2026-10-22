//! The status line binary must never disturb Claude Code (CL-04): exit 0,
//! fixed text only, nothing on stderr.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_quotalantern-statusline");

fn run(args: &[&str], stdin: &[u8]) -> Output {
    let mut child = Command::new(BIN)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut pipe = child.stdin.take().unwrap();
    // The binary may stop reading early for oversized input.
    let _ = pipe.write_all(stdin);
    drop(pipe);
    child.wait_with_output().unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

#[test]
fn prints_status_for_valid_input() {
    let output = run(&[], include_bytes!("fixtures/full.json"));
    assert!(output.status.success());
    assert_eq!(stdout(&output), "5h残 76% · 7d残 58% · ctx 8%\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn failures_exit_zero_with_fixed_text() {
    let oversized = vec![b' '; 1024 * 1024 + 10];
    let deep = format!("{}{}", "[".repeat(20), "]".repeat(20));
    let cases: [&[u8]; 5] = [
        b"",
        b"not json synthetic-canary-secret",
        &oversized,
        deep.as_bytes(),
        &[0xff, 0xfe],
    ];
    for input in cases {
        let output = run(&[], input);
        assert!(output.status.success());
        assert_eq!(stdout(&output), "QuotaLantern: 入力エラー\n");
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn unexpected_arguments_exit_zero() {
    let output = run(&["--unknown"], b"{}");
    assert!(output.status.success());
    assert_eq!(stdout(&output), "QuotaLantern: 引数が不正です\n");
}

fn scratch_file(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    dir.join(name)
}

#[test]
fn shape_mode_overwrites_file_with_shape_only() {
    let path = scratch_file("shape-overwrite.txt");
    std::fs::write(&path, "previous content that must disappear").unwrap();

    let output = run(
        &["--shape", path.to_str().unwrap()],
        include_bytes!("fixtures/full.json"),
    );
    assert!(output.status.success());
    assert_eq!(stdout(&output), "QuotaLantern: shape recorded\n");

    let shape = std::fs::read_to_string(&path).unwrap();
    assert!(!shape.contains("previous content"));
    assert!(!shape.contains("synthetic-canary"));
    assert!(shape.contains("$.rate_limits.five_hour.resets_at\tnumber\n"));
}

#[test]
fn shape_mode_records_errors_without_content() {
    let path = scratch_file("shape-error.txt");
    let output = run(
        &["--shape", path.to_str().unwrap()],
        b"synthetic-canary-secret",
    );
    assert!(output.status.success());
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "# shape\terror:input_malformed\n"
    );
}
