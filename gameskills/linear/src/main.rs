//! Optional Linear command; cleanup is preview-only unless --apply is explicit.
use clap::{Parser, Subcommand};
use gameskills_linear::{cleanup, provider::Linear, tracking, Config};
use serde_json::{json, Value};
use std::{io::Write, path::PathBuf};
#[derive(Parser)]
#[command(version, about)]
struct Args {
    #[arg(long, default_value = "gameskills-linear.toml", global = true)]
    config: PathBuf,
    #[command(subcommand)]
    command: Operation,
}
#[derive(Subcommand)]
enum Operation {
    Cleanup {
        #[arg(long)]
        project: String,
        #[arg(long)]
        retention_days: Option<u32>,
        #[arg(long, default_value = "0")]
        limit: usize,
        #[arg(long)]
        export_dir: Option<PathBuf>,
        #[arg(long)]
        apply: bool,
    },
    Observe {
        #[arg(long)]
        issue: String,
        #[arg(long)]
        project: String,
        #[arg(long)]
        pr: String,
    },
    Link {
        #[arg(long)]
        issue: String,
        #[arg(long)]
        project: String,
        #[arg(long)]
        pr: String,
    },
    Create {
        #[arg(long)]
        id: String,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        title: String,
        #[arg(long)]
        description_file: PathBuf,
    },
    Complete {
        #[arg(long)]
        issue: String,
        #[arg(long)]
        project: String,
        #[arg(long)]
        state: String,
        #[arg(long, required = true)]
        pr: Vec<String>,
    },
    Reconcile {
        #[arg(long)]
        project: String,
        #[arg(long)]
        issue: String,
        #[arg(long)]
        export_dir: PathBuf,
    },
    Route {
        path: String,
    },
}
fn run(args: Args) -> Result<Value, String> {
    let c = Config::read(&args.config)?;
    if let Operation::Route { path } = &args.command {
        return Ok(json!({"ok":true,"project":c.route(path)}));
    }
    let mut p = Linear::from_env(&c.key_env)?;
    match args.command {
        Operation::Cleanup {
            project,
            retention_days,
            limit,
            export_dir,
            apply,
        } => cleanup::run(
            &mut p,
            &c,
            &cleanup::Options {
                project: &project,
                retention_days: retention_days.unwrap_or(c.retention_days),
                limit,
                apply,
                export_dir: export_dir.as_deref(),
                now: time::OffsetDateTime::now_utc(),
            },
        ),
        Operation::Observe { issue, project, pr } => {
            tracking::observe(&mut p, &c, &issue, &project, &pr)
        }
        Operation::Link { issue, project, pr } => tracking::link(&mut p, &c, &issue, &project, &pr),
        Operation::Create {
            id,
            project,
            title,
            description_file,
        } => tracking::create(
            &mut p,
            &c,
            &id,
            project.as_deref().unwrap_or(&c.project),
            &title,
            &std::fs::read_to_string(description_file).map_err(|e| e.to_string())?,
        ),
        Operation::Complete {
            issue,
            project,
            state,
            pr,
        } => tracking::complete(&mut p, &c, &issue, &project, &state, &pr),
        Operation::Reconcile {
            project,
            issue,
            export_dir,
        } => cleanup::reconcile(&mut p, &c, &project, &issue, &export_dir),
        Operation::Route { .. } => Err("unexpected route".into()),
    }
}
fn main() {
    let result = run(Args::parse());
    let failed = result
        .as_ref()
        .map_or(true, |v| v.get("ok") == Some(&json!(false)));
    let value = result.unwrap_or_else(|e| json!({"ok":false,"error":e}));
    let mut stdout = std::io::stdout().lock();
    let _ = serde_json::to_writer_pretty(&mut stdout, &value);
    let _ = writeln!(stdout);
    if failed {
        std::process::exit(1);
    }
}
