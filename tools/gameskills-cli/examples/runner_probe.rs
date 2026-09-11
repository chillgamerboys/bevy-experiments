//! Compiled runner harness and adversarial process fixtures; never installed.
use serde_json::{json, Value};
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, ExitCode};
#[cfg(unix)]
use std::sync::{atomic::AtomicBool, Arc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn stamp() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
fn run() -> Result<Value, String> {
    let mut args = std::env::args_os().skip(1);
    let mode = args
        .next()
        .and_then(|a| a.into_string().ok())
        .ok_or("missing mode")?;
    if mode == "__runner-supervisor" {
        return gameskills_cli::runner::supervisor(&args.collect::<Vec<_>>());
    }
    if mode == "harness" {
        let root = args.next().ok_or("missing root")?;
        let config = args.next().ok_or("missing config")?;
        let family = args
            .next()
            .and_then(|a| a.into_string().ok())
            .ok_or("missing family")?;
        let config: Value =
            serde_json::from_slice(&std::fs::read(config).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        return gameskills_cli::runner::execute(
            Path::new(&root),
            &config,
            &family,
            &args.collect::<Vec<_>>(),
        );
    }
    let rest = args
        .map(|a| {
            a.into_string()
                .map_err(|_| "non UTF-8 fixture input".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let arg = |i: usize| {
        rest.get(i)
            .map(String::as_str)
            .ok_or("missing fixture argument".to_owned())
    };
    match mode.as_str() {
        "echo" => {
            writeln!(io::stdout().lock(), "{}", arg(0)?).map_err(|e| e.to_string())?;
            writeln!(io::stderr().lock(), "diagnostic").map_err(|e| e.to_string())?;
        }
        "fail" => {
            writeln!(io::stdout().lock(), "before failure").map_err(|e| e.to_string())?;
            writeln!(io::stderr().lock(), "why").map_err(|e| e.to_string())?;
            return Ok(json!({"exit_code":7}));
        }
        "cwd" => {
            writeln!(
                io::stdout().lock(),
                "{}",
                std::env::current_dir()
                    .map_err(|e| e.to_string())?
                    .display()
            )
            .map_err(|e| e.to_string())?;
        }
        "interval" | "sleep" => {
            let millis = arg(0)?
                .parse::<u64>()
                .map_err(|e| e.to_string())?
                .min(10_000);
            writeln!(io::stdout().lock(), "{}", stamp()).map_err(|e| e.to_string())?;
            std::thread::sleep(Duration::from_millis(millis));
            writeln!(io::stdout().lock(), "{}", stamp()).map_err(|e| e.to_string())?;
        }
        "mutate" => {
            std::fs::write(arg(0)?, arg(1)?).map_err(|e| e.to_string())?;
        }
        "retry" => {
            let marker = Path::new(arg(0)?);
            let previous = marker.exists();
            std::fs::write(marker, "attempt").map_err(|e| e.to_string())?;
            return Ok(json!({"exit_code":if previous{0}else{3}}));
        }
        "delay-write" => {
            #[cfg(unix)]
            let _signal = signal_hook::flag::register(
                signal_hook::consts::SIGTERM,
                Arc::new(AtomicBool::new(false)),
            )
            .map_err(|e| e.to_string())?;
            std::thread::sleep(Duration::from_millis(
                arg(1)?
                    .parse::<u64>()
                    .map_err(|e| e.to_string())?
                    .min(10_000),
            ));
            std::fs::write(arg(0)?, "escaped").map_err(|e| e.to_string())?;
        }
        "spawn" => {
            let first = !Path::new(arg(0)?).exists();
            let mut child = Command::new(std::env::current_exe().map_err(|e| e.to_string())?)
                .args(["delay-write", arg(1)?, "1200"])
                .spawn()
                .map_err(|e| e.to_string())?;
            std::fs::write(arg(0)?, format!("{} {}", std::process::id(), child.id()))
                .map_err(|e| e.to_string())?;
            writeln!(io::stdout().lock(), "started").map_err(|e| e.to_string())?;
            if arg(2)? == "hang" || arg(2)? == "once" && first {
                std::thread::sleep(Duration::from_secs(8));
                let _ = child.wait();
            }
            // The successful-leader fixture deliberately leaves an inherited-log child.
        }
        _ => return Err("unknown fixture mode".into()),
    }
    Ok(json!({"exit_code":0}))
}
fn main() -> ExitCode {
    let mode = std::env::args().nth(1).unwrap_or_default();
    let result = run();
    let (status, output) = match result {
        Ok(value) => {
            let status = value
                .get("exit_code")
                .and_then(Value::as_u64)
                .and_then(|n| u8::try_from(n).ok())
                .unwrap_or_else(|| {
                    if value.get("ok") == Some(&Value::Bool(true)) {
                        0
                    } else {
                        1
                    }
                });
            (status, value)
        }
        Err(error) => (2, json!({"ok":false,"error":error})),
    };
    if mode == "harness" && writeln!(io::stdout().lock(), "{output}").is_err() {
        return ExitCode::from(2);
    }
    ExitCode::from(status)
}
