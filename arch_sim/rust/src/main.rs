use std::io::{self, Write};
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

    /// Auto-run AI suggestions instead of only printing them
    #[arg(long, global = true)]
    auto: bool,

    /// Require offline-safe commands (block pacman/paru downloads)
    #[arg(long, global = true)]
    offline: bool,

    /// Append --noconfirm to pacman/paru actions
    #[arg(long, global = true)]
    yes: bool,

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
        auto: cli.auto,
        offline: cli.offline,
        yes: cli.yes,
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
    auto: bool,
    offline: bool,
    yes: bool,
    prefer_paru: bool,
    no_sudo: bool,
    verbose: bool,
}

fn handle_prompt(prompt: &str, config: &ExecConfig) -> Result<(), AssistError> {
    if let Some(commands) = builtin_translate(prompt, config) {
        for sugg in &commands {
            println!("{}    # {}", sugg.cmd, sugg.reason);
        }

        if !config.auto {
            // Suggest but do not run unless explicitly requested
            return Ok(());
        }

        if !confirm(&commands, config)? {
            return Ok(());
        }

        for sugg in commands {
            ensure_offline_ok(&sugg, config)?;
            validate(&sugg.cmd)?;
            run(&sugg.cmd, config)?;
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

#[derive(Debug, Clone)]
struct Suggestion {
    cmd: String,
    reason: &'static str,
}

fn builtin_translate(prompt: &str, config: &ExecConfig) -> Option<Vec<Suggestion>> {
    let lower = prompt.to_lowercase();
    let mut tokens = lower.split_whitespace();
    let first = tokens.next().unwrap_or("");
    let rest = tokens.collect::<Vec<_>>().join(" ").trim().to_string();

    if first == "install" && !rest.is_empty() {
        let installer = installer_for(&rest, config);
        return Some(vec![install_cmd(&installer, &rest, config, "install package")]);
    }

    if ["remove", "uninstall", "delete"].contains(&first) && !rest.is_empty() {
        let installer = installer_for(&rest, config);
        let base = if installer.contains("pacman") {
            format!("{installer} -Rsn {rest}")
        } else {
            format!("{installer} -R {rest}")
        };
        return Some(vec![Suggestion {
            cmd: apply_pkg_flags(base, config),
            reason: "remove package",
        }]);
    }

    if ["open", "launch", "start"].contains(&first) && !rest.is_empty() {
        let installer = installer_for(&rest, config);
        return Some(vec![
            install_cmd(&installer, &rest, config, "ensure app is installed"),
            Suggestion {
                cmd: format!("{rest}"),
                reason: "launch app",
            },
        ]);
    }

    if lower.contains("fix sound") || lower.contains("fix audio") || lower.contains("sound") {
        return Some(vec![
            Suggestion {
                cmd: "systemctl --user restart pipewire wireplumber".to_string(),
                reason: "restart audio services",
            },
            Suggestion {
                cmd: "pactl info".to_string(),
                reason: "inspect pulse server state",
            },
        ]);
    }

    if lower.contains("fix internet") || lower.contains("fix network") || lower.contains("network") {
        return Some(vec![
            Suggestion {
                cmd: "sudo systemctl restart NetworkManager".to_string(),
                reason: "restart network manager",
            },
            Suggestion {
                cmd: "nmcli networking on".to_string(),
                reason: "enable networking",
            },
            Suggestion {
                cmd: "nmcli -t -f DEVICE,STATE d".to_string(),
                reason: "list device states",
            },
        ]);
    }

    if lower.contains("upgrade system") || lower.contains("update system") || first == "upgrade" {
        let installer = installer_for("base", config);
        let base = format!("{installer} -Syu");
        return Some(vec![Suggestion {
            cmd: apply_pkg_flags(base, config),
            reason: "upgrade system packages",
        }]);
    }

    if lower.contains("clean cache") || lower.contains("cleanup") || lower.contains("clear cache") {
        let installer = installer_for("base", config);
        let base = format!("{installer} -Sc");
        return Some(vec![Suggestion {
            cmd: apply_pkg_flags(base, config),
            reason: "clean package cache",
        }]);
    }

    if lower.contains("wifi status") || lower.contains("network status") {
        return Some(vec![
            Suggestion {
                cmd: "nmcli general status".to_string(),
                reason: "show network status",
            },
            Suggestion {
                cmd: "nmcli -t -f DEVICE,STATE d".to_string(),
                reason: "list device connectivity",
            },
        ]);
    }

    if lower.contains("fix bluetooth") || lower.contains("bluetooth") {
        return Some(vec![
            Suggestion {
                cmd: "sudo systemctl restart bluetooth".to_string(),
                reason: "restart bluetooth service",
            },
            Suggestion {
                cmd: "bluetoothctl show".to_string(),
                reason: "show bluetooth adapter state",
            },
        ]);
    }

    if ["logs", "journal"].contains(&first) && !rest.is_empty() {
        return Some(vec![Suggestion {
            cmd: format!("journalctl -u {rest} --no-pager -n 50"),
            reason: "tail service logs",
        }]);
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
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AssistError::CommandFailed(format!("{prog} not found; install or adjust PATH"))
            } else {
                AssistError::CommandFailed(format!("{cmd} ({e})"))
            }
        })?
        .wait()
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
    let allowed = [
        "sudo",
        "pacman",
        "paru",
        "systemctl",
        "nmcli",
        "pactl",
        "bluetoothctl",
        "journalctl",
    ];
    let allowed_program = allowed.contains(&first) || (!first.is_empty() && !first.contains('/') && !first.starts_with('-'));
    if !allowed_program {
        return Err(AssistError::Unsafe(cmd.into()));
    }

    Ok(())
}

fn apply_pkg_flags(cmd: String, config: &ExecConfig) -> String {
    if config.yes
        && (cmd.starts_with("sudo pacman ") || cmd.starts_with("pacman ") || cmd.starts_with("paru "))
        && !cmd.contains("--noconfirm")
    {
        return format!("{cmd} --noconfirm");
    }
    cmd
}

fn install_cmd(installer: &str, pkg: &str, config: &ExecConfig, reason: &'static str) -> Suggestion {
    Suggestion {
        cmd: apply_pkg_flags(format!("{installer} -S --needed {pkg}"), config),
        reason,
    }
}

fn confirm(_suggestions: &[Suggestion], config: &ExecConfig) -> Result<bool, AssistError> {
    if config.yes {
        return Ok(true);
    }
    print!("Run these commands? [y/N] ");
    io::stdout()
        .flush()
        .map_err(|e| AssistError::CommandFailed(format!("confirm ({e})")))?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| AssistError::CommandFailed(format!("confirm ({e})")))?;
    Ok(matches!(input.trim(), "y" | "Y" | "yes" | "YES"))
}

fn ensure_offline_ok(suggestion: &Suggestion, config: &ExecConfig) -> Result<(), AssistError> {
    if !config.offline {
        return Ok(());
    }
    let cmd = suggestion.cmd.as_str();
    let is_pkg_op = cmd.contains(" pacman -S")
        || cmd.contains(" pacman -Syu")
        || cmd.contains("paru -S")
        || cmd.starts_with("pacman -S")
        || cmd.starts_with("paru -S")
        || cmd.starts_with("sudo pacman -S");
    if is_pkg_op {
        return Err(AssistError::Unsafe(format!(
            "offline mode: blocked network command: {}",
            suggestion.cmd
        )));
    }
    Ok(())
}
