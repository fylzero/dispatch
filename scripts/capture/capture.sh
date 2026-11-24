#!/bin/bash
# Automated screenshot and demo GIF capture for the Dispatch README.
#
# Runs a Dispatch.app against an isolated HOME with an invented Node project,
# types real commands into it, and writes:
#   docs/screenshots/01-terminal.png ... 04-settings-terminal.png
#   docs/demo.gif
#
# Requirements (macOS, Apple Silicon):
#   - A Dispatch.app: the fresh build in src-tauri/target/release (bun run
#     tauri build --bundles app), else /Applications/Dispatch.app, or set
#     DISPATCH_APP to another bundle
#   - zsh, git, a Node install (Homebrew's is used first), ffmpeg (brew install ffmpeg)
#   - Accessibility and Screen Recording permission for the app running this
#     script (Terminal, Cursor, ...). See scripts/capture/README.md.
#
# Usage:
#   scripts/capture/capture.sh [--dry-run] [--only <name>]
#   names: 01-terminal 02-split-panes 03-settings-general 04-settings-terminal
#
# Safety:
#   - A Dispatch that is already running (the installed app or `tauri dev`)
#     keeps running and is never quit. Its single-instance socket is set aside
#     while the capture copy runs, then put back.
#   - Your real settings, shell history and shell config are untouched. HOME is
#     redirected for the app and its shells, and the webview's localStorage is
#     saved first and restored afterwards (see localstorage.py).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SCREENSHOTS_DIR="$PROJECT_ROOT/docs/screenshots"
DEMO_GIF_PATH="$PROJECT_ROOT/docs/demo.gif"

# /tmp is a symlink; the real path keeps the prompt showing ~ for HOME.
WORK_DIR="/private/tmp/dispatch-capture"
ISOLATED_HOME="$WORK_DIR/home"
STORAGE_BACKUP="$WORK_DIR/localstorage.json"
SOCKET="/tmp/com_dispatch_app_si.sock"   # tauri-plugin-single-instance
SOCKET_ASIDE="$SOCKET.capture-aside"
DEMO_PORT=4173

WINDOW_WIDTH=1440
WINDOW_HEIGHT=900
WINDOW_Y=40

DRY_RUN=false
ONLY_SHOT=""
APP_PID=""
RECORD_PID=""

# ---------------------------------------------------------------- output ----
BLUE=$'\033[34m'; GREEN=$'\033[32m'; RED=$'\033[31m'; YELLOW=$'\033[33m'; NC=$'\033[0m'
info()    { echo "${BLUE}▸${NC} $*"; }
success() { echo "${GREEN}✓${NC} $*"; }
warn()    { echo "${YELLOW}!${NC} $*" >&2; }
die()     { echo "${RED}✗${NC} $*" >&2; exit 1; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=true; shift ;;
    --only) ONLY_SHOT="${2:-}"; shift 2 ;;
    --only=*) ONLY_SHOT="${1#*=}"; shift ;;
    -h|--help) sed -n '2,28p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) die "Unknown option: $1" ;;
  esac
done

# ------------------------------------------------------------- the app ------
if [[ -n "${DISPATCH_APP:-}" ]]; then
  APP_BUNDLE="$DISPATCH_APP"
elif [[ -d "$PROJECT_ROOT/src-tauri/target/release/bundle/macos/Dispatch.app" ]]; then
  APP_BUNDLE="$PROJECT_ROOT/src-tauri/target/release/bundle/macos/Dispatch.app"
else
  APP_BUNDLE="/Applications/Dispatch.app"
fi

[[ -x "$APP_BUNDLE/Contents/MacOS/dispatch" ]] || die "No Dispatch.app at $APP_BUNDLE (build one with: bun run tauri build --bundles app)"
command -v ffmpeg >/dev/null 2>&1 || die "ffmpeg is required: brew install ffmpeg"
command -v python3 >/dev/null 2>&1 || die "python3 is required"
command -v node >/dev/null 2>&1 || die "node is required (the demo project is a Node app)"

# --------------------------------------------------------------- cleanup ----
STORAGE_SAVED=false
cleanup() {
  local code=$?
  [[ -n "$RECORD_PID" ]] && kill -INT "$RECORD_PID" 2>/dev/null || true
  # Quit only the instance this script started, and wait so its webview has
  # written localStorage before the saved rows go back.
  if [[ -n "$APP_PID" ]] && kill -0 "$APP_PID" 2>/dev/null; then
    kill "$APP_PID" 2>/dev/null || true
    for _ in $(seq 1 20); do kill -0 "$APP_PID" 2>/dev/null || break; sleep 0.25; done
  fi
  # The demo server, should the shell it ran in outlive the app.
  local pid
  for pid in $(lsof -ti "tcp:$DEMO_PORT" -sTCP:LISTEN 2>/dev/null); do
    lsof -a -p "$pid" -d cwd -Fn 2>/dev/null | grep -q "^n$ISOLATED_HOME" && kill "$pid" 2>/dev/null || true
  done
  [[ "$STORAGE_SAVED" == true ]] && python3 "$SCRIPT_DIR/localstorage.py" restore "$STORAGE_BACKUP" || true
  if [[ -S "$SOCKET_ASIDE" ]]; then
    rm -f "$SOCKET"
    mv "$SOCKET_ASIDE" "$SOCKET"
  fi
  rm -rf "$WORK_DIR" 2>/dev/null || true
  exit $code
}
trap cleanup EXIT

# -------------------------------------------------------------- demo data ---
rm -rf "$WORK_DIR"
mkdir -p "$ISOLATED_HOME/Library/Application Support/com.dispatch.app"

info "Generating the demo project"
bash "$SCRIPT_DIR/generate-demo-data.sh" "$ISOLATED_HOME" >/dev/null

# Dark theme, the shell is zsh, new tabs open in the demo project, no global
# hotkey (your running Dispatch owns ⌘`), and quitting really quits. The
# background is solid: the GIF records a screen region, and a see-through
# window would show whatever is open behind it.
cat > "$ISOLATED_HOME/Library/Application Support/com.dispatch.app/settings.json" <<'JSON'
{
  "theme": "dark",
  "shell": "/bin/zsh",
  "startDirectory": "~/Projects/acme-web",
  "toggleHotkey": "",
  "keepRunningInBackground": false,
  "opacity": 1
}
JSON
success "Demo data ready"

if [[ "$DRY_RUN" == true ]]; then
  warn "Dry run: not launching the app or capturing anything"
  exit 0
fi

# Screen size decides where the window can sit.
read -r SCREEN_W SCREEN_H < <(osascript -l JavaScript -e 'ObjC.import("AppKit"); var f=$.NSScreen.mainScreen.frame; Math.round(f.size.width)+" "+Math.round(f.size.height)')
(( SCREEN_W >= WINDOW_WIDTH + 24 && SCREEN_H >= WINDOW_HEIGHT + WINDOW_Y + 20 )) \
  || die "Screen is ${SCREEN_W}x${SCREEN_H}; need at least $((WINDOW_WIDTH + 24))x$((WINDOW_HEIGHT + WINDOW_Y + 20)) for a ${WINDOW_WIDTH}x${WINDOW_HEIGHT} window"
WINDOW_X=$(( (SCREEN_W - WINDOW_WIDTH) / 2 ))

# A running Dispatch owns the single-instance socket, and a second launch would
# just show that one and exit. Set the socket aside (the running copy keeps
# listening on it) and put it back in cleanup.
if [[ -S "$SOCKET" ]]; then
  warn "Dispatch is already running. It will not be touched; only the copy started here is controlled."
  mv "$SOCKET" "$SOCKET_ASIDE"
fi

python3 "$SCRIPT_DIR/localstorage.py" backup "$STORAGE_BACKUP"
STORAGE_SAVED=true

info "Launching $(basename "$APP_BUNDLE") with an isolated HOME"
HOME="$ISOLATED_HOME" SHELL=/bin/zsh ZDOTDIR="" "$APP_BUNDLE/Contents/MacOS/dispatch" >/dev/null 2>&1 &
APP_PID=$!

# ------------------------------------------------------------- helpers ------
MOUSE=(osascript -l JavaScript "$SCRIPT_DIR/mouse.js")

window_id() {
  osascript -l JavaScript -e "ObjC.import('CoreGraphics');
    var l=ObjC.deepUnwrap(ObjC.castRefToObject(\$.CGWindowListCopyWindowInfo(\$.kCGWindowListOptionOnScreenOnly,0)));
    l.filter(w=>w.kCGWindowOwnerPID==$APP_PID&&w.kCGWindowLayer==0&&w.kCGWindowBounds.Width>500).map(w=>w.kCGWindowNumber).join('')" 2>/dev/null
}

app_sys_events() {  # run an AppleScript snippet "inside" our process
  osascript -e "tell application \"System Events\" to tell (first process whose unix id is $APP_PID) to $1" 2>/dev/null
}

bring_to_front() {
  for _ in 1 2 3 4 5 6 7 8 9 10; do
    osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $APP_PID) to true" 2>/dev/null || true
    [[ "$(osascript -e 'tell application "System Events" to get unix id of first process whose frontmost is true' 2>/dev/null)" == "$APP_PID" ]] && return 0
    sleep 0.3
  done
  die "Could not bring Dispatch to the front"
}

# Coordinates are CSS pixels in the webview, which fills the whole window (the
# title bar is an overlay on the tab bar).
click() { bring_to_front; "${MOUSE[@]}" click $(( WINDOW_X + $1 )) $(( WINDOW_Y + $2 )); sleep "${3:-0.8}"; }
key() {
  local using=""
  [[ -n "${2:-}" ]] && using=" using {$2}"
  bring_to_front
  osascript -e "tell application \"System Events\" to keystroke \"$1\"$using"
  sleep "${3:-0.4}"
}
press_return() { osascript -e 'tell application "System Events" to key code 36'; sleep "${1:-0.3}"; }

# Types a command a key at a time, as a person would, then runs it.
run_cmd() {
  local cmd="$1" wait="${2:-1.2}" pace="${3:-0.035}"
  bring_to_front
  osascript - "$cmd" "$pace" <<'OSA'
on run argv
  set theText to item 1 of argv
  set pace to (item 2 of argv) as real
  tell application "System Events"
    repeat with c in characters of theText
      keystroke c
      delay pace
    end repeat
  end tell
end run
OSA
  sleep 0.2
  press_return "$wait"
}

# Park the pointer on the empty middle of the tab bar, clear of tabs, panes
# and dividers, so nothing shows a hover state.
park_pointer() { "${MOUSE[@]}" move $(( WINDOW_X + 1000 )) $(( WINDOW_Y + 18 )); sleep 0.4; }

capture() {
  local name="$1"
  [[ -n "$ONLY_SHOT" && "$ONLY_SHOT" != "$name" ]] && return 0
  mkdir -p "$SCREENSHOTS_DIR"
  local wid; wid="$(window_id)"
  [[ -n "$wid" ]] || die "No Dispatch window found for pid $APP_PID"
  screencapture -l "$wid" -o "$SCREENSHOTS_DIR/$name.png" \
    || die "screencapture failed (is Screen Recording allowed for this app?)"
  success "Captured $name.png"
}

# --------------------------------------------------------------- window -----
for _ in $(seq 1 40); do [[ -n "$(window_id)" ]] && break; sleep 0.5; done
[[ -n "$(window_id)" ]] || die "Dispatch window did not appear"
sleep 1.5
app_sys_events "tell window 1 to set position to {$WINDOW_X, $WINDOW_Y}"
app_sys_events "tell window 1 to set size to {$WINDOW_WIDTH, $WINDOW_HEIGHT}"
sleep 1.5
bring_to_front
success "Window ready (${WINDOW_WIDTH}x${WINDOW_HEIGHT} at $WINDOW_X,$WINDOW_Y)"

# Click into the terminal so it has keyboard focus, and let the shell start.
click 720 450 2.5

# ----------------------------------------------------------- screenshots ----
# 1. A tab with a few command blocks
run_cmd "git log --oneline --graph" 1
run_cmd "git status -s" 1
run_cmd "node --test" 2.5
park_pointer
capture 01-terminal

# 2. Split panes: the server on the right, a request to it below
key "d" "command down" 2
run_cmd "PORT=$DEMO_PORT node src/server.js" 1.5
key "d" "command down, shift down" 2
run_cmd "curl -s localhost:$DEMO_PORT/api/products; echo" 1.5
run_cmd "ls src test" 1
park_pointer
capture 02-split-panes

# 3 and 4. Settings
key "," "command down" 1.5
park_pointer
capture 03-settings-general
click "${TERMINAL_NAV_X:-90}" "${TERMINAL_NAV_Y:-118}" 1
park_pointer
capture 04-settings-terminal

# ----------------------------------------------------------------- GIF ------
if [[ -z "$ONLY_SHOT" ]]; then
  info "Recording the demo GIF"
  # Close Settings, then start from a fresh tab.
  key "w" "command down" 1
  key "t" "command down" 2.5

  TEMP_VIDEO="$WORK_DIR/demo.mov"
  PALETTE="$WORK_DIR/palette.png"
  screencapture -v -R"$WINDOW_X,$WINDOW_Y,$WINDOW_WIDTH,$WINDOW_HEIGHT" "$TEMP_VIDEO" 2>/dev/null &
  RECORD_PID=$!
  sleep 1.5

  run_cmd "git status -s" 1.2 0.06
  run_cmd "node --test" 2.5 0.06
  key "d" "command down" 2
  run_cmd "git log --oneline --graph" 2 0.06
  key "d" "command down, shift down" 2
  run_cmd "cat src/cart.js" 2.5 0.06
  park_pointer
  sleep 1

  kill -INT "$RECORD_PID" 2>/dev/null || true
  wait "$RECORD_PID" 2>/dev/null || true
  RECORD_PID=""
  sleep 1
  [[ -s "$TEMP_VIDEO" ]] || die "Recording failed (is Screen Recording allowed for this app?)"

  ffmpeg -loglevel error -y -i "$TEMP_VIDEO" -vf "fps=12,scale=960:-1:flags=lanczos,palettegen=stats_mode=diff" "$PALETTE"
  ffmpeg -loglevel error -y -i "$TEMP_VIDEO" -i "$PALETTE" \
    -lavfi "fps=12,scale=960:-1:flags=lanczos[x];[x][1:v]paletteuse=dither=bayer:bayer_scale=5" "$DEMO_GIF_PATH"

  size_bytes=$(stat -f%z "$DEMO_GIF_PATH")
  success "Demo GIF created: docs/demo.gif ($((size_bytes / 1024)) KB)"
  (( size_bytes > 8388608 )) && warn "GIF is over 8 MB; shorten the flow or lower the fps"
fi

success "Done. Review docs/screenshots/*.png and docs/demo.gif, then commit them."
