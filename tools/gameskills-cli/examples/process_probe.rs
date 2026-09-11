//! Compiled subprocess fixture, never installed as a GameSkills binary.

use serde_json::json;
use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mode = args.next().unwrap_or_default();
    if mode == "sleep" {
        let Some(duration) = args
            .next()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value <= 10_000)
        else {
            return ExitCode::from(2);
        };
        std::thread::sleep(std::time::Duration::from_millis(duration));
    }
    let status = match mode.as_str() {
        "echo" | "sleep" => 0,
        "fail" => 7,
        _ => 2,
    };
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(_) => return ExitCode::from(2),
    };
    if writeln!(
        io::stdout().lock(),
        "{}",
        json!({"args": args.collect::<Vec<_>>(), "cwd": cwd, "mode": mode})
    )
    .is_err()
        || writeln!(io::stderr().lock(), "probe stderr").is_err()
    {
        return ExitCode::from(2);
    }
    ExitCode::from(status)
}
