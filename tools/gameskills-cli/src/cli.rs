//! Command boundary and versioned diagnostics for the Rust runtime.

use clap::{Parser, Subcommand};
use serde_json::json;
use std::ffi::OsString;
use std::path::PathBuf;

const SETUP_HELP: &str = "gameskills setup [--packages NAME ...] [--bundle PATH] [--apply]\ngameskills setup --recover\nWithout --apply, setup only prints a proposal.";
const BUNDLE_HELP: &str = "gameskills bundle --out NEW_DIRECTORY [--packages NAME ...]\ngameskills bundle --source CHECKOUT --revision FULL_COMMIT_OR_TAG --out NEW_DIRECTORY";
const NATIVE_HELP: &str = "gameskills native codex|claude [--launch] [-- CLIENT_ARGUMENTS ...]\ngameskills native codex --verify\nWithout --launch or --verify, print the selected client's command.";
const PLAN_HELP: &str = "gameskills plan validate --file PLAN.json";
const QUEUE_HELP: &str = "gameskills queue create --file PLAN.json\ngameskills queue status QUEUE_ID\ngameskills queue inject QUEUE_ID --file ORDER.json --expected-revision N\ngameskills queue start|resume QUEUE_ID ORDER_ID --worktree PATH --expected-revision N\ngameskills queue block|report|integrated QUEUE_ID ORDER_ID --file OBSERVATION.json --expected-revision N";
const RUN_HELP: &str = "gameskills run COMMAND ... [--max-workers N] [--resource-wait-seconds SECONDS] [--resume RUN_ID]\nCommands come from gameskills.toml; resume reruns the graph into a new record.";
const EVIDENCE_HELP: &str = "gameskills evidence list\ngameskills evidence show|validate RUN_ID";
const LEGACY_HELP: &str = "gameskills legacy import [--apply]\nInspect the old installation first; --apply preserves old client files and overlays.";

#[derive(Parser)]
#[command(
    name = "gameskills",
    version,
    about = "Install GameSkills, coordinate work and verify configured commands"
)]
struct Arguments {
    #[arg(long, default_value = ".", global = true)]
    root: PathBuf,
    #[command(subcommand)]
    command: Operation,
}

#[derive(Subcommand)]
enum Operation {
    #[command(name = "__runner-supervisor", hide = true)]
    Supervisor {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
    /// Inspect the installed configuration or validate a standalone TOML file.
    Config {
        #[command(subcommand)]
        command: Option<ConfigOperation>,
    },
    /// Inspect the embedded instruction catalog.
    Catalog,
    /// Verify the installed instruction bundle and configuration.
    Status,
    /// Propose, apply or recover an immutable instruction installation.
    #[command(after_help = SETUP_HELP)]
    Setup {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
    /// Export a verified immutable instruction bundle.
    #[command(after_help = BUNDLE_HELP)]
    Bundle {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
    /// Construct or launch a selected native agent client.
    #[command(after_help = NATIVE_HELP)]
    Native {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
    /// Validate a scoped work plan.
    #[command(after_help = PLAN_HELP)]
    Plan {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
    /// Coordinate revision-guarded durable work queues.
    #[command(after_help = QUEUE_HELP)]
    Queue {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
    /// Execute the selected configured command graph.
    #[command(after_help = RUN_HELP)]
    Run {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
    /// Inspect and validate execution records.
    #[command(after_help = EVIDENCE_HELP)]
    Evidence {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
    /// Import an older installation while preserving its owned files.
    #[command(after_help = LEGACY_HELP)]
    Legacy {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
}

#[derive(Subcommand)]
enum ConfigOperation {
    /// Validate a TOML file without setup, writes, or execution. Does not establish readiness.
    Validate {
        /// Configuration file, relative to --root unless absolute.
        #[arg(long, default_value = "gameskills.toml")]
        file: PathBuf,
    },
}

/// One command result; output is JSON except for help/version.
pub struct Response {
    /// Process exit status, zero only for the requested supported operation.
    pub exit_code: u8,
    /// Human-readable help/version, or one JSON document.
    pub output: String,
}

fn failure(code: &str, message: impl ToString) -> Response {
    Response { exit_code: 2, output: json!({"schema_version": 1, "ok": false, "error": {"code": code, "message": message.to_string()}}).to_string() }
}

/// Parse an argument vector and execute the requested operation.
pub fn execute(args: impl IntoIterator<Item = OsString>) -> Response {
    let parsed = match Arguments::try_parse_from(args) {
        Ok(parsed) => parsed,
        Err(error)
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) =>
        {
            return Response {
                exit_code: 0,
                output: error.to_string().trim_end().into(),
            };
        }
        Err(error) => return failure("invalid_arguments", error),
    };
    match parsed.command {
        Operation::Config {
            command: Some(ConfigOperation::Validate { file }),
        } => {
            let path = parsed.root.join(file);
            let source = match crate::platform::read_ordinary_file(&path) {
                Ok(source) => source,
                Err(error) => return failure("configuration_io", error),
            };
            match crate::config::parse(&source) {
                Ok(configuration) => Response { exit_code: 0, output: json!({"schema_version": 1, "ok": true, "scope": "configuration_structure", "configuration": configuration}).to_string() },
                Err(error) => failure("invalid_configuration", error),
            }
        }
        operation => {
            let (family, arguments) = match operation {
                Operation::Catalog => ("catalog", Vec::new()),
                Operation::Status => ("status", Vec::new()),
                Operation::Config { command: None } => ("config", Vec::new()),
                Operation::Setup { args } => ("setup", args),
                Operation::Bundle { args } => ("bundle", args),
                Operation::Native { args } => ("native", args),
                Operation::Legacy { args } => ("legacy", args),
                Operation::Plan { args } => ("plan", args),
                Operation::Queue { args } => ("queue", args),
                Operation::Run { args } => ("run", args),
                Operation::Evidence { args } => ("evidence", args),
                Operation::Supervisor { args } => ("__runner-supervisor", args),
                Operation::Config { command: Some(_) } => {
                    return failure("invalid_arguments", "unexpected config operation")
                }
            };
            // Nested help must be available before installation or filesystem IO.
            // Native client arguments after `--` belong to the launched client.
            if arguments
                .iter()
                .take_while(|arg| *arg != "--")
                .any(|arg| arg == "--help" || arg == "-h")
            {
                let help = match family {
                    "setup" => Some(SETUP_HELP),
                    "bundle" => Some(BUNDLE_HELP),
                    "native" => Some(NATIVE_HELP),
                    "plan" => Some(PLAN_HELP),
                    "queue" => Some(QUEUE_HELP),
                    "run" => Some(RUN_HELP),
                    "evidence" => Some(EVIDENCE_HELP),
                    "legacy" => Some(LEGACY_HELP),
                    _ => None,
                };
                if let Some(help) = help {
                    return Response {
                        exit_code: 0,
                        output: help.into(),
                    };
                }
            }
            // Native plugin paths must not be resolved relative to the adopter twice.
            // The private supervisor must reach its inherited handshake without root IO.
            let root = if family == "__runner-supervisor" {
                parsed.root
            } else {
                match parsed.root.canonicalize() {
                    Ok(root) => root,
                    Err(error) => return failure("root_io", error),
                }
            };
            let result = match family {
                "__runner-supervisor" => crate::runner::supervisor(&arguments),
                "plan" | "queue" | "run" | "evidence" => crate::installation::ready_config(&root)
                    .and_then(|config| {
                        if matches!(family, "plan" | "queue") {
                            crate::workflow::execute(&root, &config, family, &arguments)
                        } else {
                            crate::runner::execute(&root, &config, family, &arguments)
                        }
                    }),
                _ => crate::installation::execute(&root, family, &arguments),
            };
            match result {
                Ok(mut value) => {
                    let Some(object) = value.as_object_mut() else {
                        return failure("invalid_result", "command did not return an object");
                    };
                    object.entry("schema_version").or_insert(json!(1));
                    object.entry("ok").or_insert(json!(true));
                    let exit_code = object
                        .get("exit_code")
                        .and_then(serde_json::Value::as_u64)
                        .and_then(|code| u8::try_from(code).ok())
                        .unwrap_or_else(|| u8::from(object.get("ok") == Some(&json!(false))));
                    let output = object
                        .get("help")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned)
                        .unwrap_or_else(|| value.to_string());
                    Response { exit_code, output }
                }
                Err(error) => failure("operation_failed", error),
            }
        }
    }
}
