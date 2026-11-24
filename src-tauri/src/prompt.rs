//! The Dispatch prompt: startup files that load the user's own zsh or bash
//! setup and then replace the prompt with Node version, folder, git branch,
//! and changed files. The page draws each part as a chip (see OSC 6973 in
//! TerminalPane.vue).

use std::fs;
use std::path::{Path, PathBuf};

use portable_pty::CommandBuilder;
use tauri::{AppHandle, Manager};

const ZSH_FILES: &[(&str, &str)] = &[
    (".zshenv", include_str!("../shell/zsh/.zshenv")),
    (".zprofile", include_str!("../shell/zsh/.zprofile")),
    (".zshrc", include_str!("../shell/zsh/.zshrc")),
    ("dispatch.zsh", include_str!("../shell/zsh/dispatch.zsh")),
];
const BASH_INIT: &str = include_str!("../shell/bash/dispatch.bash");

enum Shell {
    Zsh,
    Bash,
}

fn shell_kind(shell: &str) -> Option<Shell> {
    let name = Path::new(shell).file_name()?.to_str()?;
    match name.trim_end_matches(".exe") {
        "zsh" => Some(Shell::Zsh),
        "bash" => Some(Shell::Bash),
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
/// and the shell is one it supports. Other shells start as plain login shells.
pub fn configure(app: &AppHandle, cmd: &mut CommandBuilder, shell: &str, enabled: bool) {
    let applied = enabled && apply(app, cmd, shell).is_some();
    #[cfg(unix)]
    if !applied {
        cmd.arg("-l");
    }
    /* Windows shells take no login flag. */
    #[cfg(not(unix))]
    let _ = applied;
}

fn apply(app: &AppHandle, cmd: &mut CommandBuilder, shell: &str) -> Option<()> {
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
    }
    Some(())
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
        assert!(shell_kind("pwsh.exe").is_none());
    }
}
