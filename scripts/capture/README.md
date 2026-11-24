# Screenshot and Demo Capture Automation

These scripts generate the screenshots and demo GIF in Dispatch's README.

## Overview

The capture runs on macOS against a real Dispatch.app. It builds a throwaway HOME with an invented Node project, launches Dispatch in it, types real commands, and captures the window with `screencapture`.

Output:

- `docs/screenshots/01-terminal.png`: a tab with git log, git status and a test run as command blocks
- `docs/screenshots/02-split-panes.png`: three panes, with a running server and a request to it
- `docs/screenshots/03-settings-general.png`: Settings › General
- `docs/screenshots/04-settings-terminal.png`: Settings › Terminal
- `docs/demo.gif`: commands typed live, then the tab split side by side and stacked

## Prerequisites

- **macOS** on a screen of at least 1464×960 points
- **A Dispatch.app**. The script uses, in order: `DISPATCH_APP` if set, the fresh build at `src-tauri/target/release/bundle/macos/Dispatch.app`, then `/Applications/Dispatch.app`
- **git** and **Node**. Homebrew's `node` is used first, since a version manager's shim (fnm, lerd) may not work under the throwaway HOME
- **ffmpeg**, for the GIF: `brew install ffmpeg`
- **Accessibility** and **Screen Recording** permission for the app that runs the script (Terminal, iTerm, Cursor, Claude, ...), in *System Settings › Privacy & Security*

## Usage

Build the app from source, then run the capture:

```bash
bun run tauri build --bundles app
```

```bash
bash scripts/capture/capture.sh
```

The build ends with an error about `TAURI_SIGNING_PRIVATE_KEY` unless the updater key is set. That step only signs the update archive; `Dispatch.app` is already built by then.

Other options:

```bash
bash scripts/capture/capture.sh --dry-run              # build the demo HOME only
bash scripts/capture/capture.sh --only 02-split-panes  # write just one screenshot (and no GIF)
```

Don't touch the mouse or keyboard while it runs, about a minute. Review the images, then commit them.

## How it works

1. **Demo HOME.** `generate-demo-data.sh` makes `/private/tmp/dispatch-capture/home` (the real path, so the prompt shows `~`) with a small `.zshrc`, a `.gitconfig` with a made-up identity and no pager, and `~/Projects/acme-web`: an invented storefront with git history, a tag, a feature branch and three uncommitted changes. Every name and address in it is made up.
2. **Isolated settings.** The app runs with `HOME` set to that folder, so it reads a seeded `settings.json`: dark theme, zsh, new tabs in the demo project, a solid background (the GIF records a screen region, and a see-through window would show what's behind it), no global hotkey, and quitting really quits.
3. **Running alongside your Dispatch.** Dispatch is single-instance: a second launch hands over to the running copy through `/tmp/com_dispatch_app_si.sock` and exits. If that socket exists, the script moves it aside while the capture copy runs and puts it back afterwards. Your copy, the installed app or `bun run tauri dev`, keeps running and is never quit.
4. **localStorage.** The webview keeps the theme in `~/Library/WebKit/com.dispatch.app`, which HOME doesn't redirect. `localstorage.py` saves every row before the run and restores them exactly afterwards.
5. **Driving the UI.** Commands are typed a key at a time through System Events; splits, new tabs and Settings use the app's own shortcuts (⌘D, ⌘⇧D, ⌘T, ⌘,). The one click, on Settings › Terminal, is a CSS pixel position in the 1440×900 window, whose webview fills the whole window under the overlay title bar.
6. **GIF.** `screencapture -v` records the window region, and ffmpeg turns it into a 960 pixel wide, 12 fps GIF with a generated palette.

## Troubleshooting

- **The Terminal settings click misses.** Set `TERMINAL_NAV_X` and `TERMINAL_NAV_Y` to the item's position in points from the window's top left (half its pixel position in `03-settings-general.png`).
- **A chip is missing from the prompt.** The Node chip needs a `node` that runs under the throwaway HOME; install one with Homebrew.
- **"Operation not permitted" or blank captures.** Grant Accessibility and Screen Recording to the app running the script and restart it.
- **Port 4173 is taken.** Change `DEMO_PORT` in `capture.sh`.

## Files

```
scripts/capture/
├── capture.sh              # main script
├── generate-demo-data.sh   # the throwaway HOME and demo project
├── mouse.js                # CoreGraphics clicks (JXA)
├── localstorage.py         # saves and restores the webview's localStorage
└── README.md
```
