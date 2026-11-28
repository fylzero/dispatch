//! Suggestions: what Tab would most likely complete for the word being typed
//! at a prompt, which the page shows in grey after the cursor. Any word is
//! matched against the files and folders where the shell is (`sr` → `src/`,
//! `cat REA` → `README.md`); the first word of a command with no such match
//! is matched against commands on PATH (`gi` → `git`). It works from the command
//! line as the page reads it off the screen, so it is the same for every
//! shell; only the quoting of what it adds differs.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};


use crate::pty::{self, TerminalState};

/// How a shell reads a command line: its escape character and quotes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// zsh, bash, fish and other Unix shells (Git Bash on Windows too).
    Posix,
    PowerShell,
    Cmd,
}

impl Style {
    pub fn for_shell(shell: &str) -> Style {
        /* By hand, since a Windows path's backslashes are no separator to Path on macOS. */
        let name = shell
            .trim()
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        match name.trim_end_matches(".exe") {
            "pwsh" | "powershell" => Style::PowerShell,
            "cmd" => Style::Cmd,
            _ => Style::Posix,
        }
    }

    fn escape(self) -> Option<char> {
        match self {
            Style::Posix => Some('\\'),
            Style::PowerShell => Some('`'),
            Style::Cmd => None,
        }
    }

    fn is_quote(self, c: char) -> bool {
        c == '"' || (c == '\'' && self != Style::Cmd)
    }

    /// Characters that end a command, so the next word is a command again.
    fn is_separator(self, c: char) -> bool {
        match self {
            Style::Posix => matches!(c, ';' | '|' | '&' | '(' | ')'),
            Style::PowerShell => matches!(c, ';' | '|' | '&' | '(' | ')' | '{' | '}'),
            Style::Cmd => matches!(c, '&' | '|' | '(' | ')'),
        }
    }

    /// Characters that make a word more than plain text (variables, globs,
    /// command substitution); such words are left to Tab.
    fn is_special(self, c: char) -> bool {
        match self {
            Style::Posix => matches!(c, '$' | '`' | '*' | '?' | '[' | ']' | '{' | '}'),
            Style::PowerShell => matches!(c, '$' | '*' | '?' | '[' | ']' | '@'),
            Style::Cmd => matches!(c, '%' | '*' | '?' | '!' | '^'),
        }
    }

    /// Command names and file names on Windows don't care about case.
    fn commands_ignore_case(self) -> bool {
        self != Style::Posix || cfg!(windows)
    }

    /// What goes after a folder's name: `/` everywhere but PowerShell and cmd on Windows.
    fn folder_separator(self) -> char {
        if cfg!(windows) && self != Style::Posix {
            '\\'
        } else {
            '/'
        }
    }
}

/// Words after which the next one is a command again.
const LEAD_WORDS: &[&str] = &[
    "sudo", "command", "builtin", "exec", "nohup", "time", "nice", "env", "xargs", "then", "do",
    "else", "if", "while", "until", "!", "watch",
];

/// Commands whose argument is a folder.
const FOLDER_COMMANDS: &[&str] = &[
    "cd", "pushd", "rmdir", "z", "chdir", "set-location", "sl", "push-location",
];

/// Commands the shell runs itself, so they are on no PATH.
const POSIX_BUILTINS: &[&str] = &[
    "alias", "bg", "bind", "builtin", "cd", "command", "declare", "dirs", "disown", "echo",
    "eval", "exec", "exit", "export", "fc", "fg", "hash", "history", "jobs", "kill", "local",
    "popd", "printf", "pushd", "pwd", "read", "readonly", "return", "set", "shift", "source",
    "test", "times", "trap", "type", "typeset", "ulimit", "umask", "unalias", "unset", "wait",
    "which",
];

const POWERSHELL_COMMANDS: &[&str] = &[
    "Clear-Host", "Copy-Item", "ForEach-Object", "Get-ChildItem", "Get-Command", "Get-Content",
    "Get-Help", "Get-History", "Get-Item", "Get-Location", "Get-Member", "Get-Process",
    "Get-Service", "Import-Module", "Invoke-Expression", "Invoke-RestMethod",
    "Invoke-WebRequest", "Move-Item", "New-Item", "Out-File", "Pop-Location", "Push-Location",
    "Remove-Item", "Rename-Item", "Select-Object", "Select-String", "Set-Content",
    "Set-Location", "Sort-Object", "Start-Process", "Stop-Process", "Test-Path", "Where-Object",
    "Write-Host", "Write-Output", "cd", "cls", "copy", "del", "dir", "echo", "exit", "ls",
    "mkdir", "move", "pwd", "ren", "rm", "rmdir", "type",
];

const CMD_BUILTINS: &[&str] = &[
    "assoc", "call", "cd", "chdir", "cls", "copy", "del", "dir", "echo", "endlocal", "erase",
    "exit", "for", "ftype", "goto", "if", "md", "mkdir", "mklink", "move", "path", "popd",
    "pushd", "rd", "ren", "rename", "rmdir", "set", "setlocal", "start", "title", "type", "ver",
];

/// The word the cursor is at the end of, as the shell will read it.
#[derive(Debug, PartialEq, Eq)]
struct Word {
    /// The word without its quotes and escapes.
    text: String,
    /// The quote still open at the cursor, if any.
    quote: Option<char>,
    /// Where a command name goes.
    at_command: bool,
    /// The command this word is an argument of.
    command: Option<String>,
}

/// Reads the last word of `line`, or None when the cursor is not on a word
/// that can be completed plainly (after a space, in a variable, on a glob).
fn last_word(line: &str, style: Style) -> Option<Word> {
    let mut text = String::new();
    let mut started = false;
    let mut special = false;
    let mut quote: Option<char> = None;
    let mut next_is_command = true;
    let mut at_command = true;
    let mut command: Option<String> = None;
    let mut chars = line.chars();

    while let Some(c) = chars.next() {
        if !started {
            at_command = next_is_command;
        }
        if let Some(open) = quote {
            if c == open {
                quote = None;
            } else if open == '"' && style.escape() == Some(c) {
                text.push(chars.next()?);
            } else {
                if open == '"' && (c == '$' || (style == Style::Posix && c == '`')) {
                    special = true;
                }
                text.push(c);
            }
            continue;
        }
        if style.escape() == Some(c) {
            text.push(chars.next()?);
            started = true;
            continue;
        }
        let ends_word = c.is_whitespace() || style.is_separator(c) || c == '<' || c == '>';
        if ends_word {
            if started {
                if at_command {
                    let lead = LEAD_WORDS.contains(&text.as_str());
                    if !lead {
                        command = Some(text.clone());
                    }
                    next_is_command = lead;
                } else {
                    next_is_command = false;
                }
            }
            if style.is_separator(c) {
                next_is_command = true;
                command = None;
            } else if c == '<' || c == '>' {
                next_is_command = false;
            }
            text.clear();
            started = false;
            special = false;
            continue;
        }
        started = true;
        if style.is_quote(c) {
            quote = Some(c);
        } else if c == '=' && !at_command {
            /* --file=src/ma completes the part after the =. */
            text.clear();
        } else {
            if style.is_special(c) {
                special = true;
            }
            text.push(c);
        }
    }

    if !started || special || text.is_empty() {
        return None;
    }
    Some(Word {
        text,
        quote,
        at_command,
        command,
    })
}

/// Whether `name` starts with `prefix` (ignoring case when asked) and is longer.
fn extends(name: &str, prefix: &str, ignore_case: bool) -> bool {
    if name.chars().count() <= prefix.chars().count() {
        return false;
    }
    if ignore_case {
        name.to_lowercase().starts_with(&prefix.to_lowercase())
    } else {
        name.starts_with(prefix)
    }
}

/// What is left of `name` after the first `count` characters.
fn after_chars(name: &str, count: usize) -> String {
    name.chars().skip(count).collect()
}

fn is_separator_char(c: char, style: Style) -> bool {
    c == '/' || (c == '\\' && style != Style::Posix && cfg!(windows))
}

/// The rest of the likeliest file or folder for `typed`: names in that
/// folder that start with it, in alphabetical order, folders ending in a separator.
fn complete_path(
    typed: &str,
    style: Style,
    cwd: Option<&Path>,
    home: Option<&Path>,
    folders_only: bool,
) -> Option<String> {
    let sep = style.folder_separator();
    if typed == "~" {
        return home.map(|_| sep.to_string());
    }
    let split = typed
        .char_indices()
        .filter(|(_, c)| is_separator_char(*c, style))
        .map(|(at, c)| at + c.len_utf8())
        .next_back()
        .unwrap_or(0);
    let (folder, base) = typed.split_at(split);
    let dir: PathBuf = if typed.starts_with('~') && folder.is_empty() {
        /* ~user is left to Tab. */
        return None;
    } else if folder.is_empty() {
        cwd?.to_path_buf()
    } else if let Some(rest) = folder.strip_prefix('~') {
        if !rest.starts_with(|c| is_separator_char(c, style)) {
            /* ~user is left to Tab. */
            return None;
        }
        home?.join(&rest[1..])
    } else {
        let path = Path::new(folder);
        if path.is_absolute() || folder.starts_with('/') {
            path.to_path_buf()
        } else {
            cwd?.join(path)
        }
    };
    let ignore_case = cfg!(windows);
    let hidden = base.starts_with('.');
    let separator = folder
        .chars()
        .rev()
        .find(|c| is_separator_char(*c, style))
        .unwrap_or(sep);
    let mut found: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(&dir).ok()?.take(20_000).flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') && !hidden {
            continue;
        }
        let starts = if ignore_case {
            name.to_lowercase().starts_with(&base.to_lowercase())
        } else {
            name.starts_with(base)
        };
        if !starts {
            continue;
        }
        /* Follows links, so a link to a folder counts as one. */
        let is_dir = entry.path().is_dir();
        if folders_only && !is_dir {
            continue;
        }
        let candidate = if is_dir {
            format!("{name}{separator}")
        } else {
            name
        };
        if candidate.chars().count() > base.chars().count() {
            found.push(candidate);
        }
    }
    found.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()).then(a.cmp(b)));
    found
        .first()
        .map(|name| after_chars(name, base.chars().count()))
}

/// Programs on a PATH, read again at most every 30 seconds per PATH.
fn programs_on(path_var: &str) -> Vec<String> {
    static CACHE: Mutex<Option<(String, Instant, Vec<String>)>> = Mutex::new(None);
    if let Ok(cache) = CACHE.lock() {
        if let Some((key, at, names)) = cache.as_ref() {
            if key == path_var && at.elapsed() < Duration::from_secs(30) {
                return names.clone();
            }
        }
    }
    let mut names = Vec::new();
    for dir in std::env::split_paths(path_var) {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.take(20_000).flatten() {
            if let Some(name) = program_name(&entry) {
                names.push(name);
            }
        }
    }
    names.sort();
    names.dedup();
    if let Ok(mut cache) = CACHE.lock() {
        *cache = Some((path_var.to_string(), Instant::now(), names.clone()));
    }
    names
}

#[cfg(unix)]
fn program_name(entry: &std::fs::DirEntry) -> Option<String> {
    use std::os::unix::fs::PermissionsExt;
    let meta = std::fs::metadata(entry.path()).ok()?;
    (meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        .then(|| entry.file_name().to_string_lossy().into_owned())
}

/// On Windows a program is a file with an extension from PATHEXT, named without it.
#[cfg(windows)]
fn program_name(entry: &std::fs::DirEntry) -> Option<String> {
    let path = entry.path();
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    let pathext = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
    let known = pathext
        .split(';')
        .any(|known| known.trim_start_matches('.').eq_ignore_ascii_case(&ext));
    (known && path.is_file()).then(|| path.file_stem()?.to_str().map(str::to_string))?
}

/// The rest of the likeliest command for `typed`: the shortest name that
/// starts with it, so `gi` is git rather than gifsicle.
fn complete_command(typed: &str, style: Style, path_var: &str, aliases: &[String]) -> Option<String> {
    let ignore_case = style.commands_ignore_case();
    let builtins: &[&str] = match style {
        Style::Posix => POSIX_BUILTINS,
        Style::PowerShell => POWERSHELL_COMMANDS,
        Style::Cmd => CMD_BUILTINS,
    };
    let programs = programs_on(path_var);
    let best = builtins
        .iter()
        .copied()
        .chain(programs.iter().map(String::as_str))
        .chain(aliases.iter().map(String::as_str))
        .filter(|name| extends(name, typed, ignore_case))
        .min_by(|a, b| {
            a.chars()
                .count()
                .cmp(&b.chars().count())
                .then_with(|| a.cmp(b))
        })?;
    Some(after_chars(best, typed.chars().count()))
}

/// `rest` as it has to be typed after the word, or None when it can't be
/// typed plainly there (a quote inside a quote, a space in cmd).
fn quote(rest: &str, style: Style, open: Option<char>) -> Option<String> {
    if rest.is_empty() || rest.chars().any(char::is_control) {
        return None;
    }
    let mut out = String::new();
    match (style, open) {
        (_, Some('\'')) => {
            if rest.contains('\'') {
                return None;
            }
            out.push_str(rest);
        }
        (Style::Cmd, Some(_)) => {
            if rest.contains('"') {
                return None;
            }
            out.push_str(rest);
        }
        (Style::Posix, Some(_)) => {
            if rest.contains('!') {
                return None;
            }
            for c in rest.chars() {
                if matches!(c, '"' | '$' | '`' | '\\') {
                    out.push('\\');
                }
                out.push(c);
            }
        }
        (Style::PowerShell, Some(_)) => {
            for c in rest.chars() {
                if matches!(c, '"' | '$' | '`') {
                    out.push('`');
                }
                out.push(c);
            }
        }
        (Style::Posix, None) => {
            for c in rest.chars() {
                if c.is_whitespace() || "'\"\\$`!&|;<>()*?[]{}#".contains(c) {
                    out.push('\\');
                }
                out.push(c);
            }
        }
        (Style::PowerShell, None) => {
            if rest.contains(['[', ']']) {
                return None;
            }
            for c in rest.chars() {
                if c.is_whitespace() || "'\"`$&|;<>(){}@#,".contains(c) {
                    out.push('`');
                }
                out.push(c);
            }
        }
        (Style::Cmd, None) => {
            if rest.contains(|c: char| c.is_whitespace() || "&|<>^()\"".contains(c)) {
                return None;
            }
            out.push_str(rest);
        }
    }
    Some(out)
}

/// What to show after `line`, already quoted as it would be typed, or None.
fn suggest(
    line: &str,
    style: Style,
    cwd: Option<&Path>,
    home: Option<&Path>,
    path_var: &str,
    aliases: &[String],
) -> Option<String> {
    let word = last_word(line, style)?;
    /* Files and folders where you are, for any word; cd and the like want folders only. */
    let folders_only = word.command.as_deref().is_some_and(|command| {
        FOLDER_COMMANDS
            .iter()
            .any(|known| known.eq_ignore_ascii_case(command))
    });
    let file = complete_path(&word.text, style, cwd, home, folders_only);
    /* No file or folder: the first word of a command may be a command name. */
    let plain_name = !word.text.contains(|c| is_separator_char(c, style) || c == '\\')
        && !word.text.starts_with(['.', '~']);
    let rest = match file {
        Some(rest) => rest,
        None if word.at_command && plain_name => complete_command(&word.text, style, path_var, aliases)?,
        None => return None,
    };
    quote(&rest, style, word.quote)
}

/// The suggestion for the command line `line` typed in terminal `id`. `cwd`
/// and `path` are the folder and PATH the shell last reported; without them
/// it goes by the shell process's folder and Dispatch's own PATH.
#[tauri::command]
pub async fn suggest_completion(
    app: AppHandle,
    id: String,
    line: String,
    cwd: Option<String>,
    path: Option<String>,
) -> Result<Option<String>, String> {
    let Some((style, pid, aliases)) = pty::session_shell(&app.state::<TerminalState>(), &id) else {
        return Ok(None);
    };
    let home = app.path().home_dir().ok();
    /* Reading folders can be slow (a network drive); keep it off the async threads. */
    tauri::async_runtime::spawn_blocking(move || {
        let cwd = cwd
            .map(PathBuf::from)
            .filter(|dir| dir.is_dir())
            .or_else(|| pid.and_then(pty::process_cwd));
        let path_var = path
            .filter(|path| !path.is_empty())
            .or_else(|| std::env::var("PATH").ok())
            .unwrap_or_default();
        suggest(&line, style, cwd.as_deref(), home.as_deref(), &path_var, &aliases)
    })
    .await
    .map_err(|err| format!("Could not suggest: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn word(line: &str) -> Option<Word> {
        last_word(line, Style::Posix)
    }

    /// A folder with: src/ (main.ts, components/), scripts/, My Folder/, .hidden/, README.md, run.sh.
    fn fixture(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("dispatch-suggest-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src/components")).unwrap();
        fs::create_dir_all(root.join("scripts")).unwrap();
        fs::create_dir_all(root.join("My Folder/inner")).unwrap();
        fs::create_dir_all(root.join(".hidden")).unwrap();
        fs::write(root.join("README.md"), "").unwrap();
        fs::write(root.join("run.sh"), "").unwrap();
        fs::write(root.join("src/main.ts"), "").unwrap();
        root
    }

    fn posix(line: &str, cwd: &Path) -> Option<String> {
        suggest(line, Style::Posix, Some(cwd), Some(cwd), "", &[])
    }

    #[test]
    fn reads_the_word_at_the_cursor() {
        let w = word("cd sr").unwrap();
        assert_eq!(w.text, "sr");
        assert!(!w.at_command);
        assert_eq!(w.command.as_deref(), Some("cd"));
        assert!(word("gi").unwrap().at_command);
        assert!(word("sudo gi").unwrap().at_command);
        assert!(word("ls | gr").unwrap().at_command);
        assert!(word("make && ec").unwrap().at_command);
        assert!(!word("cat < fi").unwrap().at_command);
        assert_eq!(word("ls My\\ F").unwrap().text, "My F");
        let quoted = word("ls \"My F").unwrap();
        assert_eq!((quoted.text.as_str(), quoted.quote), ("My F", Some('"')));
        assert_eq!(word("npm run --prefix=sr").unwrap().text, "sr");
    }

    #[test]
    fn leaves_some_words_to_tab() {
        assert_eq!(word("cd "), None);
        assert_eq!(word(""), None);
        assert_eq!(word("ls $HO"), None);
        assert_eq!(word("ls *.t"), None);
        assert_eq!(word("echo \"$(da"), None);
        assert_eq!(word("ls foo\\"), None);
    }

    #[test]
    fn suggests_files_and_folders() {
        let root = fixture("files");
        assert_eq!(posix("cd sr", &root).as_deref(), Some("c/"));
        assert_eq!(posix("cd src", &root).as_deref(), Some("/"));
        assert_eq!(posix("ls src/m", &root).as_deref(), Some("ain.ts"));
        assert_eq!(posix("cd src/c", &root).as_deref(), Some("omponents/"));
        assert_eq!(posix("cat REA", &root).as_deref(), Some("DME.md"));
        /* Alphabetical: scripts/ before src/. */
        assert_eq!(posix("ls s", &root).as_deref(), Some("cripts/"));
        assert_eq!(posix("ls ./ru", &root).as_deref(), Some("n.sh"));
        assert_eq!(posix("ls nothing", &root), None);
        /* No command needed in front. */
        assert_eq!(posix("sr", &root).as_deref(), Some("c/"));
        assert_eq!(posix("REA", &root).as_deref(), Some("DME.md"));
        assert_eq!(posix("src/m", &root).as_deref(), Some("ain.ts"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn cd_suggests_only_folders() {
        let root = fixture("folders");
        assert_eq!(posix("cat r", &root).as_deref(), Some("un.sh"));
        assert_eq!(posix("cd r", &root), None);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn hidden_files_need_a_dot() {
        let root = fixture("hidden");
        assert_eq!(posix("ls .h", &root).as_deref(), Some("idden/"));
        assert_eq!(posix("ls h", &root), None);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn quotes_what_it_adds() {
        let root = fixture("quotes");
        assert_eq!(posix("ls My", &root).as_deref(), Some("\\ Folder/"));
        assert_eq!(posix("ls My\\ F", &root).as_deref(), Some("older/"));
        assert_eq!(posix("ls \"My", &root).as_deref(), Some(" Folder/"));
        assert_eq!(posix("ls 'My", &root).as_deref(), Some(" Folder/"));
        assert_eq!(
            suggest("ls My", Style::PowerShell, Some(&root), None, "", &[]).as_deref(),
            Some("` Folder/")
        );
        assert_eq!(suggest("dir My", Style::Cmd, Some(&root), None, "", &[]), None);
        assert_eq!(
            suggest("dir \"My", Style::Cmd, Some(&root), None, "", &[]).as_deref(),
            Some(" Folder/")
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn expands_home() {
        let root = fixture("home");
        assert_eq!(posix("cd ~/sr", &root).as_deref(), Some("c/"));
        assert_eq!(posix("cd ~", &root).as_deref(), Some("/"));
        assert_eq!(posix("cd ~ro", &root), None);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn suggests_commands_shortest_first() {
        let root = fixture("commands");
        let bin = root.join("bin");
        fs::create_dir_all(&bin).unwrap();
        for name in ["git", "gifsicle", "git-lfs"] {
            let path = bin.join(name);
            fs::write(&path, "").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
            }
        }
        let path_var = bin.to_string_lossy().into_owned();
        let at = |line: &str| suggest(line, Style::Posix, Some(&root), None, &path_var, &[]);
        assert_eq!(at("gi").as_deref(), Some("t"));
        assert_eq!(at("git-").as_deref(), Some("lfs"));
        assert_eq!(at("sudo gi").as_deref(), Some("t"));
        assert_eq!(at("ech").as_deref(), Some("o"));
        /* Not where a command goes: only files. */
        assert_eq!(at("echo gi"), None);
        /* A file or folder wins over a command: bin/ here. */
        assert_eq!(at("bi").as_deref(), Some("n/"));
        assert_eq!(
            suggest("get-chi", Style::PowerShell, None, None, "", &[]).as_deref(),
            Some("ldItem")
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn suggests_aliases_as_commands() {
        let aliases = ["gst".to_string(), "deploy-staging".to_string()];
        let at = |line: &str| suggest(line, Style::Posix, None, None, "", &aliases);
        assert_eq!(at("deploy-s").as_deref(), Some("taging"));
        assert_eq!(at("gs").as_deref(), Some("t"));
        assert_eq!(at("echo depl"), None);
    }

    #[test]
    fn knows_shells() {
        assert_eq!(Style::for_shell("/bin/zsh"), Style::Posix);
        assert_eq!(Style::for_shell("C:\\Program Files\\PowerShell\\7\\pwsh.exe"), Style::PowerShell);
        assert_eq!(Style::for_shell("powershell.exe"), Style::PowerShell);
        assert_eq!(Style::for_shell("C:\\Windows\\System32\\cmd.exe"), Style::Cmd);
    }
}
