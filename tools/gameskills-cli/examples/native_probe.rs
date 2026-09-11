//! Compiled native app-server protocol fixture; never used by production commands.
use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, Write},
    path::PathBuf,
    process::Command,
    time::Duration,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args();
    while let Some(arg) = args.next() {
        if arg == "--client-exit" {
            std::process::exit(args.next().ok_or("exit status")?.parse()?);
        }
    }

    if std::env::args().any(|arg| arg == "--hold-stdout") {
        std::thread::sleep(Duration::from_secs(60));
        return Ok(());
    }
    let root: PathBuf = std::env::current_exe()?
        .parent()
        .ok_or("probe directory")?
        .to_owned();
    let scenario: Value = serde_json::from_slice(&fs::read(root.join("scenario.json"))?)?;
    fs::write(root.join("pid"), std::process::id().to_string())?;
    let mode = scenario
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("success");
    let mut seen = 0;
    for line in std::io::stdin().lock().lines() {
        let line = line?;
        let mut trace = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(root.join("requests.jsonl"))?;
        writeln!(trace, "{line}")?;
        let request: Value = serde_json::from_str(&line)?;
        let Some(id) = request.get("id") else {
            continue;
        };
        let method = request
            .get("method")
            .and_then(Value::as_str)
            .ok_or("method")?;
        match mode {
            "exit" => std::process::exit(3),
            "malformed" => {
                writeln!(std::io::stdout(), "not JSON")?;
                std::io::stdout().flush()?;
                continue;
            }
            "oversized" => {
                std::io::stdout().write_all(&vec![b'x'; 4 * 1024 * 1024 + 2])?;
                std::io::stdout().flush()?;
                continue;
            }
            "error" => {
                writeln!(
                    std::io::stdout(),
                    "{}",
                    json!({"id":id,"error":{"code":-1,"message":"fixture error"}})
                )?;
                std::io::stdout().flush()?;
                continue;
            }
            "interactive" => {
                writeln!(
                    std::io::stdout(),
                    "{}",
                    json!({"id":999,"method":"approval/request"})
                )?;
                std::io::stdout().flush()?;
                continue;
            }
            _ => {}
        }
        let result = match method {
            "initialize" => {
                json!({"codexHome":scenario.get("home"),"userAgent":"rust-native-probe/1"})
            }
            "plugin/list" => {
                json!({"marketplaces":[{"name":scenario.get("market"),"plugins":scenario.get("plugins")}]})
            }
            "skills/list" => {
                if mode == "hang" {
                    std::thread::sleep(Duration::from_secs(60));
                }
                if mode == "child_pipe" {
                    let child = Command::new(std::env::current_exe()?)
                        .arg("--hold-stdout")
                        .spawn()?;
                    fs::write(root.join("child-pid"), child.id().to_string())?;
                    std::process::exit(0);
                }
                seen += 1;
                let skills = if mode == "delayed" && seen == 1 {
                    json!([])
                } else {
                    scenario.get("skills").cloned().ok_or("skills")?
                };
                json!({"data":[{"cwd":request.pointer("/params/cwds/0"),"skills":skills,"errors":[]} ]})
            }
            _ => return Err(format!("unexpected RPC {method}").into()),
        };
        let id = if mode == "wrong_id" {
            json!(777)
        } else {
            id.clone()
        };
        writeln!(std::io::stdout(), "{}", json!({"id":id,"result":result}))?;
        std::io::stdout().flush()?;
        if mode == "backpressure" && method == "plugin/list" {
            // Stop reading requests but emit plausible, incomplete discovery
            // responses. The separate full-request writer regression establishes
            // backpressure without relying on a platform-specific pipe capacity.
            #[cfg(unix)]
            signal_hook::flag::register(
                signal_hook::consts::SIGPIPE,
                std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            )?;
            fs::write(root.join("stopped-draining"), "true")?;
            let mut next = id.as_u64().ok_or("request id")? + 1;
            loop {
                let result = json!({"data":[{"cwd":request.pointer("/params/cwds/0"),"skills":[],"errors":[]}]});
                let output = writeln!(std::io::stdout(), "{}", json!({"id":next,"result":result}))
                    .and_then(|()| std::io::stdout().flush());
                if output.is_err() {
                    // Even if the bounded response reader closes its pipe, keep
                    // stdin open and undrained. Cleanup must not depend on the
                    // peer exiting cooperatively after a broken stdout pipe.
                    fs::write(root.join("output-closed"), "true")?;
                    std::thread::sleep(Duration::from_secs(60));
                    return Ok(());
                }
                fs::write(root.join("response-count"), (next - 1).to_string())?;
                next += 1;
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
    Ok(())
}
