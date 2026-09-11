//! GameSkills executable entrypoint.

use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let response = gameskills_cli::cli::execute(std::env::args_os());
    if writeln!(io::stdout().lock(), "{}", response.output).is_err() {
        return ExitCode::from(2);
    }
    ExitCode::from(response.exit_code)
}
