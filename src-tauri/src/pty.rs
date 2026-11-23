use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use tauri::ipc::{Channel, Response};
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

struct TerminalSession {
    writer: Mutex<Box<dyn Write + Send>>,
    master: Box<dyn MasterPty + Send>,
    child: Mutex<Box<dyn portable_pty::Child + Send + Sync>>,
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
        }
    }
}

#[derive(Clone, Default)]
pub struct TerminalState {
    sessions: Arc<Mutex<HashMap<String, TerminalSession>>>,
}

/// The user's login shell from the password database. Apps opened from
/// Finder or a Linux desktop launcher don't always get $SHELL, so this is the
/// reliable answer on Unix.
#[cfg(unix)]
fn passwd_shell() -> Option<String> {
    use std::ffi::CStr;
    // SAFETY: getpwuid returns a pointer into static storage or null; the
    // shell string is copied out before anything else can call it.
    unsafe {
        let entry = libc::getpwuid(libc::getuid());
        if entry.is_null() || (*entry).pw_shell.is_null() {
            return None;
        }
        let shell = CStr::from_ptr((*entry).pw_shell)
            .to_string_lossy()
            .into_owned();
        (!shell.trim().is_empty()).then_some(shell)
    }
}

/// The first of `names` found on PATH.
#[cfg(windows)]
fn find_on_path(names: &[&str]) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    names.iter().find_map(|name| {
        std::env::split_paths(&path)
            .map(|dir| dir.join(name))
            .find(|candidate| candidate.is_file())
            .map(|found| found.to_string_lossy().into_owned())
    })
}

#[cfg(unix)]
fn default_shell() -> String {
    std::env::var("SHELL")
        .ok()
        .filter(|shell| !shell.trim().is_empty())
        .or_else(passwd_shell)
        .unwrap_or_else(|| {
            if cfg!(target_os = "macos") {
                "/bin/zsh".into()
            } else {
                "/bin/sh".into()
            }
        })
}

/// PowerShell 7 if it's installed, else Windows PowerShell, else cmd.
#[cfg(windows)]
fn default_shell() -> String {
    find_on_path(&["pwsh.exe", "powershell.exe"])
        .or_else(|| std::env::var("COMSPEC").ok())
        .unwrap_or_else(|| "cmd.exe".into())
}

fn lock_sessions(
    state: &TerminalState,
) -> Result<std::sync::MutexGuard<'_, HashMap<String, TerminalSession>>, String> {
    state
        .sessions
        .lock()
        .map_err(|_| "Could not lock terminal sessions.".to_string())
}

fn pty_size(cols: u16, rows: u16) -> PtySize {
    PtySize {
        rows: rows.max(2),
        cols: cols.max(20),
        pixel_width: 0,
        pixel_height: 0,
    }
}

/// The folder a new tab starts in: the one asked for if it exists, else home.
fn start_dir(app: &AppHandle, cwd: Option<String>) -> Option<PathBuf> {
    cwd.map(|path| PathBuf::from(path.trim()))
        .filter(|path| path.is_dir())
        .or_else(|| app.path().home_dir().ok())
}

/// Starts a shell in a new pseudo-terminal. Output streams back as raw bytes
/// on `on_data` (xterm decodes UTF-8 across chunk boundaries itself), and
/// `on_exit` fires once when the shell goes away.
/* Each argument is a separate IPC parameter from the page, so the count is the API. */
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn open_terminal(
    app: AppHandle,
    state: State<TerminalState>,
    cols: u16,
    rows: u16,
    shell: Option<String>,
    cwd: Option<String>,
    on_data: Channel<Response>,
    on_exit: Channel<()>,
) -> Result<String, String> {
    let pair = native_pty_system()
        .openpty(pty_size(cols, rows))
        .map_err(|err| format!("Could not open a terminal: {err}"))?;

    let shell = shell
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(default_shell);
    let mut cmd = CommandBuilder::new(&shell);
    #[cfg(unix)]
    {
        cmd.arg("-l");
    }
    if let Some(dir) = start_dir(&app, cwd) {
        cmd.cwd(dir);
    }
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("TERM_PROGRAM", "Dispatch");
    cmd.env(
        "TERM_PROGRAM_VERSION",
        app.package_info().version.to_string(),
    );
    /* Apps opened from Finder get no locale, which breaks Unicode in the shell. */
    #[cfg(unix)]
    if std::env::var_os("LANG").is_none() {
        cmd.env("LANG", "en_US.UTF-8");
    }

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|err| format!("Could not start {shell}: {err}"))?;
    drop(pair.slave);
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|err| format!("Could not read from the terminal: {err}"))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|err| format!("Could not write to the terminal: {err}"))?;

    let id = Uuid::new_v4().to_string();
    lock_sessions(&state)?.insert(
        id.clone(),
        TerminalSession {
            writer: Mutex::new(writer),
            master: pair.master,
            child: Mutex::new(child),
        },
    );

    let session_id = id.clone();
    let sessions = state.sessions.clone();
    thread::spawn(move || {
        let mut buf = [0u8; 16 * 1024];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if on_data.send(Response::new(buf[..n].to_vec())).is_err() {
                        break;
                    }
                }
            }
        }
        if let Ok(mut map) = sessions.lock() {
            map.remove(&session_id);
        }
        let _ = on_exit.send(());
    });

    Ok(id)
}

#[tauri::command]
pub fn write_terminal(state: State<TerminalState>, id: String, data: String) -> Result<(), String> {
    let sessions = lock_sessions(&state)?;
    let session = sessions
        .get(&id)
        .ok_or_else(|| "Terminal is not open.".to_string())?;
    let mut writer = session
        .writer
        .lock()
        .map_err(|_| "Could not lock the terminal writer.".to_string())?;
    writer
        .write_all(data.as_bytes())
        .and_then(|_| writer.flush())
        .map_err(|err| format!("Could not write to the terminal: {err}"))
}

#[tauri::command]
pub fn resize_terminal(
    state: State<TerminalState>,
    id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let sessions = lock_sessions(&state)?;
    let session = sessions
        .get(&id)
        .ok_or_else(|| "Terminal is not open.".to_string())?;
    session
        .master
        .resize(pty_size(cols, rows))
        .map_err(|err| format!("Could not resize the terminal: {err}"))
}

#[tauri::command]
pub fn close_terminal(state: State<TerminalState>, id: String) -> Result<(), String> {
    lock_sessions(&state)?.remove(&id);
    Ok(())
}
