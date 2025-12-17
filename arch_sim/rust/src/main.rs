use std::process::{Command, Stdio};

use clap::{Parser, Subcommand};
use shell_words::split as shell_split;
use thiserror::Error;

#[derive(Parser)]
#[command(name = "arch-assist", version, about = "Lightweight Arch helper with AI-ish shortcuts")]
struct Cli {
    /// Only print the commands that would run
    #[arg(long, global = true)]
    dry_run: bool,

    /// Apply AI suggestions instead of only printing them
    #[arg(long, global = true)]
    apply: bool,

    /// Prefer paru for installs even when a -bin package is not specified
    #[arg(long, global = true)]
    prefer_paru: bool,

    /// Avoid sudo when using pacman
    #[arg(long, global = true)]
    no_sudo: bool,

    /// Log exit codes and command outcomes
    #[arg(long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Interpret a natural language prompt into real commands
    Ai { prompt: String },
    /// Run a single command after safety validation
    Run { command: String },
}

#[derive(Debug, Error)]
enum AssistError {
    #[error("unsafe command blocked: {0}")]
    Unsafe(String),
    #[error("no suggestion for prompt")]
    NoSuggestion,
    #[error("command failed: {0}")]
    CommandFailed(String),
}

fn main() -> Result<(), AssistError> {
    let cli = Cli::parse();
    let config = ExecConfig {
        dry_run: cli.dry_run,
        apply: cli.apply,
        prefer_paru: cli.prefer_paru,
        no_sudo: cli.no_sudo,
        verbose: cli.verbose,
    };

    match cli.command {
        Commands::Ai { prompt } => handle_prompt(&prompt, &config)?,
        Commands::Run { command } => {
            validate(&command)?;
            run(&command, &config)?;
        }
    }

    Ok(())
}

#[derive(Clone, Copy)]
struct ExecConfig {
    dry_run: bool,
    apply: bool,
    prefer_paru: bool,
    no_sudo: bool,
    verbose: bool,
}

fn handle_prompt(prompt: &str, config: &ExecConfig) -> Result<(), AssistError> {
    if let Some(commands) = builtin_translate(prompt, config) {
        for cmd in &commands {
            println!("{cmd}");
        }

        if !config.apply {
            // Suggest but do not run unless explicitly applied
            return Ok(());
        }

        for cmd in commands {
            validate(&cmd)?;
            run(&cmd, config)?;
        }
        return Ok(());
    }

    Err(AssistError::NoSuggestion)
}

fn installer_for(pkg: &str, config: &ExecConfig) -> &'static str {
    if config.prefer_paru || pkg.ends_with("-bin") {
        "paru"
    } else if config.no_sudo {
        "pacman"
    } else {
        "sudo pacman"
    }
}

fn builtin_translate(prompt: &str, config: &ExecConfig) -> Option<Vec<String>> {
    let lower = prompt.to_lowercase();
    let mut tokens = lower.split_whitespace();
    let first = tokens.next().unwrap_or("");
    let rest = tokens.collect::<Vec<_>>().join(" ").trim().to_string();

    if first == "install" && !rest.is_empty() {
        let installer = installer_for(&rest, config);
        return Some(vec![format!("{installer} -S --needed {rest}")]);
    }

    if ["remove", "uninstall", "delete"].contains(&first) && !rest.is_empty() {
        let installer = installer_for(&rest, config);
        let base = if installer.contains("pacman") {
            format!("{installer} -Rsn {rest}")
        } else {
            format!("{installer} -R {rest}")
        };
        return Some(vec![base]);
    }

    if ["open", "launch", "start"].contains(&first) && !rest.is_empty() {
        let installer = installer_for(&rest, config);
        return Some(vec![
            format!("{installer} -S --needed {rest}"),
            format!("{rest}"),
        ]);
    }

    if lower.contains("fix sound") || lower.contains("fix audio") || lower.contains("sound") {
        return Some(vec![
            "systemctl --user restart pipewire wireplumber".to_string(),
            "pactl info".to_string(),
        ]);
    }

    if lower.contains("fix internet") || lower.contains("fix network") || lower.contains("network") {
        return Some(vec![
            "sudo systemctl restart NetworkManager".to_string(),
            "nmcli networking on".to_string(),
            "nmcli -t -f DEVICE,STATE d".to_string(),
        ]);
    }

    None
}

fn run(cmd: &str, config: &ExecConfig) -> Result<(), AssistError> {
    println!("{cmd}");

    if config.dry_run {
        return Ok(());
    }

    let parts = shell_split(cmd).map_err(|e| AssistError::CommandFailed(format!("{cmd} ({e})")))?;
    let mut iter = parts.iter();
    let prog = iter.next().ok_or_else(|| AssistError::CommandFailed(cmd.into()))?;
    let args: Vec<&str> = iter.map(|s| s.as_str()).collect();

    let status = Command::new(prog)
        .args(&args)
        .stdin(Stdio::null())
        .spawn()
        .and_then(|mut child| child.wait())
        .map_err(|e| AssistError::CommandFailed(format!("{cmd} ({e})")))?;

    if config.verbose {
        eprintln!("-> {cmd} exited with {}", status);
    }

    if !status.success() {
        return Err(AssistError::CommandFailed(format!("{cmd} exited with {status}")));
    }

    Ok(())
}

fn validate(cmd: &str) -> Result<(), AssistError> {
    const FORBIDDEN: [&str; 12] = [
        "|", ">", "<", "&&", "||", ";", "`", "$(", "rm -rf", "mkfs", "dd ", " :",
    ];
    for bad in FORBIDDEN {
        if cmd.contains(bad) {
            return Err(AssistError::Unsafe(cmd.into()));
        }
    }

    // Minimal allowlist on the leading token
    let mut parts = cmd.split_whitespace();
    let first = parts.next().unwrap_or("");
    let allowed = ["sudo", "pacman", "paru", "systemctl", "nmcli", "pactl"];
    let allowed_program = allowed.contains(&first) || (!first.is_empty() && !first.contains('/') && !first.starts_with('-'));
    if !allowed_program {
        return Err(AssistError::Unsafe(cmd.into()));
    }

    Ok(())
}
