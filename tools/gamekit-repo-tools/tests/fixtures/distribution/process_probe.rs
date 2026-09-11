//! Compiled subprocess fixture; never installed as a repository command.

use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let mode = args.next().unwrap_or_default();
    if mode == "fail" {
        if io::stdout().lock().write_all(b"failed probe").is_err() {
            return ExitCode::from(2);
        }
        return ExitCode::from(23);
    }
    let Some(literal) = args.next().and_then(|value| value.into_string().ok()) else {
        return ExitCode::from(2);
    };
    if io::stdout().lock().write_all(literal.as_bytes()).is_err() {
        return ExitCode::from(2);
    }
    ExitCode::SUCCESS
}
