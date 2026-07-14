use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::ChildStderr;

const STDERR_SUMMARY_MAX_LINES: usize = 3;
const STDERR_SUMMARY_MAX_CHARS: usize = 240;

type StderrBufferInner = Arc<Mutex<VecDeque<String>>>;

#[derive(Clone, Debug, Default)]
pub struct StderrBuffer {
    inner: StderrBufferInner,
}

impl StderrBuffer {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn push(&self, line: &str) {
        let summary = summarize_stderr_line(line);
        if summary.is_empty() {
            return;
        }

        if let Ok(mut lines) = self.inner.lock() {
            if lines.len() == STDERR_SUMMARY_MAX_LINES {
                lines.pop_front();
            }
            lines.push_back(summary);
        }
    }

    pub fn summary(&self) -> Option<String> {
        self.inner.lock().ok().and_then(|lines| {
            (!lines.is_empty()).then(|| lines.iter().cloned().collect::<Vec<_>>().join(" | "))
        })
    }
}

pub fn spawn_stderr_collector(stderr: Option<ChildStderr>) -> StderrBuffer {
    let buffer = StderrBuffer::new();

    if let Some(stderr) = stderr {
        let buffer_clone = buffer.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                buffer_clone.push(&line);
            }
        });
    }

    buffer
}

pub fn repair_process_path() {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        if let Some(shell_path) = read_login_shell_path() {
            let current_path = std::env::var("PATH").ok();
            let merged_path = merge_path_strings(&[Some(shell_path), current_path], false);
            if !merged_path.is_empty() {
                // SAFETY: updating PATH during single-threaded app startup before background work begins.
                unsafe { std::env::set_var("PATH", merged_path) };
            }
        }
    }

    #[cfg(windows)]
    {
        let current_path = std::env::var("PATH").ok();
        let user_path = read_windows_registry_path(windows_user_environment_key());
        let system_path = read_windows_registry_path(windows_system_environment_key());
        let merged_path = merge_path_strings(&[current_path, user_path, system_path], true);
        if !merged_path.is_empty() {
            // SAFETY: updating PATH during single-threaded app startup before background work begins.
            unsafe { std::env::set_var("PATH", merged_path) };
        }
    }
}

pub fn build_stdio_environment(
    parent_env: &HashMap<String, String>,
    server_env: &HashMap<String, String>,
) -> HashMap<String, String> {
    build_stdio_environment_for_platform(parent_env, server_env, cfg!(windows))
}

pub fn find_executable_on_path(command: &str, env: &HashMap<String, String>) -> Option<String> {
    let path = Path::new(command);
    if path.is_absolute()
        || path
            .parent()
            .is_some_and(|parent| !parent.as_os_str().is_empty())
    {
        return is_executable_path(path).then(|| command.to_string());
    }

    let is_windows = cfg!(windows);
    let path_var = path_value_for_platform(env, is_windows)?;
    let separator = if is_windows { ';' } else { ':' };
    let candidate_names = executable_names_for_platform(command, env, is_windows);

    for dir in path_var.split(separator) {
        if dir.trim().is_empty() {
            continue;
        }

        for name in &candidate_names {
            let candidate = PathBuf::from(dir).join(name);
            if is_executable_path(&candidate) {
                return Some(candidate.to_string_lossy().into_owned());
            }
        }
    }

    None
}

fn summarize_stderr_line(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut chars = trimmed.chars();
    let summary: String = chars.by_ref().take(STDERR_SUMMARY_MAX_CHARS).collect();
    if chars.next().is_some() {
        format!("{summary}…")
    } else {
        summary
    }
}

fn build_stdio_environment_for_platform(
    parent_env: &HashMap<String, String>,
    server_env: &HashMap<String, String>,
    is_windows: bool,
) -> HashMap<String, String> {
    let mut env = parent_env.clone();
    env.extend(
        server_env
            .iter()
            .map(|(key, value)| (key.clone(), value.clone())),
    );

    let home = env_value_for_platform(&env, "HOME", is_windows)
        .cloned()
        .or_else(|| env_value_for_platform(&env, "USERPROFILE", is_windows).cloned())
        .unwrap_or_default();

    #[cfg(unix)]
    let default_candidates: &[&str] = &[
        "~/.local/share/mise/shims",
        "~/.local/bin",
        "~/Library/pnpm",
        "~/.cargo/bin",
        "~/.asdf/shims",
        "~/.volta/bin",
        "~/.bun/bin",
        "/opt/homebrew/bin",
        "/opt/homebrew/sbin",
        "/usr/local/bin",
        "/usr/local/sbin",
        "/opt/local/bin",
        "/usr/bin",
        "/bin",
        "/usr/sbin",
        "/sbin",
    ];

    #[cfg(windows)]
    let default_candidates: &[&str] = &[
        "~/AppData/Local/pnpm",
        "~/.cargo/bin",
        "~/AppData/Local/Programs",
        "~/AppData/Local/Microsoft/WinGet/Links",
        "~/scoop/shims",
        "~/AppData/Roaming/npm",
    ];

    let default_entries = default_candidates
        .iter()
        .map(|path| expand_home(path, &home))
        .collect::<Vec<_>>();

    let server_path = path_value_for_platform(server_env, is_windows).cloned();
    let parent_path = path_value_for_platform(parent_env, is_windows).cloned();
    #[cfg(windows)]
    let registry_user_path = read_windows_registry_path(windows_user_environment_key());
    #[cfg(not(windows))]
    let registry_user_path: Option<String> = None;
    #[cfg(windows)]
    let registry_system_path = read_windows_registry_path(windows_system_environment_key());
    #[cfg(not(windows))]
    let registry_system_path: Option<String> = None;
    let merged_path = merge_path_strings(
        &[
            server_path,
            parent_path,
            registry_user_path,
            registry_system_path,
            Some(join_paths(default_entries, is_windows)),
        ],
        is_windows,
    );

    if !merged_path.is_empty() {
        env.insert("PATH".to_string(), merged_path);
    }

    env
}

fn merge_path_strings(values: &[Option<String>], is_windows: bool) -> String {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();

    for value in values.iter().flatten() {
        for entry in split_path(value, is_windows) {
            let dedupe_key = if is_windows {
                entry.to_ascii_lowercase()
            } else {
                entry.clone()
            };
            if seen.insert(dedupe_key) {
                merged.push(entry);
            }
        }
    }

    join_paths(merged, is_windows)
}

fn join_paths(entries: Vec<String>, is_windows: bool) -> String {
    let separator = if is_windows { ";" } else { ":" };
    entries.join(separator)
}

fn expand_home(path: &str, home: &str) -> String {
    if home.is_empty() {
        return path.to_string();
    }

    if path == "~" {
        return home.to_string();
    }

    if let Some(rest) = path.strip_prefix("~/") {
        return PathBuf::from(home)
            .join(rest)
            .to_string_lossy()
            .into_owned();
    }

    path.to_string()
}

fn split_path(value: &str, is_windows: bool) -> Vec<String> {
    let separator = if is_windows { ';' } else { ':' };
    value
        .split(separator)
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| trim_matching_quotes(entry).to_string())
        .filter(|entry| !entry.is_empty())
        .collect()
}

fn trim_matching_quotes(value: &str) -> &str {
    match value.as_bytes() {
        [first, middle @ .., last]
            if !middle.is_empty()
                && ((*first == b'"' && *last == b'"') || (*first == b'\'' && *last == b'\'')) =>
        {
            std::str::from_utf8(middle).unwrap_or(value).trim()
        }
        _ => value,
    }
}

fn env_value_for_platform<'a>(
    env: &'a HashMap<String, String>,
    key: &str,
    is_windows: bool,
) -> Option<&'a String> {
    env.get(key).or_else(|| {
        is_windows
            .then(|| {
                env.iter()
                    .find(|(candidate, _)| candidate.eq_ignore_ascii_case(key))
                    .map(|(_, value)| value)
            })
            .flatten()
    })
}

fn path_value_for_platform(env: &HashMap<String, String>, is_windows: bool) -> Option<&String> {
    env_value_for_platform(env, "PATH", is_windows)
}

fn executable_names_for_platform(
    command: &str,
    env: &HashMap<String, String>,
    is_windows: bool,
) -> Vec<String> {
    if !is_windows || Path::new(command).extension().is_some() {
        return vec![command.to_string()];
    }

    let path_ext = env_value_for_platform(env, "PATHEXT", is_windows)
        .cloned()
        .unwrap_or_else(|| ".EXE;.CMD;.BAT;.COM".to_string());

    std::iter::once(command.to_string())
        .chain(split_path(&path_ext, true).into_iter().map(|extension| {
            let extension = if extension.starts_with('.') {
                extension.to_ascii_lowercase()
            } else {
                format!(".{}", extension.to_ascii_lowercase())
            };
            format!("{command}{extension}")
        }))
        .collect()
}

fn is_executable_path(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        std::fs::metadata(path).ok().is_some_and(|metadata| {
            metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
        })
    }

    #[cfg(windows)]
    {
        let metadata = match std::fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(_) => return false,
        };
        if !metadata.is_file() {
            return false;
        }

        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();

        matches!(extension.as_str(), "exe" | "cmd" | "bat" | "com")
    }
}

#[cfg(windows)]
const fn windows_user_environment_key() -> &'static str {
    "Environment"
}

#[cfg(windows)]
const fn windows_system_environment_key() -> &'static str {
    r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment"
}

#[cfg(windows)]
fn read_windows_registry_path(subkey: &str) -> Option<String> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    let hive = if subkey == windows_user_environment_key() {
        RegKey::predef(HKEY_CURRENT_USER)
    } else {
        RegKey::predef(HKEY_LOCAL_MACHINE)
    };

    let key = hive.open_subkey(subkey).ok()?;
    key.get_value::<String, _>("Path")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn read_login_shell_path() -> Option<String> {
    let mut shell_candidates = Vec::new();
    if let Ok(shell) = std::env::var("SHELL") {
        shell_candidates.push(shell);
    }
    shell_candidates.push("/bin/zsh".to_string());
    shell_candidates.push("/bin/bash".to_string());

    for shell in shell_candidates {
        let shell_path = Path::new(&shell);
        if !shell_path.exists() {
            continue;
        }

        let Some(args) = login_shell_args(&shell) else {
            continue;
        };

        let output = std::process::Command::new(&shell)
            .args(args)
            .arg("printf %s \"$PATH\"")
            .output();

        let Ok(output) = output else {
            continue;
        };
        if !output.status.success() {
            continue;
        }

        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(path);
        }
    }

    None
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn login_shell_args(shell: &str) -> Option<&'static [&'static str]> {
    let name = Path::new(shell).file_name()?.to_str()?;
    match name {
        "zsh" | "bash" => Some(&["-ilc"]),
        "fish" => Some(&["-lic"]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stderr_summary_keeps_recent_lines() {
        let buffer = StderrBuffer::new();
        buffer.push("first");
        buffer.push("second");
        buffer.push("third");
        buffer.push("fourth");

        assert_eq!(buffer.summary().as_deref(), Some("second | third | fourth"));
    }

    #[test]
    fn build_stdio_environment_merges_paths_and_defaults() {
        let parent = HashMap::from([
            ("HOME".to_string(), "/Users/test".to_string()),
            ("PATH".to_string(), "/usr/bin:/bin".to_string()),
        ]);
        let server = HashMap::from([("PATH".to_string(), "/custom/bin".to_string())]);

        let env = build_stdio_environment_for_platform(&parent, &server, false);
        let path = env.get("PATH").expect("PATH should exist");

        assert!(path.starts_with("/custom/bin:/usr/bin:/bin"));
        assert!(path.contains("/Users/test/.local/share/mise/shims"));
    }

    #[test]
    fn windows_path_lookup_accepts_mixed_case_key() {
        let env = HashMap::from([("Path".to_string(), r"C:\Tools".to_string())]);

        assert_eq!(
            path_value_for_platform(&env, true).map(String::as_str),
            Some(r"C:\Tools")
        );
    }

    #[test]
    fn windows_merge_path_is_case_insensitive() {
        let merged = merge_path_strings(
            &[
                Some(r"C:\Tools;C:\NodeJS".to_string()),
                Some(r"c:\tools;C:\Other".to_string()),
            ],
            true,
        );

        assert_eq!(merged, r"C:\Tools;C:\NodeJS;C:\Other");
    }

    #[test]
    fn split_path_trims_wrapped_quotes() {
        let entries = split_path(r#""C:\Program Files\nodejs";C:\Tools"#, true);

        assert_eq!(
            entries,
            vec![
                r"C:\Program Files\nodejs".to_string(),
                r"C:\Tools".to_string()
            ]
        );
    }
}
