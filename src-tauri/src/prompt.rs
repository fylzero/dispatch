//! The Dispatch prompt: startup files that load the user's own zsh or bash
//! setup and then replace the prompt with Node version, folder, git branch,
//! and changed files. The page draws each part as a chip (see OSC 6973 in
//! TerminalPane.vue).

use std::fs;
use std::path::{Path, PathBuf};

use portable_pty::CommandBuilder;
use tauri::{AppHandle, Manager};

use crate::aliases::{self, Alias};

const ZSH_FILES: &[(&str, &str)] = &[
    (".zshenv", include_str!("../shell/zsh/.zshenv")),
    (".zprofile", include_str!("../shell/zsh/.zprofile")),
    (".zshrc", include_str!("../shell/zsh/.zshrc")),
    ("dispatch.zsh", include_str!("../shell/zsh/dispatch.zsh")),
];
const BASH_INIT: &str = include_str!("../shell/bash/dispatch.bash");
const POWERSHELL_INIT: &str = include_str!("../shell/powershell/dispatch.ps1");

/* Named after the shells, PowerShell included. */
#[allow(clippy::enum_variant_names)]
enum Shell {
    Zsh,
    Bash,
    PowerShell,
    Cmd,
}

fn shell_kind(shell: &str) -> Option<Shell> {
    /* By hand, since a Windows path's backslashes are no separator to Path elsewhere. */
    let name = shell.trim().rsplit(['/', '\\']).next()?.to_ascii_lowercase();
    match name.trim_end_matches(".exe") {
        "zsh" => Some(Shell::Zsh),
        "bash" => Some(Shell::Bash),
        "pwsh" | "powershell" => Some(Shell::PowerShell),
        "cmd" => Some(Shell::Cmd),
        _ => None,
    }
}

/// Writes the files every time a shell starts, so a cleaned-out app data
/// folder can never leave zsh with a ZDOTDIR that has no startup files.
fn write_files(dir: &Path, files: &[(&str, &str)]) -> Option<()> {
    fs::create_dir_all(dir).ok()?;
    for (name, contents) in files {
        let path = dir.join(name);
        if fs::read_to_string(&path).ok().as_deref() != Some(*contents) {
            fs::write(&path, contents).ok()?;
        }
    }
    Some(())
}

fn integration_dir(app: &AppHandle) -> Option<PathBuf> {
    Some(app.path().app_data_dir().ok()?.join("shell-integration"))
}

/// Adds the shell's arguments to `cmd`, with the Dispatch prompt when `enabled`
/// and the shell is zsh or bash. Other shells start as plain login shells,
/// except that with `suggestions` PowerShell and cmd mark their prompt so
/// Dispatch can suggest as you type (zsh and bash get that from the prompt).
/// Every shell defines `aliases` after the user's own startup files; zsh and
/// bash go through the Dispatch startup files for that even with the prompt off.
pub fn configure(
    app: &AppHandle,
    cmd: &mut CommandBuilder,
    shell: &str,
    enabled: bool,
    suggestions: bool,
    aliases: &[Alias],
) {
    match shell_kind(shell) {
        Some(Shell::PowerShell) => {
            let mut script = String::new();
            if suggestions {
                script.push_str(POWERSHELL_INIT);
            }
            script.push_str(&aliases::powershell(aliases));
            if !script.is_empty() {
                /* Encoded, so it needs no quoting and no execution policy allows a script file. */
                cmd.arg("-NoExit");
                cmd.arg("-EncodedCommand");
                cmd.arg(encode_command(&script));
                return;
            }
        }
        Some(Shell::Cmd) => {
            if suggestions {
                let own = std::env::var("PROMPT").unwrap_or_else(|_| "$P$G".into());
                cmd.env("PROMPT", cmd_prompt(&own));
            }
            let macros = aliases::doskey(aliases);
            if !macros.is_empty() {
                if let Some(root) = integration_dir(app) {
                    if write_files(&root, &[("aliases.doskey", &macros)]).is_some() {
                        cmd.arg("/K");
                        cmd.arg("doskey");
                        let mut file = std::ffi::OsString::from("/macrofile=");
                        file.push(root.join("aliases.doskey"));
                        cmd.arg(file);
                    }
                }
            }
            return;
        }
        _ => {}
    }
    let posix = aliases::posix(aliases);
    let applied = (enabled || !posix.is_empty()) && apply(app, cmd, shell, enabled, &posix).is_some();
    #[cfg(unix)]
    if !applied {
        cmd.arg("-l");
    }
    /* Windows shells take no login flag. */
    #[cfg(not(unix))]
    let _ = applied;
}

fn apply(app: &AppHandle, cmd: &mut CommandBuilder, shell: &str, prompt: bool, aliases: &str) -> Option<()> {
    let root = integration_dir(app)?;
    match shell_kind(shell)? {
        Shell::Zsh => {
            let dir = root.join("zsh");
            write_files(&dir, ZSH_FILES)?;
            cmd.arg("-l");
            /* The Dispatch .zshrc loads the user's files from here, then hands ZDOTDIR back. */
            if let Some(user_dir) = std::env::var_os("ZDOTDIR") {
                cmd.env("DISPATCH_USER_ZDOTDIR", user_dir);
            }
            cmd.env("ZDOTDIR", dir);
        }
        Shell::Bash => {
            write_files(&root, &[("dispatch.bash", BASH_INIT)])?;
            /* --init-file only applies to a non-login shell; the file loads the login files itself. */
            cmd.arg("--init-file");
            cmd.arg(root.join("dispatch.bash"));
            cmd.arg("-i");
        }
        Shell::PowerShell | Shell::Cmd => return None,
    }
    /* Read once by the startup files, which then unset them. Set either way, so values Dispatch itself was started with don't count. */
    cmd.env("DISPATCH_PROMPT", if prompt { "1" } else { "0" });
    cmd.env("DISPATCH_ALIASES", aliases);
    Some(())
}

/// cmd's prompt with Dispatch's marks around it: the folder before, and where the command starts after.
fn cmd_prompt(own: &str) -> String {
    format!("$E]6973;cwd;$P$E\\{own}$E]6973;line$E\\")
}

/// A script as PowerShell's -EncodedCommand takes it: UTF-16LE in base64.
fn encode_command(script: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |n, (i, byte)| n | (u32::from(*byte) << (16 - 8 * i)));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knows_zsh_and_bash_by_path() {
        assert!(matches!(shell_kind("/bin/zsh"), Some(Shell::Zsh)));
        assert!(matches!(
            shell_kind("/opt/homebrew/bin/bash"),
            Some(Shell::Bash)
        ));
        assert!(shell_kind("/opt/homebrew/bin/fish").is_none());
        assert!(matches!(shell_kind("pwsh.exe"), Some(Shell::PowerShell)));
        assert!(matches!(
            shell_kind("C:\\Windows\\System32\\cmd.exe"),
            Some(Shell::Cmd)
        ));
    }

    #[test]
    fn encodes_powershell_commands() {
        /* "dir" in UTF-16LE is 64 00 69 00 72 00. */
        assert_eq!(encode_command("dir"), "ZABpAHIA");
        assert_eq!(encode_command("ab"), "YQBiAA==");
    }

    #[test]
    fn marks_the_cmd_prompt() {
        assert_eq!(cmd_prompt("$P$G"), "$E]6973;cwd;$P$E\\$P$G$E]6973;line$E\\");
    }
}
