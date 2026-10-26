// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! `opentrack-plugin-scan` entry point.
//!
//! This build performs folder discovery and a PE architecture precheck only.
//! It does not load plug-in code or enumerate classes. See the crate library
//! docs and `docs/plugin-hosting-plan.md` section 4.12.

use std::ffi::OsStr;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

use opentrack_plugin_scan::DiscoveryLimits;
use opentrack_plugin_scan::SCANNER_VERSION;
use opentrack_plugin_scan::default_vst3_folders;
use opentrack_plugin_scan::scan_folders;

const USAGE: &str = "\
opentrack-plugin-scan - bounded VST3 discovery and PE precheck

Usage:
  opentrack-plugin-scan [FOLDER...] [OPTIONS]

Configured folders:
  --folder PATH           Add a VST3 folder (repeatable). Positional FOLDER
                          arguments are treated the same way.
  --no-default-folders    Do not add the two default per-machine folders.

Output:
  --json                  Emit JSON instead of text.
  --text                  Emit text (default).
  --format FORMAT         json or text.
  --output PATH           Write the report to PATH instead of stdout.

Limits:
  --max-modules N         Maximum module paths retained (default 4096).
  --max-depth N           Maximum folder recursion depth (default 32).

Other:
  -h, --help              Show this help.
  -V, --version           Show the scanner version.

This build does not load VST3 modules or enumerate classes. x64 modules are
reported as ready_for_enumeration after the PE precheck; no plug-in hosting is
implemented or claimed.
";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OutputFormat {
    Text,
    Json,
}

struct Cli {
    folders: Vec<PathBuf>,
    use_default_folders: bool,
    format: OutputFormat,
    output: Option<PathBuf>,
    limits: DiscoveryLimits,
    help: bool,
    version: bool,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("opentrack-plugin-scan: {error}");
            eprintln!("Try 'opentrack-plugin-scan --help'.");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let cli = parse_args(std::env::args_os().skip(1))?;
    if cli.help {
        print!("{USAGE}");
        return Ok(());
    }
    if cli.version {
        println!("opentrack-plugin-scan {SCANNER_VERSION}");
        return Ok(());
    }

    let mut folders = Vec::new();
    if cli.use_default_folders {
        folders.extend(default_vst3_folders());
    }
    for folder in cli.folders {
        if !folders.contains(&folder) {
            folders.push(folder);
        }
    }

    if folders.is_empty() {
        return Err(
            "no VST3 folders configured; pass --folder PATH or omit --no-default-folders"
                .to_owned(),
        );
    }

    let report = scan_folders(&folders, cli.limits);
    let serialized = match cli.format {
        OutputFormat::Text => report.to_text(),
        OutputFormat::Json => report
            .to_json_pretty()
            .map(|mut json| {
                json.push('\n');
                json
            })
            .map_err(|error| format!("cannot serialize report: {error}"))?,
    };

    match cli.output {
        Some(path) => std::fs::write(&path, serialized)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?,
        None => {
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            handle
                .write_all(serialized.as_bytes())
                .map_err(|error| format!("cannot write report: {error}"))?;
        }
    }
    Ok(())
}

fn parse_args(mut args: impl Iterator<Item = std::ffi::OsString>) -> Result<Cli, String> {
    let mut cli = Cli {
        folders: Vec::new(),
        use_default_folders: true,
        format: OutputFormat::Text,
        output: None,
        limits: DiscoveryLimits::default(),
        help: false,
        version: false,
    };

    while let Some(argument) = args.next() {
        if argument == OsStr::new("--folder") {
            let Some(path) = args.next() else {
                return Err("--folder requires a path".to_owned());
            };
            cli.folders.push(PathBuf::from(path));
            continue;
        }
        if argument == OsStr::new("--output") {
            let Some(path) = args.next() else {
                return Err("--output requires a path".to_owned());
            };
            cli.output = Some(PathBuf::from(path));
            continue;
        }
        if argument == OsStr::new("--format") {
            let Some(format) = args.next() else {
                return Err("--format requires json or text".to_owned());
            };
            cli.format = parse_format(&format)?;
            continue;
        }
        if argument == OsStr::new("--max-modules") {
            let value = required_value(&mut args, "--max-modules")?;
            cli.limits.max_modules = parse_positive_usize(&value, "--max-modules")?;
            continue;
        }
        if argument == OsStr::new("--max-depth") {
            let value = required_value(&mut args, "--max-depth")?;
            cli.limits.max_depth = parse_usize(&value, "--max-depth")?;
            continue;
        }

        match argument.to_str() {
            Some("--no-default-folders") => cli.use_default_folders = false,
            Some("--json") => cli.format = OutputFormat::Json,
            Some("--text") => cli.format = OutputFormat::Text,
            Some("-h" | "--help") => cli.help = true,
            Some("-V" | "--version") => cli.version = true,
            Some(flag) if flag.starts_with('-') => {
                return Err(format!("unknown option {flag}"));
            }
            _ => cli.folders.push(PathBuf::from(argument)),
        }
    }

    Ok(cli)
}

fn required_value(
    args: &mut impl Iterator<Item = std::ffi::OsString>,
    option: &str,
) -> Result<std::ffi::OsString, String> {
    args.next()
        .ok_or_else(|| format!("{option} requires a value"))
}

fn parse_format(value: &OsStr) -> Result<OutputFormat, String> {
    match value.to_str() {
        Some(value) if value.eq_ignore_ascii_case("json") => Ok(OutputFormat::Json),
        Some(value) if value.eq_ignore_ascii_case("text") => Ok(OutputFormat::Text),
        _ => Err(format!(
            "invalid --format {:?}; expected json or text",
            value
        )),
    }
}

fn parse_usize(value: &OsStr, option: &str) -> Result<usize, String> {
    value
        .to_str()
        .and_then(|value| value.parse::<usize>().ok())
        .ok_or_else(|| format!("{option} requires a non-negative integer"))
}

fn parse_positive_usize(value: &OsStr, option: &str) -> Result<usize, String> {
    let value = parse_usize(value, option)?;
    if value == 0 {
        return Err(format!("{option} must be at least 1"));
    }
    Ok(value)
}
