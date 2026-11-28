//! Aliases in other terminals. With Settings › Aliases › Use in other
//! terminals on, Dispatch writes the aliases to files of its own in
//! ~/.config/dispatch (one for zsh and bash, one for PowerShell) and adds a
//! marked block that loads them to each startup file, twice: at the top, so
//! the rest of the file can use them, and at the end, so they win over an
//! alias of the same name defined in between (oh-my-zsh's, say). Turning it
//! off takes out only those blocks. Nothing else in a startup file is touched.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::aliases::{self, Alias};
use crate::settings::SettingsState;

const START: &str = "# >>> Dispatch aliases >>>";
const END: &str = "# <<< Dispatch aliases <<<";
const HEADER: &str =
    "# Written by Dispatch from Settings › Aliases. Changes made here are replaced.\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Posix,
    PowerShell,
}

const KINDS: [Kind; 2] = [Kind::Posix, Kind::PowerShell];

impl Kind {
    fn file_name(self) -> &'static str {
        match self {
            Kind::Posix => "aliases.sh",
            Kind::PowerShell => "aliases.ps1",
        }
    }

    /// The line that loads the alias file. The home folder is left to the
    /// shell, so a startup file synced to another machine still works there,
    /// and the check skips it where the file isn't (Dispatch uninstalled).
    fn line(self) -> &'static str {
        match self {
            Kind::Posix => {
                r#"[ -f "$HOME/.config/dispatch/aliases.sh" ] && . "$HOME/.config/dispatch/aliases.sh""#
            }
            Kind::PowerShell => {
                r#"if (Test-Path "$HOME/.config/dispatch/aliases.ps1") { . "$HOME/.config/dispatch/aliases.ps1" }"#
            }
        }
    }

    fn contents(self, list: &[Alias]) -> String {
        let body = match self {
            Kind::Posix => aliases::posix(list),
            Kind::PowerShell => aliases::powershell(list),
        };
        format!("{HEADER}{body}")
    }

    /// Whether `content` loads the alias file some other way, such as a line
    /// the user added by hand.
    fn mentioned_in(self, content: &str) -> bool {
        let name = self.file_name();
        content.contains(&format!("dispatch/{name}"))
            || content.contains(&format!("dispatch\\{name}"))
    }
}

/// A startup file the block goes in.
struct Candidate {
    shell: String,
    path: PathBuf,
    kind: Kind,
}

/// What the Settings page shows for each startup file.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupFile {
    pub shell: String,
    /// With the home folder as ~.
    pub path: String,
    /// The file actually written when `path` is a symlink, as dotfile
    /// managers make them.
    pub writes_to: Option<String>,
    /// "added" (Dispatch's block is there), "yours" (a line of the user's
    /// own loads the file), or "off".
    pub state: &'static str,
    /// The line, for pasting by hand when Dispatch can't write the file.
    pub line: String,
    pub error: Option<String>,
}

fn home() -> Option<PathBuf> {
    let var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(var)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn alias_dir(home: &Path) -> PathBuf {
    home.join(".config").join("dispatch")
}

fn display(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".into(),
        Ok(rest) => format!("~/{}", rest.to_string_lossy().replace('\\', "/")),
        Err(_) => path.to_string_lossy().into_owned(),
    }
}

fn newline_of(content: &str) -> &'static str {
    if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

fn block_count(content: &str) -> usize {
    content.lines().filter(|line| line.trim() == START).count()
}

fn has_block(content: &str) -> bool {
    block_count(content) > 0
}

fn block(kind: Kind, nl: &str) -> String {
    [START, kind.line(), END]
        .iter()
        .map(|line| format!("{line}{nl}"))
        .collect()
}

/// `content` with the block at the top, then a blank line, and again at the
/// end after a blank line. An empty file gets it once.
fn add_block(content: &str, kind: Kind) -> String {
    let nl = newline_of(content);
    let block = block(kind, nl);
    if content.trim().is_empty() {
        return block;
    }
    let mut out = format!("{block}{nl}{content}");
    if !out.ends_with('\n') {
        out.push_str(nl);
    }
    out.push_str(nl);
    out.push_str(&block);
    out
}

/// Whether the blocks need putting in again: a file from before the block
/// went at the top too has it only at the end.
fn needs_blocks(content: &str) -> Result<bool, String> {
    Ok(block_count(content) < 2 && !remove_block(content)?.trim().is_empty())
}

/// `content` without the blocks, and without the blank lines put beside them.
/// Errors when the block has lost its end line, rather than guess how much
/// of the file is Dispatch's.
fn remove_block(content: &str) -> Result<String, String> {
    let lines: Vec<&str> = content.split_inclusive('\n').collect();
    let mut kept: Vec<&str> = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim() != START {
            kept.push(lines[i]);
            i += 1;
            continue;
        }
        let Some(end) = (i..lines.len()).find(|&j| lines[j].trim() == END) else {
            return Err(format!(
                "The Dispatch block has no “{END}” line, so remove it by hand."
            ));
        };
        if kept.last().is_some_and(|line| line.trim().is_empty()) {
            kept.pop();
        }
        i = end + 1;
        /* The block at the top has its blank line after it. */
        if kept.is_empty() && lines.get(i).is_some_and(|line| line.trim().is_empty()) {
            i += 1;
        }
    }
    Ok(kept.concat())
}

fn state_of(content: &str, kind: Kind) -> &'static str {
    if has_block(content) {
        "added"
    } else if kind.mentioned_in(content) {
        "yours"
    } else {
        "off"
    }
}

/// The file behind `path` when it's a symlink, so the link stays a link.
fn real_path(path: &Path) -> Result<PathBuf, String> {
    if path.is_symlink() {
        fs::canonicalize(path).map_err(|err| format!("Could not follow the link: {err}"))
    } else {
        Ok(path.to_path_buf())
    }
}

/// Writes `contents` to a file beside `path` and moves it into place, so a
/// crash can't leave half a file. With `backup`, the file as it was first is
/// kept once, as `<name>.dispatch-backup`.
fn write_atomic(path: &Path, contents: &str, backup: bool) -> Result<(), String> {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or("Not a file.")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("Could not create its folder: {err}"))?;
    }
    let existing = fs::metadata(path).ok();
    if backup && existing.is_some() {
        let copy = path.with_file_name(format!("{name}.dispatch-backup"));
        if !copy.exists() {
            fs::copy(path, &copy).map_err(|err| format!("Could not back it up: {err}"))?;
        }
    }
    let temp = path.with_file_name(format!(".{name}.dispatch-new"));
    let written = fs::write(&temp, contents)
        .and_then(|()| match &existing {
            Some(meta) => fs::set_permissions(&temp, meta.permissions()),
            None => Ok(()),
        })
        .and_then(|()| fs::rename(&temp, path));
    written.map_err(|err| {
        let _ = fs::remove_file(&temp);
        format!("Could not write it: {err}")
    })
}

/// The first of `names` on PATH.
fn find_on_path(names: &[&str]) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|dir| {
        names
            .iter()
            .map(|name| dir.join(name))
            .find(|candidate| candidate.is_file())
    })
}

/// The profile every PowerShell host loads, asked of PowerShell itself, since
/// it moves with a OneDrive-redirected Documents folder.
fn powershell_profile(exe: &Path) -> Option<PathBuf> {
    let mut command = std::process::Command::new(exe);
    command.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "$PROFILE.CurrentUserAllHosts",
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let output = command
        .output()
        .ok()
        .filter(|output| output.status.success())?;
    let path = String::from_utf8_lossy(&output.stdout)
        .trim()
        .trim_start_matches('\u{feff}')
        .to_string();
    (!path.is_empty()).then(|| PathBuf::from(path))
}

/// Whether a bash login file already loads ~/.bashrc, which then covers it.
fn loads_bashrc(content: &str) -> bool {
    content
        .lines()
        .map(str::trim)
        .any(|line| !line.starts_with('#') && line.contains(".bashrc"))
}

/// zsh and bash startup files for the shells the user has: a file that
/// already exists, or the one their login shell would read.
fn posix_candidates(
    home: &Path,
    login: &str,
    zdotdir: Option<PathBuf>,
    macos: bool,
) -> Vec<Candidate> {
    let mut found = Vec::new();
    let zshrc = zdotdir.unwrap_or_else(|| home.to_path_buf()).join(".zshrc");
    if zshrc.exists() || login == "zsh" {
        found.push(Candidate {
            shell: "zsh".into(),
            path: zshrc,
            kind: Kind::Posix,
        });
    }
    let bashrc = home.join(".bashrc");
    let profile = home.join(".bash_profile");
    let profile_content = fs::read_to_string(&profile).ok();
    let profile_loads_rc = profile_content.as_deref().is_some_and(loads_bashrc);
    if bashrc.exists() || (login == "bash" && (!macos || profile_loads_rc)) {
        found.push(Candidate {
            shell: "bash".into(),
            path: bashrc,
            kind: Kind::Posix,
        });
    }
    /* Terminal on macOS starts bash as a login shell, which reads only .bash_profile. */
    let profile_needed = match &profile_content {
        Some(_) => !profile_loads_rc,
        None => macos && login == "bash",
    };
    if profile_needed {
        found.push(Candidate {
            shell: "bash".into(),
            path: profile,
            kind: Kind::Posix,
        });
    }
    found
}

fn candidates() -> Vec<Candidate> {
    let Some(home) = home() else {
        return Vec::new();
    };
    let mut found = Vec::new();
    if cfg!(unix) {
        let login = crate::pty::login_shell();
        let login = Path::new(&login)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let zdotdir = std::env::var_os("ZDOTDIR")
            .filter(|dir| !dir.is_empty())
            .map(PathBuf::from);
        found.extend(posix_candidates(
            &home,
            &login,
            zdotdir,
            cfg!(target_os = "macos"),
        ));
    }
    let shells: &[(&str, &[&str])] = if cfg!(windows) {
        &[
            ("PowerShell", &["pwsh.exe"]),
            ("Windows PowerShell", &["powershell.exe"]),
        ]
    } else {
        &[("PowerShell", &["pwsh"])]
    };
    for (shell, names) in shells {
        if let Some(profile) = find_on_path(names).and_then(|exe| powershell_profile(&exe)) {
            found.push(Candidate {
                shell: (*shell).into(),
                path: profile,
                kind: Kind::PowerShell,
            });
        }
    }
    found
}

fn status(candidate: &Candidate, home: &Path, error: Option<String>) -> StartupFile {
    let content = fs::read_to_string(&candidate.path).unwrap_or_default();
    let writes_to = candidate
        .path
        .is_symlink()
        .then(|| real_path(&candidate.path).ok())
        .flatten()
        .map(|real| display(&real, home));
    StartupFile {
        shell: candidate.shell.clone(),
        path: display(&candidate.path, home),
        writes_to,
        state: state_of(&content, candidate.kind),
        line: candidate.kind.line().into(),
        error,
    }
}

/// Adds (`enabled`) or takes out the block in one startup file.
fn change(candidate: &Candidate, enabled: bool) -> Result<(), String> {
    let path = real_path(&candidate.path)?;
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => return Err(format!("Could not read it: {err}")),
    };
    let next = match (enabled, state_of(&content, candidate.kind)) {
        (true, "off") => add_block(&content, candidate.kind),
        (true, "added") if needs_blocks(&content)? => {
            add_block(&remove_block(&content)?, candidate.kind)
        }
        (false, "added") => remove_block(&content)?,
        _ => return Ok(()),
    };
    write_atomic(&path, &next, true)
}

fn current_aliases(app: &AppHandle) -> Vec<Alias> {
    app.state::<SettingsState>()
        .0
        .lock()
        .map(|settings| aliases::flatten(&settings.alias_groups))
        .unwrap_or_default()
}

/// The startup files, and whether each one loads Dispatch's aliases.
#[tauri::command]
pub async fn alias_startup_files() -> Result<Vec<StartupFile>, String> {
    let home = home().ok_or("Could not find the home folder.")?;
    Ok(candidates()
        .iter()
        .map(|c| status(c, &home, None))
        .collect())
}

/// Turns Use in other terminals on or off, and says how each file came out.
#[tauri::command]
pub async fn set_aliases_everywhere(
    app: AppHandle,
    enabled: bool,
) -> Result<Vec<StartupFile>, String> {
    let home = home().ok_or("Could not find the home folder.")?;
    let dir = alias_dir(&home);
    let found = candidates();
    if enabled {
        let list = current_aliases(&app);
        for kind in KINDS
            .into_iter()
            .filter(|kind| found.iter().any(|c| c.kind == *kind))
        {
            write_atomic(&dir.join(kind.file_name()), &kind.contents(&list), false)
                .map_err(|err| format!("~/.config/dispatch/{}: {err}", kind.file_name()))?;
        }
    }
    let files: Vec<StartupFile> = found
        .iter()
        .map(|candidate| status(candidate, &home, change(candidate, enabled).err()))
        .collect();
    if !enabled {
        /* A file still loaded by a line of the user's own stays, and current. */
        for kind in KINDS {
            let still_used = found
                .iter()
                .zip(&files)
                .any(|(candidate, file)| candidate.kind == kind && file.state != "off");
            if !still_used {
                let _ = fs::remove_file(dir.join(kind.file_name()));
            }
        }
    }
    Ok(files)
}

/// Keeps the alias files current after Settings is saved. Only files that are
/// there are rewritten, so with the switch off this does nothing.
pub fn refresh(list: &[Alias]) {
    let Some(home) = home() else {
        return;
    };
    let dir = alias_dir(&home);
    for kind in KINDS {
        let path = dir.join(kind.file_name());
        if path.exists() {
            if let Err(err) = write_atomic(&path, &kind.contents(list), false) {
                eprintln!("Dispatch: could not update {}: {err}", path.display());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_home() -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("dispatch-alias-files-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn adds_the_block_at_the_top_and_the_end() {
        let block = format!("{START}\n{}\n{END}\n", Kind::Posix.line());
        assert_eq!(
            add_block("export A=1", Kind::Posix),
            format!("{block}\nexport A=1\n\n{block}")
        );
        assert_eq!(add_block("", Kind::Posix), block);
    }

    #[test]
    fn keeps_windows_line_endings() {
        let out = add_block("$x = 1\r\n", Kind::PowerShell);
        let block = format!("{START}\r\n{}\r\n{END}\r\n", Kind::PowerShell.line());
        assert_eq!(out, format!("{block}\r\n$x = 1\r\n\r\n{block}"));
        assert_eq!(remove_block(&out).unwrap(), "$x = 1\r\n");
    }

    #[test]
    fn removing_gives_back_the_file_as_it_was() {
        for original in [
            "",
            "export A=1\n",
            "export A=1",
            "a\n\nb\n",
            "\nleading blank\n",
        ] {
            let added = add_block(original, Kind::Posix);
            let expected = if original.is_empty() || original.ends_with('\n') {
                original.to_string()
            } else {
                format!("{original}\n")
            };
            assert_eq!(remove_block(&added).unwrap(), expected, "{original:?}");
        }
    }

    #[test]
    fn removes_only_the_block_in_the_middle() {
        let content = format!("a\n\n{START}\nwhatever\n{END}\nb\n");
        assert_eq!(remove_block(&content).unwrap(), "a\nb\n");
    }

    #[test]
    fn puts_the_top_block_in_a_file_from_before() {
        let old = format!("c\n\n{START}\n{}\n{END}\n", Kind::Posix.line());
        assert!(needs_blocks(&old).unwrap());
        let fixed = add_block(&remove_block(&old).unwrap(), Kind::Posix);
        assert_eq!(block_count(&fixed), 2);
        assert!(!needs_blocks(&fixed).unwrap());
        assert!(!needs_blocks(&add_block("", Kind::Posix)).unwrap());
    }

    #[test]
    fn refuses_a_block_without_its_end() {
        assert!(remove_block(&format!("a\n{START}\nline\nb\n")).is_err());
    }

    #[test]
    fn tells_its_block_from_a_line_of_your_own() {
        assert_eq!(
            state_of(&add_block("x\n", Kind::Posix), Kind::Posix),
            "added"
        );
        assert_eq!(
            state_of("source ~/.config/dispatch/aliases.sh\n", Kind::Posix),
            "yours"
        );
        assert_eq!(state_of("source ~/.aliases\n", Kind::Posix), "off");
    }

    #[test]
    fn the_alias_file_loads_in_zsh_and_bash() {
        let list = [Alias {
            name: "hi".into(),
            command: "echo 'it works'".into(),
        }];
        let contents = Kind::Posix.contents(&list);
        assert!(contents.starts_with("# Written by Dispatch"));
        assert!(contents.ends_with("alias -- hi='echo '\\''it works'\\'''\n"));
    }

    #[test]
    fn picks_startup_files_for_the_shells_in_use() {
        let home = temp_home();
        let paths = |found: Vec<Candidate>| -> Vec<String> {
            found.iter().map(|c| display(&c.path, &home)).collect()
        };

        /* A zsh user with nothing yet. */
        assert_eq!(
            paths(posix_candidates(&home, "zsh", None, true)),
            ["~/.zshrc"]
        );

        /* bash on macOS needs .bash_profile, which Terminal reads. */
        assert_eq!(
            paths(posix_candidates(&home, "bash", None, true)),
            ["~/.bash_profile"]
        );
        assert_eq!(
            paths(posix_candidates(&home, "bash", None, false)),
            ["~/.bashrc"]
        );

        /* A .bash_profile that loads .bashrc is covered by it. */
        fs::write(
            home.join(".bash_profile"),
            "[ -f ~/.bashrc ] && . ~/.bashrc\n",
        )
        .unwrap();
        assert_eq!(
            paths(posix_candidates(&home, "bash", None, true)),
            ["~/.bashrc"]
        );

        /* Files that exist count even when it's not the login shell. */
        fs::write(home.join(".zshrc"), "").unwrap();
        fs::write(home.join(".bash_profile"), "export A=1\n").unwrap();
        assert_eq!(
            paths(posix_candidates(&home, "zsh", None, true)),
            ["~/.zshrc", "~/.bash_profile"]
        );

        /* ZDOTDIR moves .zshrc. */
        let zdot = home.join("zsh");
        assert_eq!(
            paths(posix_candidates(&home, "zsh", Some(zdot), true))[0],
            "~/zsh/.zshrc"
        );
        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn writes_and_removes_through_the_file() {
        let home = temp_home();
        let rc = home.join(".zshrc");
        fs::write(&rc, "alias ll='ls -l'\n").unwrap();
        let candidate = Candidate {
            shell: "zsh".into(),
            path: rc.clone(),
            kind: Kind::Posix,
        };

        change(&candidate, true).unwrap();
        change(&candidate, true).unwrap();
        let added = fs::read_to_string(&rc).unwrap();
        assert_eq!(added.matches(START).count(), 2);
        assert_eq!(
            fs::read_to_string(home.join(".zshrc.dispatch-backup")).unwrap(),
            "alias ll='ls -l'\n"
        );

        change(&candidate, false).unwrap();
        assert_eq!(fs::read_to_string(&rc).unwrap(), "alias ll='ls -l'\n");
        assert!(!home.join("..zshrc.dispatch-new").exists());
        fs::remove_dir_all(&home).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn writes_through_a_symlink() {
        let home = temp_home();
        let dotfiles = home.join("dotfiles");
        fs::create_dir_all(&dotfiles).unwrap();
        fs::write(dotfiles.join("zshrc"), "export A=1\n").unwrap();
        let link = home.join(".zshrc");
        std::os::unix::fs::symlink(dotfiles.join("zshrc"), &link).unwrap();
        let candidate = Candidate {
            shell: "zsh".into(),
            path: link.clone(),
            kind: Kind::Posix,
        };

        change(&candidate, true).unwrap();
        assert!(link.is_symlink());
        assert!(has_block(
            &fs::read_to_string(dotfiles.join("zshrc")).unwrap()
        ));
        let real = fs::canonicalize(dotfiles.join("zshrc")).unwrap();
        assert!(status(&candidate, &home, None).writes_to.is_some());
        assert!(real.exists());
        fs::remove_dir_all(&home).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn the_line_loads_the_aliases_in_zsh_and_bash() {
        let home = temp_home();
        let list = [Alias {
            name: "hi".into(),
            command: "echo dispatch-ok".into(),
        }];
        write_atomic(
            &alias_dir(&home).join("aliases.sh"),
            &Kind::Posix.contents(&list),
            false,
        )
        .unwrap();
        for shell in ["zsh", "bash"] {
            if find_on_path(&[shell]).is_none() {
                continue;
            }
            /* The rest of the file can use an alias right after the top block. */
            let script = add_block("hi\nalias hi", Kind::Posix);
            let out = std::process::Command::new(shell)
                .args(
                    ["-f", "-c"]
                        .iter()
                        .filter(|a| shell == "zsh" || **a != "-f"),
                )
                .arg(&script)
                .env("HOME", &home)
                .output()
                .unwrap();
            let text = String::from_utf8_lossy(&out.stdout);
            assert!(text.contains("echo dispatch-ok"), "{shell}: {text}");
        }
        fs::remove_dir_all(&home).unwrap();
    }
}
