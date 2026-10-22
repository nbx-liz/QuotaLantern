//! `quotalantern-statusline`: Claude Code status line command.
//!
//! - no arguments: prints a compact quota/context status;
//! - `--shape <path>`: overwrites `<path>` with the key/type shape of the
//!   input (never values) for G0 audits, and prints a fixed confirmation.
//!
//! Always exits 0 so that a failure never disturbs Claude Code, and never
//! writes input content or library messages to stdout or stderr.

use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use quotalantern_adapter_claude::{
    AdapterError, MAX_INPUT_BYTES, describe_shape, extract, render_status,
};

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let input = read_stdin();
    let line = match args.as_slice() {
        [] => render_status(&input.and_then(|raw| extract(&raw))),
        [flag, path] if flag == "--shape" => record_shape(input, Path::new(path)),
        _ => "QuotaLantern: 引数が不正です".to_owned(),
    };
    // A closed stdout must not turn into a failure exit either.
    let _ = writeln!(io::stdout().lock(), "{line}");
    ExitCode::SUCCESS
}

/// Reads at most one byte more than the limit, so that oversized input is
/// detected without buffering it.
fn read_stdin() -> Result<Vec<u8>, AdapterError> {
    let mut raw = Vec::new();
    let limit = u64::try_from(MAX_INPUT_BYTES)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    io::stdin()
        .lock()
        .take(limit)
        .read_to_end(&mut raw)
        .map_err(|_| AdapterError::Malformed)?;
    if raw.len() > MAX_INPUT_BYTES {
        return Err(AdapterError::TooLarge);
    }
    Ok(raw)
}

fn record_shape(input: Result<Vec<u8>, AdapterError>, path: &Path) -> String {
    let shape = match input.and_then(|raw| describe_shape(&raw)) {
        Ok(shape) => shape,
        Err(err) => format!("# shape\terror:{}\n", err.code()),
    };
    match write_private(path, shape.as_bytes()) {
        Ok(()) => "QuotaLantern: shape recorded".to_owned(),
        Err(_) => "QuotaLantern: shape not recorded".to_owned(),
    }
}

/// Writes a new sibling file, readable by the owner only on Unix, and renames
/// it over `path`. A symlink at `path` is replaced rather than followed, and
/// the permissions of an existing file are not inherited.
fn write_private(path: &Path, contents: &[u8]) -> io::Result<()> {
    let mut temp = path.as_os_str().to_owned();
    temp.push(format!(".{}.tmp", std::process::id()));
    let temp = PathBuf::from(temp);

    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temp)?;
    let written = file.write_all(contents);
    drop(file);
    let result = written.and_then(|()| fs::rename(&temp, path));
    if result.is_err() {
        // Best effort: the write or rename error is what the caller reports.
        let _ = fs::remove_file(&temp);
    }
    result
}
