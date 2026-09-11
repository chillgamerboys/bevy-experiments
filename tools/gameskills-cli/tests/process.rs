//! Compiled probes verify the foundation's literal subprocess boundary.

use gameskills_cli::platform::ProcessSpec;
use serde_json::Value;
use std::error::Error;

#[test]
fn compiled_probe_preserves_argv_cwd_output_and_failure() -> Result<(), Box<dyn Error>> {
    let current = std::env::current_exe()?;
    let profile = current
        .parent()
        .and_then(std::path::Path::parent)
        .ok_or("test executable has no profile directory")?;
    let probe = profile
        .join("examples")
        .join(format!("process_probe{}", std::env::consts::EXE_SUFFIX));
    assert!(
        probe.is_file(),
        "Cargo must build the process_probe example for this test: {}",
        probe.display()
    );
    let directory = tempfile::tempdir()?;
    let literal = "$(touch SHOULD_NOT_EXIST); echo text with spaces";
    for (mode, code) in [("echo", 0), ("fail", 7)] {
        let result = ProcessSpec {
            program: probe.clone().into_os_string(),
            args: vec![mode.into(), literal.into()],
            cwd: directory.path().to_path_buf(),
        }
        .output()?;
        assert_eq!(result.status.code(), Some(code));
        let result_json: Value = serde_json::from_slice(&result.stdout)?;
        assert_eq!(result_json.get("args"), Some(&serde_json::json!([literal])));
        let cwd = result_json
            .get("cwd")
            .and_then(Value::as_str)
            .ok_or("probe cwd missing")?;
        assert_eq!(
            std::fs::canonicalize(cwd)?,
            std::fs::canonicalize(directory.path())?
        );
        assert_eq!(String::from_utf8(result.stderr)?, "probe stderr\n");
    }
    assert_eq!(std::fs::read_dir(directory.path())?.count(), 0);
    Ok(())
}
