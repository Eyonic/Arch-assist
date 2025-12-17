use std::io::{self, Write};
use std::process::{Command, Stdio};

use clap::{Parser, Subcommand};
use reqwest::blocking::Client as HttpClient;
use serde::{Deserialize, Serialize};
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

    // Fall back to OpenAI suggestion
    let llm_cmds = llm_translate(prompt, config)?;
    for cmd in &llm_cmds {
        println!("{cmd}    # from openai");
    }

    if !config.auto {
        return Ok(());
    }

    if !confirm(
        &llm_cmds
            .iter()
            .map(|c| Suggestion {
                cmd: c.clone(),
                reason: "LLM suggestion",
            })
            .collect::<Vec<_>>(),
        config,
    )? {
        return Ok(());
    }

    for cmd in llm_cmds {
        let sugg = Suggestion {
            cmd: cmd.clone(),
            reason: "LLM suggestion",
        };
        ensure_offline_ok(&sugg, config)?;
        validate(&sugg.cmd)?;
        run(&sugg.cmd, config)?;
    }

    Ok(())
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

    if lower == "test ai" {
        return Some(vec![Suggestion {
            cmd: "echo ai-ok".to_string(),
            reason: "built-in test command",
        }]);
    }

    if first == "install" && !rest.is_empty() {
        // Defer to LLM unless offline; offline falls back to literal pkg name.
        if config.offline {
            let installer = installer_for(&rest, config);
            return Some(vec![install_cmd(
                &installer,
                &rest,
                config,
                "install package",
            )]);
        }

        return None;
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
        if config.offline {
            if let Some(install) = build_install_command(&rest, "-S --needed", config) {
                return Some(vec![
                    Suggestion {
                        cmd: install,
                        reason: "ensure app is installed",
                    },
                    Suggestion {
                        cmd: rest.clone(),
                        reason: "launch app",
                    },
                ]);
            }
            // fallback to previous behavior if resolution failed
            let installer = installer_for(&rest, config);
            return Some(vec![
                install_cmd(&installer, &rest, config, "ensure app is installed"),
                Suggestion {
                    cmd: format!("{rest}"),
                    reason: "launch app",
                },
            ]);
        }

        // Non-offline: let LLM handle fuzzy package mapping
        return None;
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

    if lower.contains("fix time") || lower.contains("time sync") || lower.contains("clock") {
        return Some(vec![
            Suggestion {
                cmd: "sudo timedatectl set-ntp true".to_string(),
                reason: "enable NTP sync",
            },
            Suggestion {
                cmd: "timedatectl status".to_string(),
                reason: "show time sync status",
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
        "timedatectl",
        "echo",
        "launch",
    ];
    let allowed_program = allowed.contains(&first);
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

fn llm_translate(prompt: &str, config: &ExecConfig) -> Result<Vec<String>, AssistError> {
    if config.offline {
        return Err(AssistError::CommandFailed(
            "offline mode: LLM suggestions disabled".into(),
        ));
    }

    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| AssistError::CommandFailed("OPENAI_API_KEY not set".into()))?;

    let client = HttpClient::new();
    let system_prompt = "You are an Arch Linux expert. Respond with ONLY shell commands, one per line. Use pacman for repo packages; use paru for AUR packages (e.g., *-bin). Do not suggest generic shells (bash/sh) as commands. Never use dangerous operators (rm, dd, mkfs, pipes, redirects). Keep responses concise and focused on the requested task.";
    let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());

    let req_body = ChatRequest {
        model,
        max_completion_tokens: Some(150),
        temperature: Some(1.0),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: vec![ChatContent {
                    kind: "text".to_string(),
                    text: system_prompt.to_string(),
                }],
            },
            ChatMessage {
                role: "user".to_string(),
                content: vec![ChatContent {
                    kind: "text".to_string(),
                    text: prompt.to_string(),
                }],
            },
        ],
    };

    let resp: ChatResponse = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&req_body)
        .send()
        .map_err(|e| AssistError::CommandFailed(format!("llm call ({e})")))?
        .error_for_status()
        .map_err(|e| AssistError::CommandFailed(format!("llm call ({e})")))?
        .json()
        .map_err(|e| AssistError::CommandFailed(format!("llm decode ({e})")))?;

    if resp.choices.is_empty() {
        return Err(AssistError::CommandFailed(
            "LLM returned no choices".into(),
        ));
    }

    let content_raw = resp
        .choices
        .first()
        .and_then(|c| c.message.content.clone())
        .ok_or_else(|| AssistError::CommandFailed("LLM returned no content".into()))?;

    if config.verbose {
        eprintln!("LLM raw content: {}", content_raw);
    }

    let content = content_raw.trim();
    if content.is_empty() {
        return Err(AssistError::CommandFailed(
            "LLM returned only whitespace".into(),
        ));
    }

    use std::collections::HashSet;
    let mut seen = HashSet::new();
    let mut cmds: Vec<String> = Vec::new();
    for line in content.lines() {
        let clean = line.trim_matches('`').trim();
        if clean.is_empty() {
            continue;
        }
        if seen.insert(clean.to_string()) {
            cmds.push(clean.to_string());
        }
    }

    if cmds.is_empty() {
        return Err(AssistError::CommandFailed(
            "LLM returned an empty command list".into(),
        ));
    }

    let mut safe_cmds = Vec::new();
    for cmd in cmds {
        if validate(&cmd).is_ok() {
            safe_cmds.push(cmd);
        }
    }

    if safe_cmds.is_empty() {
        return Err(AssistError::CommandFailed(
            "LLM produced no safe commands (blocked or unsupported)".into(),
        ));
    }

    let adjusted = adjust_commands_for_intent(safe_cmds, prompt);

    let remapped: Vec<String> = adjusted
        .into_iter()
        .map(|cmd| rewrite_install_with_resolution(cmd, config))
        .collect();

    // If this was a launch intent and we only have installs, add a launch step
    if is_launch_intent(prompt) && !remapped.iter().any(|c| c.starts_with("launch ")) {
        if let Some(app) = extract_app_name_from_install(&remapped) {
            let mut with_launch = remapped.clone();
            with_launch.push(format!("launch {}", app));
            return Ok(with_launch);
        }
    }

    Ok(remapped)
}

fn adjust_commands_for_intent(cmds: Vec<String>, prompt: &str) -> Vec<String> {
    let prompt_lower = prompt.to_lowercase();
    let desired_pkg = if prompt_lower.contains("word") || prompt_lower.contains("office") {
        Some("libreoffice-fresh")
    } else {
        None
    };
    let is_launch_intent = is_launch_intent(&prompt_lower);

    let mut out = Vec::new();
    for cmd in &cmds {
        // Drop suggestions that install helper tools we don't want
        if cmd.contains(" yay") || cmd.starts_with("yay ") || cmd == "yay" {
            continue;
        }

        if let Some(pkg) = desired_pkg {
            if let Some(rewritten) = rewrite_install_pkg(cmd, pkg) {
                out.push(rewritten);
                continue;
            }
        }

        if is_launch_intent && needs_launch_wrapper(cmd) {
            out.push(format!("launch {}", cmd));
            continue;
        }

        out.push(cmd.clone());
    }

    if out.is_empty() {
        return cmds;
    }

    out
}

fn rewrite_install_pkg(cmd: &str, new_pkg: &str) -> Option<String> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let (tool, rest) = (parts[0], &parts[1..]);
    if tool != "pacman" && tool != "paru" && !(tool == "sudo" && rest.first() == Some(&"pacman")) {
        return None;
    }

    let mut installer = tool;
    let mut args = rest;
    if tool == "sudo" && rest.first() == Some(&"pacman") {
        installer = "sudo pacman";
        args = &rest[1..];
    }

    if args.is_empty() || !args[0].starts_with("-S") {
        return None;
    }

    let mut new_args = args.to_vec();
    if let Some(last) = new_args.last_mut() {
        *last = new_pkg;
    }
    let rebuilt = format!("{} {}", installer, new_args.join(" "));
    Some(rebuilt)
}

fn needs_launch_wrapper(cmd: &str) -> bool {
    let allowed = [
        "sudo", "pacman", "paru", "systemctl", "nmcli", "pactl", "bluetoothctl", "journalctl",
        "timedatectl", "echo", "launch",
    ];
    let mut parts = cmd.split_whitespace();
    let first = parts.next().unwrap_or("");
    if allowed.contains(&first) {
        return false;
    }
    // If it's a single token (likely app name), wrap it
    !cmd.contains(' ')
}

fn extract_app_name_from_install(cmds: &[String]) -> Option<String> {
    for cmd in cmds {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.len() >= 2 && parts[0] == "launch" {
            return Some(parts[1].to_string());
        }
        if parts.len() >= 3 && parts[0] == "sudo" && parts[1] == "pacman" && parts[2].starts_with("-S") {
            if let Some(pkg) = parts.last() {
                return Some((*pkg).to_string());
            }
        }
        if parts.len() >= 2 && parts[0] == "pacman" && parts[1].starts_with("-S") {
            if let Some(pkg) = parts.last() {
                return Some((*pkg).to_string());
            }
        }
        if parts.len() >= 2 && parts[0] == "paru" && parts[1].starts_with("-S") {
            if let Some(pkg) = parts.last() {
                return Some((*pkg).to_string());
            }
        }
    }
    None
}

fn is_launch_intent(prompt: &str) -> bool {
    let prompt_lower = prompt.to_lowercase();
    ["open", "launch", "start"]
        .iter()
        .any(|k| prompt_lower.starts_with(k))
}

fn rewrite_install_with_resolution(cmd: String, config: &ExecConfig) -> String {
    let trimmed = cmd.trim();
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.len() >= 3 && parts[0] == "sudo" && parts[1] == "pacman" && parts[2].starts_with("-S") {
        if let Some(pkg) = parts.last() {
            if let Some(cmd) = resolve_installer(parts[2..].to_vec(), pkg, config) {
                return cmd;
            }
        }
    }
    if parts.len() >= 2 && parts[0] == "pacman" && parts[1].starts_with("-S") {
        if let Some(pkg) = parts.last() {
            if let Some(cmd) = resolve_installer(parts[1..].to_vec(), pkg, config) {
                return cmd;
            }
        }
    }
    cmd
}

fn resolve_installer(flags_and_pkg: Vec<&str>, pkg: &str, config: &ExecConfig) -> Option<String> {
    let mut rest = flags_and_pkg;
    rest.pop(); // drop pkg
    let flags = rest.join(" ");

    let resolution = resolve_package(pkg, config);
    match resolution {
        PackageOrigin::Repo => {
            let installer = if config.no_sudo { "pacman" } else { "sudo pacman" };
            Some(format!("{installer} {} {}", flags, pkg))
        }
        PackageOrigin::Aur => Some(format!("paru {} {}", flags, pkg)),
        PackageOrigin::Unknown => {
            if is_probably_aur(pkg) {
                Some(format!("paru {} {}", flags, pkg))
            } else {
                Some(format!(
                    "{} {} {}",
                    if config.no_sudo { "pacman" } else { "sudo pacman" },
                    flags,
                    pkg
                ))
            }
        }
        PackageOrigin::Offline => None,
    }
}

fn build_install_command(pkg: &str, flags: &str, config: &ExecConfig) -> Option<String> {
    let resolution = resolve_package(pkg, config);
    match resolution {
        PackageOrigin::Repo => {
            let installer = if config.no_sudo { "pacman" } else { "sudo pacman" };
            Some(format!("{installer} {flags} {pkg}"))
        }
        PackageOrigin::Aur => Some(format!("paru {flags} {pkg}")),
        PackageOrigin::Unknown => {
            if is_probably_aur(pkg) {
                Some(format!("paru {flags} {pkg}"))
            } else {
                Some(format!(
                    "{} {flags} {}",
                    if config.no_sudo { "pacman" } else { "sudo pacman" },
                    pkg
                ))
            }
        }
        PackageOrigin::Offline => None,
    }
}

fn is_probably_aur(pkg: &str) -> bool {
    let aur_suffixes = ["-bin", "-git", "-svn", "-hg"];
    if aur_suffixes.iter().any(|s| pkg.ends_with(s)) {
        return true;
    }

    let common_aur = [
        "google-chrome",
        "brave-bin",
        "microsoft-edge-stable-bin",
        "visual-studio-code-bin",
        "wps-office",
        "slack-desktop",
        "zoom",
        "spotify",
    ];

    common_aur.contains(&pkg)
}

enum PackageOrigin {
    Repo,
    Aur,
    Unknown,
    Offline,
}

fn resolve_package(pkg: &str, config: &ExecConfig) -> PackageOrigin {
    if config.offline {
        return PackageOrigin::Offline;
    }

    if check_arch_repo(pkg) {
        return PackageOrigin::Repo;
    }

    if check_aur(pkg) {
        return PackageOrigin::Aur;
    }

    PackageOrigin::Unknown
}

fn check_arch_repo(pkg: &str) -> bool {
    let client = HttpClient::new();
    let url = format!(
        "https://archlinux.org/packages/search/json/?q={}",
        urlencoding::encode(pkg)
    );
    if let Ok(resp) = client.get(url).send() {
        if let Ok(json) = resp.json::<ArchSearch>() {
            return !json.results.is_empty();
        }
    }
    false
}

fn check_aur(pkg: &str) -> bool {
    let client = HttpClient::new();
    let url = format!(
        "https://aur.archlinux.org/rpc/?v=5&type=info&arg={}",
        urlencoding::encode(pkg)
    );
    if let Ok(resp) = client.get(url).send() {
        if let Ok(json) = resp.json::<AurInfo>() {
            return json.resultcount.unwrap_or(0) > 0;
        }
    }
    false
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: Vec<ChatContent>,
}

#[derive(Serialize)]
struct ChatContent {
    #[serde(rename = "type")]
    kind: String,
    text: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: LlmMessage,
}

#[derive(Deserialize)]
struct LlmMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct ArchSearch {
    results: Vec<ArchResult>,
}

#[derive(Deserialize)]
struct ArchResult {
    #[allow(dead_code)]
    pkgname: String,
}

#[derive(Deserialize)]
struct AurInfo {
    resultcount: Option<u32>,
}
