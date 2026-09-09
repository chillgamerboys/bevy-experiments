//! Print the canonical rules/content identity for release diagnostics.

use std::io::{self, Write};

fn main() -> io::Result<()> {
    writeln!(
        io::stdout().lock(),
        "{}",
        labyrinth_rules::rules_fingerprint()
    )
}
