//! `skaldsaga`: the command-line front end for Skaldsaga's deterministic scripts.
//!
//! One binary with subcommands. First-run setup, the session meter, the
//! session-start hook, the index rebuild, the dependency checker, the
//! committer, the demo and `complete` (with the writer's stored decisions) are
//! built so far.

mod wrapped;

use std::io::{self, Read as _, Write as _};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use skaldsaga_core::check::{self, InboxEntry};
use skaldsaga_core::commit::{self, CommitOptions, Decision, Outcome};
use skaldsaga_core::complete;
use skaldsaga_core::decisions::{self, DecisionError};
use skaldsaga_core::demo::{self, DemoOutcome};
use skaldsaga_core::fsio::{self, Refused};
use skaldsaga_core::hooks;
use skaldsaga_core::index;
use skaldsaga_core::init::{self, InitOutcome, InitReport};
use skaldsaga_core::json::{self, Value, ValueExt as _};
use skaldsaga_core::meter::{self, StatusInput};
use skaldsaga_core::project::{self, Location, Project};
use skaldsaga_core::safety;
use skaldsaga_core::settings::{self, MeterSettings};
use skaldsaga_core::statusline::{self, WireError, WireOutcome};

const USAGE: &str = "\
skaldsaga: deterministic scripts for the Skaldsaga story mentor

Usage:
  skaldsaga init [--project DIR] [--meter-command CMD] [--user-settings FILE]
                                     First-run setup for a fresh project: story folders,
                                     .harness and default settings. Safe to run again.
                                     With --meter-command, also wires the session meter.
                                     Prints what it did as JSON
  skaldsaga meter statusline         Status line command: reads Claude Code's JSON on
                                     stdin, records context usage, prints a status line
  skaldsaga meter check [--session ID] [--project DIR]
                                     Prints the meter state as JSON
  skaldsaga meter reset [--project DIR]
                                     Clears the last reading (for a new session)
  skaldsaga meter wire --command CMD [--project DIR] [--user-settings FILE]
                                     Wires the meter into the project's personal status line
                                     (.claude/settings.local.json)
  skaldsaga locate [--project DIR]   Finds the story folder: DIR itself, or the one child
                                     folder that is a project. Prints JSON
  skaldsaga hook session-start       SessionStart hook: reads Claude Code's JSON on stdin;
                                     in a Skaldsaga project, prepares the session and adds
                                     context for Claude. Elsewhere it prints nothing. With
                                     SKALDSAGA_HOOK_DEBUG=1, says why (stderr and context)
  skaldsaga check [--project DIR] [--proposal ID]...
                                     Checks pending proposals in .harness/inbox (all, or the
                                     ones named) against the spec and writes each verdict into
                                     its file. Changes nothing in the spec. Prints JSON
  skaldsaga commit --proposal ID [--decision keep_anyway|not_sure] [--cleared] [--project DIR]
                                     Re-checks one proposal and, if it may commit, writes it into
                                     the spec, archives it in .harness/committed and rebuilds the
                                     index. Prints JSON, including the paths to hand to git
  skaldsaga demo [--project DIR]     Files two sample answers in a set-up project with an empty
                                     story, for trying check and commit
  skaldsaga complete [--project DIR] Reports how close the whole story is, as codes and IDs:
                                     whether each stage can start (weave, outline, complete)
                                     and what blocks it, plus the soft findings from the
                                     script checks. Deletes the writer's acknowledgments
                                     that are resolved. Prints JSON, including paths for git
  skaldsaga decisions acknowledge --code CODE --words-file FILE [--ids ID,ID] [--detail TEXT]
                                     [--project DIR]
                                     Records that the writer accepts a current finding as it
                                     is. The writer's own words come from FILE. Prints JSON
  skaldsaga decisions withdraw --decision ID [--project DIR]
                                     Deletes one of the writer's stored decisions
  skaldsaga index [--project DIR]    Rebuilds the generated index in spec/_index (backlinks,
                                     attributes, grid, scenes, status). Prints a summary as JSON
  skaldsaga --version                Prints the version

DIR defaults to $CLAUDE_PROJECT_DIR, then the current folder.
";

/// Exit code for usage mistakes.
const USAGE_ERROR: u8 = 64;
/// Exit code for a failure while doing the work.
const FAILURE: u8 = 1;

/// Upper bound on status line input; the real payload is a few kilobytes.
const MAX_STDIN: u64 = 4 * 1024 * 1024;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [] | ["help" | "--help" | "-h"] => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        ["--version" | "-V" | "version"] => {
            println!("skaldsaga {}", skaldsaga_core::VERSION);
            ExitCode::SUCCESS
        }
        ["meter", "statusline", ..] => meter_statusline(),
        ["meter", "check", rest @ ..] => run(meter_check(rest)),
        ["meter", "reset", rest @ ..] => run(meter_reset(rest)),
        ["meter", "wire", rest @ ..] => run(meter_wire(rest)),
        ["hook", "session-start", ..] => hook_session_start(),
        ["locate", rest @ ..] => run(locate_cmd(rest)),
        ["init", rest @ ..] => run(init_cmd(rest)),
        ["index", rest @ ..] => run(index_cmd(rest)),
        ["check", rest @ ..] => run(check_cmd(rest)),
        ["commit", rest @ ..] => run(commit_cmd(rest)),
        ["demo", rest @ ..] => run(demo_cmd(rest)),
        ["complete", rest @ ..] => run(complete_cmd(rest)),
        ["decisions", "acknowledge", rest @ ..] => run(acknowledge_cmd(rest)),
        ["decisions", "withdraw", rest @ ..] => run(withdraw_cmd(rest)),
        _ => {
            eprint!("skaldsaga: unknown command\n\n{USAGE}");
            ExitCode::from(USAGE_ERROR)
        }
    }
}

enum CliError {
    Usage(String),
    Failed(String),
}

fn run(result: Result<(), CliError>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(CliError::Usage(msg)) => {
            eprintln!("skaldsaga: {msg}\n\n{USAGE}");
            ExitCode::from(USAGE_ERROR)
        }
        Err(CliError::Failed(msg)) => {
            eprintln!("skaldsaga: {msg}");
            ExitCode::from(FAILURE)
        }
    }
}

/// Parses `--flag value` pairs, allowing only the given flags.
fn flags<'a>(rest: &[&'a str], allowed: &[&str]) -> Result<Vec<(&'a str, &'a str)>, CliError> {
    let mut out = Vec::new();
    let mut it = rest.iter();
    while let Some(&flag) = it.next() {
        if !allowed.contains(&flag) {
            return Err(CliError::Usage(format!("unexpected argument '{flag}'")));
        }
        let Some(&value) = it.next() else {
            return Err(CliError::Usage(format!("{flag} needs a value")));
        };
        out.push((flag, value));
    }
    Ok(out)
}

fn flag<'a>(parsed: &[(&str, &'a str)], name: &str) -> Option<&'a str> {
    parsed
        .iter()
        .rev()
        .find(|(f, _)| *f == name)
        .map(|(_, v)| *v)
}

fn default_project_dir() -> PathBuf {
    std::env::var_os("CLAUDE_PROJECT_DIR")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn project_from(parsed: &[(&str, &str)]) -> Project {
    Project::new(flag(parsed, "--project").map_or_else(default_project_dir, PathBuf::from))
}

/// The writer's personal Claude Code settings file.
fn default_user_settings() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("CLAUDE_CONFIG_DIR").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(dir).join("settings.json"));
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .filter(|v| !v.is_empty())?;
    Some(PathBuf::from(home).join(".claude").join("settings.json"))
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn print_json(v: &Value) -> Result<(), CliError> {
    let mut out = io::stdout().lock();
    out.write_all(v.to_pretty_string().as_bytes())
        .map_err(|e| CliError::Failed(e.to_string()))
}

/// The status line command. It must never fail visibly: whatever happens, it
/// prints a line (possibly empty) and exits successfully.
fn meter_statusline() -> ExitCode {
    let mut raw = String::new();
    let _ = io::stdin().lock().take(MAX_STDIN).read_to_string(&mut raw);
    let input = StatusInput::parse(&raw);

    let project = input
        .as_ref()
        .and_then(|i| i.project_dir.as_deref())
        .filter(|d| !d.is_empty())
        .map_or_else(|| Project::new(default_project_dir()), Project::new);

    if !project.is_initialized() {
        // Not a Skaldsaga project: stay out of the way.
        println!();
        return ExitCode::SUCCESS;
    }

    let reading = input.as_ref().map(|i| i.to_reading(now()));
    if let Some(reading) = &reading {
        // A failed write only means a missing reading; the manual session boundary still works.
        let _ = meter::write_reading(&project, reading);
    }

    let settings = MeterSettings::load(&project).unwrap_or_default();
    let line = settings
        .wrapped_statusline
        .as_ref()
        .and_then(|w| w.get("command").and_then(Value::as_str))
        .and_then(|cmd| wrapped::run(cmd, &raw, project.root()))
        .unwrap_or_else(|| {
            let current = input.as_ref().and_then(|i| i.session_id.as_deref());
            format!(
                "{}\n",
                meter::evaluate(reading.as_ref(), &settings, current).status_text()
            )
        });
    let mut out = io::stdout().lock();
    let _ = out.write_all(line.as_bytes());
    let _ = out.flush();
    ExitCode::SUCCESS
}

fn meter_check(rest: &[&str]) -> Result<(), CliError> {
    let parsed = flags(rest, &["--session", "--project"])?;
    let project = project_from(&parsed);
    let settings = MeterSettings::load(&project).unwrap_or_default();
    let reading = meter::read_reading(&project);
    let eval = meter::evaluate(reading.as_ref(), &settings, flag(&parsed, "--session"));
    print_json(&eval.to_json())
}

fn meter_reset(rest: &[&str]) -> Result<(), CliError> {
    let parsed = flags(rest, &["--project"])?;
    let project = project_from(&parsed);
    if !project.is_initialized() {
        // The SessionStart hook calls this in every folder; outside a project it does nothing.
        return Ok(());
    }
    meter::reset(&project)
        .map(|_| ())
        .map_err(|e| CliError::Failed(e.to_string()))
}

fn meter_wire(rest: &[&str]) -> Result<(), CliError> {
    let parsed = flags(rest, &["--command", "--project", "--user-settings"])?;
    let command = flag(&parsed, "--command")
        .ok_or_else(|| CliError::Usage("--command is required".into()))?;
    if !command.contains(statusline::METER_SUBCOMMAND) {
        return Err(CliError::Usage(format!(
            "--command must run '{}'",
            statusline::METER_SUBCOMMAND
        )));
    }
    let project = project_from(&parsed);
    let user = flag(&parsed, "--user-settings")
        .map(PathBuf::from)
        .or_else(default_user_settings);

    print_json(&wire_meter(&project, command, user.as_deref())?)
}

/// Wires the meter and describes the outcome as JSON.
fn wire_meter(
    project: &Project,
    command: &str,
    user: Option<&std::path::Path>,
) -> Result<Value, CliError> {
    // An entry an earlier version left in the shared settings goes first,
    // even when the personal file is already wired.
    let retired =
        statusline::retire_shared(project).map_err(|e| CliError::Failed(e.to_string()))?;
    // Likewise a status line command older versions kept in the committed
    // settings: it moves to this machine's file and is never run from there.
    let moved =
        settings::retire_committed_wrapped(project).map_err(|e| CliError::Failed(e.to_string()))?;
    let outcome = statusline::wire(project, command, user).map_err(|e| match e {
        WireError::NotInitialized => CliError::Failed(e.to_string()),
        WireError::Fs(e) => CliError::Failed(e.to_string()),
    })?;

    let mut v = json::object();
    match outcome {
        WireOutcome::AlreadyWired => v.set("outcome", "already_wired".into()),
        WireOutcome::Updated => v.set("outcome", "updated".into()),
        WireOutcome::Wired { wrapped } => {
            v.set("outcome", "wired".into());
            v.set("wrapped", wrapped.unwrap_or(Value::Null));
        }
    }
    // Files the skill should add to git if git already tracks them (an edit or
    // a removal of the shared settings file).
    let mut cleaned: Vec<String> = retired
        .map(|_| vec![".claude/settings.json".to_string()])
        .unwrap_or_default();
    if moved {
        cleaned.push(".harness/settings.json".to_string());
        cleaned.push(".harness/.gitignore".to_string());
    }
    v.set("cleaned", strings(&cleaned));
    Ok(v)
}

/// Links, executable files and special files in the story folder, for the
/// skill to tell the writer about (SR-11). A scan that can't run reports null.
fn unsafe_files(project: &Project) -> Value {
    safety::scan(project)
        .map(|s| s.to_value())
        .unwrap_or(Value::Null)
}

fn strings(items: &[String]) -> Value {
    Value::Array(items.iter().map(|s| Value::from(s.as_str())).collect())
}

fn init_cmd(rest: &[&str]) -> Result<(), CliError> {
    let parsed = flags(rest, &["--project", "--meter-command", "--user-settings"])?;
    let meter_command = flag(&parsed, "--meter-command");
    if let Some(cmd) = meter_command
        && !cmd.contains(statusline::METER_SUBCOMMAND)
    {
        return Err(CliError::Usage(format!(
            "--meter-command must run '{}'",
            statusline::METER_SUBCOMMAND
        )));
    }
    let project = project_from(&parsed);
    let report = init::init(&project).map_err(|e| CliError::Failed(e.to_string()))?;

    let mut v = json::object();
    v.set("outcome", report.outcome.as_str().into());
    if report.outcome == InitOutcome::ExistingFiles {
        v.set("existing", strings(&report.existing));
        v.set("existing_count", Value::from(report.existing_count));
    } else {
        v.set("created", strings(&report.created));
        v.set("commit_paths", strings(&InitReport::commit_paths()));
        let meter = match meter_command {
            Some(cmd) => {
                let user = flag(&parsed, "--user-settings")
                    .map(PathBuf::from)
                    .or_else(default_user_settings);
                wire_meter(&project, cmd, user.as_deref())?
            }
            None => Value::Null,
        };
        v.set("meter", meter);
        v.set("unsafe_files", unsafe_files(&project));
    }
    print_json(&v)
}

/// Rebuilds `spec/_index/`. Refuses outside a Skaldsaga project, so it never
/// writes into a folder the writer hasn't set up.
fn index_cmd(rest: &[&str]) -> Result<(), CliError> {
    let parsed = flags(rest, &["--project"])?;
    let project = project_from(&parsed);
    if !project.is_initialized() {
        return Err(CliError::Failed(
            "this folder isn't set up for Skaldsaga yet (no .harness folder)".into(),
        ));
    }
    let rebuilt =
        index::rebuild(&project.spec_dir()).map_err(|e| CliError::Failed(e.to_string()))?;
    let mut v = json::object();
    v.set("outcome", "rebuilt".into());
    v.set("files", strings(&rebuilt.files));
    v.set("nodes", Value::from(rebuilt.nodes));
    v.set("problems", Value::from(rebuilt.problems));
    v.set("broken_references", Value::from(rebuilt.broken_references));
    v.set("unsafe_files", unsafe_files(&project));
    print_json(&v)
}

/// Checks pending proposals. Refuses outside a Skaldsaga project.
fn check_cmd(rest: &[&str]) -> Result<(), CliError> {
    let parsed = flags(rest, &["--project", "--proposal"])?;
    let project = project_from(&parsed);
    if !project.is_initialized() {
        return Err(CliError::Failed(
            "this folder isn't set up for Skaldsaga yet (no .harness folder)".into(),
        ));
    }
    let only: Vec<String> = parsed
        .iter()
        .filter(|(f, _)| *f == "--proposal")
        .map(|(_, v)| v.to_string())
        .collect();
    let report =
        check::check_inbox(&project, &only).map_err(|e| CliError::Failed(e.to_string()))?;
    let mut list = Vec::new();
    for entry in &report.entries {
        let mut v = json::object();
        match entry {
            InboxEntry::Checked { id, report } => {
                v.set("id", id.as_str().into());
                let r = report.to_value();
                for key in ["status", "lane", "fan_out", "touches_confirmed", "findings"] {
                    v.set(key, r[key].clone());
                }
            }
            InboxEntry::Unreadable { file, message } => {
                v.set("file", file.as_str().into());
                v.set("status", "unreadable".into());
                v.set("message", message.as_str().into());
            }
        }
        list.push(v);
    }
    let missing: Vec<&String> = only
        .iter()
        .filter(|id| {
            !report.entries.iter().any(|e| match e {
                InboxEntry::Checked { id: got, .. } => got == *id,
                InboxEntry::Unreadable { file, .. } => {
                    file.strip_suffix(".json") == Some(id.as_str())
                }
            })
        })
        .collect();
    let mut v = json::object();
    v.set("outcome", "checked".into());
    v.set("spec_problems", Value::from(report.spec_problems));
    v.set("proposals", Value::Array(list));
    v.set("unsafe_files", unsafe_files(&project));
    if !missing.is_empty() {
        v.set(
            "not_found",
            Value::Array(missing.iter().map(|s| Value::from(s.as_str())).collect()),
        );
    }
    print_json(&v)
}

fn require_project(parsed: &[(&str, &str)]) -> Result<Project, CliError> {
    let project = project_from(parsed);
    if !project.is_initialized() {
        return Err(CliError::Failed(
            "this folder isn't set up for Skaldsaga yet (no .harness folder)".into(),
        ));
    }
    Ok(project)
}

/// Copies the verdict fields a skill needs from a check report.
fn verdict_into(v: &mut Value, report: &check::Report) {
    let r = report.to_value();
    for key in ["status", "lane", "fan_out", "touches_confirmed", "findings"] {
        v.set(key, r[key].clone());
    }
}

/// Commits one proposal.
fn commit_cmd(rest: &[&str]) -> Result<(), CliError> {
    let cleared = rest.contains(&"--cleared");
    let rest: Vec<&str> = rest.iter().copied().filter(|a| *a != "--cleared").collect();
    let parsed = flags(&rest, &["--project", "--proposal", "--decision"])?;
    let Some(id) = flag(&parsed, "--proposal") else {
        return Err(CliError::Usage("commit needs --proposal ID".into()));
    };
    let decision =
        match flag(&parsed, "--decision") {
            None => None,
            Some(d) => Some(Decision::parse(d).ok_or_else(|| {
                CliError::Usage("--decision must be keep_anyway or not_sure".into())
            })?),
        };
    let project = require_project(&parsed)?;
    let outcome = commit::commit(&project, id, CommitOptions { decision, cleared })
        .map_err(|e| CliError::Failed(e.to_string()))?;
    let mut v = json::object();
    match outcome {
        Outcome::Committed {
            files,
            add_paths,
            remove_paths,
            message,
            report,
            decision,
        } => {
            v.set("outcome", "committed".into());
            v.set("id", id.into());
            verdict_into(&mut v, &report);
            v.set(
                "decision",
                decision.map_or(Value::Null, |d| d.as_str().into()),
            );
            v.set("files", strings(&files));
            v.set("add_paths", strings(&add_paths));
            v.set("remove_paths", strings(&remove_paths));
            v.set("message", message.into());
        }
        Outcome::Held { reason, report } => {
            v.set("outcome", "held".into());
            v.set("id", id.into());
            v.set("reason", reason.as_str().into());
            verdict_into(&mut v, &report);
        }
        Outcome::NotFound => {
            v.set("outcome", "not_found".into());
            v.set("id", id.into());
        }
        Outcome::Unreadable(message) => {
            v.set("outcome", "unreadable".into());
            v.set("id", id.into());
            v.set("message", message.into());
        }
    }
    print_json(&v)
}

/// Files the sample answers.
fn demo_cmd(rest: &[&str]) -> Result<(), CliError> {
    let parsed = flags(rest, &["--project"])?;
    let project = require_project(&parsed)?;
    let outcome = demo::write(&project).map_err(|e| CliError::Failed(e.to_string()))?;
    let mut v = json::object();
    v.set(
        "outcome",
        match outcome {
            DemoOutcome::Written => "written",
            DemoOutcome::SpecNotEmpty => "spec_not_empty",
            DemoOutcome::AlreadyWritten => "already_written",
        }
        .into(),
    );
    v.set(
        "proposals",
        Value::Array(demo::PROPOSALS.iter().map(|p| Value::from(*p)).collect()),
    );
    print_json(&v)
}

/// Reports how close the story is. Refuses outside a Skaldsaga project.
fn complete_cmd(rest: &[&str]) -> Result<(), CliError> {
    let parsed = flags(rest, &["--project"])?;
    let project = require_project(&parsed)?;
    let report = complete::run(&project).map_err(|e| CliError::Failed(e.to_string()))?;
    let mut v = report.to_value();
    v.set("unsafe_files", unsafe_files(&project));
    if !report.remove_paths.is_empty() {
        v.set(
            "message",
            "Skaldsaga: clear acknowledgments that no longer apply".into(),
        );
    }
    print_json(&v)
}

/// The longest file of the writer's words we read, in bytes.
const MAX_WORDS_FILE: u64 = 16 * 1024;

fn decision_error(e: DecisionError) -> CliError {
    match e {
        DecisionError::Invalid(m) => CliError::Usage(m),
        DecisionError::Fs(e) => CliError::Failed(e.to_string()),
    }
}

/// Records that the writer accepts a finding. The writer's words come from a
/// file, never from the command line, so they can't be read as part of a command.
fn acknowledge_cmd(rest: &[&str]) -> Result<(), CliError> {
    let parsed = flags(
        rest,
        &["--project", "--code", "--ids", "--detail", "--words-file"],
    )?;
    let code =
        flag(&parsed, "--code").ok_or_else(|| CliError::Usage("--code is required".into()))?;
    let words_file = flag(&parsed, "--words-file")
        .ok_or_else(|| CliError::Usage("--words-file is required".into()))?;
    let ids: Vec<String> = flag(&parsed, "--ids")
        .map(|list| {
            list.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let project = require_project(&parsed)?;
    let words = match fsio::read_regular_capped(std::path::Path::new(words_file), MAX_WORDS_FILE)
        .map_err(|e| CliError::Failed(e.to_string()))?
    {
        Some(Ok(text)) => text,
        Some(Err(Refused::TooBig)) => {
            return Err(CliError::Usage("the words file is too big".into()));
        }
        Some(Err(Refused::NotARegularFile)) => {
            return Err(CliError::Usage(
                "the words file isn't a regular text file".into(),
            ));
        }
        None => return Err(CliError::Usage("the words file doesn't exist".into())),
    };
    let date = decisions::date_from_unix(now());
    let ack = complete::acknowledge(
        &project,
        code,
        &ids,
        flag(&parsed, "--detail"),
        &words,
        &date,
    )
    .map_err(decision_error)?;
    let mut v = json::object();
    v.set("outcome", "acknowledged".into());
    v.set("id", ack.decision.id.as_str().into());
    v.set("renewed", Value::from(ack.renewed));
    v.set("add_paths", strings(&[ack.path]));
    v.set(
        "message",
        "Skaldsaga: note that a finding is fine as it is".into(),
    );
    print_json(&v)
}

/// Deletes one stored decision (the writer withdraws it).
fn withdraw_cmd(rest: &[&str]) -> Result<(), CliError> {
    let parsed = flags(rest, &["--project", "--decision"])?;
    let id = flag(&parsed, "--decision")
        .ok_or_else(|| CliError::Usage("--decision is required".into()))?;
    let project = require_project(&parsed)?;
    let removed = decisions::remove(&project, id).map_err(decision_error)?;
    let mut v = json::object();
    match removed {
        Some(path) => {
            v.set("outcome", "withdrawn".into());
            v.set("remove_paths", strings(&[path]));
            v.set("message", "Skaldsaga: withdraw an acknowledgment".into());
        }
        None => {
            v.set("outcome", "not_found".into());
            v.set("remove_paths", strings(&[]));
        }
    }
    v.set("id", id.into());
    print_json(&v)
}

/// Finds the story folder from the session's folder (or `--project`), with
/// the same rule the session-start hook uses.
fn locate_cmd(rest: &[&str]) -> Result<(), CliError> {
    let parsed = flags(rest, &["--project"])?;
    let start = flag(&parsed, "--project").map_or_else(default_project_dir, PathBuf::from);
    let found = project::locate(&start);
    let mut v = json::object();
    v.set("found", found.as_str().into());
    let (path, name) = match &found {
        Location::Here(p) => (Some(p.clone()), None),
        Location::Below { path, name } | Location::Candidate { path, name } => {
            (Some(path.clone()), Some(name.clone()))
        }
        Location::Several(_) | Location::Nothing => (None, None),
    };
    v.set(
        "path",
        path.map_or(Value::Null, |p| p.display().to_string().into()),
    );
    v.set("name", name.map_or(Value::Null, Value::from));
    if let Location::Several(names) = &found {
        v.set("several", strings(names));
    }
    print_json(&v)
}

/// The SessionStart hook. Like the status line, it must never get in the
/// writer's way: it always exits successfully, and prints nothing outside a
/// Skaldsaga project. With `SKALDSAGA_HOOK_DEBUG=1` it also says what it
/// found or why it stayed silent, both on stderr and in Claude's context
/// (stderr isn't shown to Claude at session start).
fn hook_session_start() -> ExitCode {
    let mut raw = String::new();
    let _ = io::stdin().lock().take(MAX_STDIN).read_to_string(&mut raw);
    let input = json::parse(&raw).ok();
    let field = |key: &str| {
        input
            .as_ref()
            .and_then(|v| v.get(key))
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };

    // Claude Code exports CLAUDE_PROJECT_DIR to hooks; fall back to the cwd it reports.
    let start = std::env::var_os("CLAUDE_PROJECT_DIR")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| field("cwd").map(PathBuf::from))
        .unwrap_or_else(default_project_dir);
    let source = field("source").unwrap_or_else(|| "startup".to_string());
    let plugin_root = std::env::var("CLAUDE_PLUGIN_ROOT").ok();

    let result = hooks::session_start(&start, &source, plugin_root.as_deref());
    let debug = std::env::var_os("SKALDSAGA_HOOK_DEBUG").is_some_and(|v| v == "1");
    if debug {
        eprintln!("skaldsaga hook: {}", result.reason);
    }
    let Some(context) = result.context_for(debug) else {
        return ExitCode::SUCCESS;
    };
    let mut specific = json::object();
    specific.set("hookEventName", "SessionStart".into());
    specific.set("additionalContext", context.into());
    let mut out = json::object();
    out.set("hookSpecificOutput", specific);
    println!("{}", out.to_compact_string());
    ExitCode::SUCCESS
}
