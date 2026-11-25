export interface ChangelogRelease {
  version: string;
  date: string;
  notes: string[];
}

export const CHANGELOG: ChangelogRelease[] = [
  {
    version: "1.0.0",
    date: "November 24, 2026",
    notes: [
      "The command line works like a text field. Click anywhere in the command to move the cursor there, or below it to jump to the end. Drag across the command to select part of it (the cursor hides while you do), then press Backspace or Delete to remove it, or just type to replace it.",
      "Cut, Copy, and Paste work on a selection in the command, as in any text editor: Cut copies it and removes it, and Paste replaces it. Cut is new in the Edit menu, and on Windows and Linux it's Ctrl+Shift+X.",
      "Pasted text looks like the rest of the command instead of standing out highlighted, in both zsh and bash.",
      "Drag files or folders from Finder onto a terminal to type their paths, escaped the way Terminal.app does it. They go to the pane under the cursor, which shows an outline while you drag.",
      "Command+A at the Dispatch prompt with nothing typed no longer selects the whole screen, and spaces the shell leaves behind after deleting a tab no longer count as part of the command.",
      "Settings has a new Appearance tab with the theme, font, text size, line height, window opacity, and background blur, as in Lockshot, Recon, and Shipyard. Terminal keeps the cursor and prompt settings.",
      "A new Dim unfocused panes slider in Appearance sets how much the other panes in a split fade while one has focus, from off to 80%. The default, 40%, is the look you had before.",
      "A new JSON tab edits your settings file directly, with Copy, Reveal, Import, Export, Discard, and Save. It replaces the Settings file row in General.",
      "Each command you run leaves a snapshot of its prompt above its output: a muted card with a clock that counts how long the command has run, to the millisecond (12.300s, then 2m05.120s and 2h12m23.043s for long ones), and shows the exit code if it fails.",
      "When a command's snapshot scrolls past the top of the pane, it stays pinned there until the next command's snapshot pushes it off.",
      "Dragging the line between split panes is smooth now: the text glides with it instead of shuddering, and each terminal resizes once when you let go.",
      "A little more room around the line you type on: a small gap between the prompt's chips and the cursor, and more space below it at the bottom of the pane.",
      "Right-click the folder or branch chip in a prompt to copy the full path or the branch name.",
      "Splitting a pane (Command-D or Command-Shift-D) opens the new pane in the folder you're in. New tabs still open in your start directory.",
    ],
  },
  {
    version: "0.2.4",
    date: "November 23, 2026",
    notes: [
      "New default terminal settings: 16px text with 1.20 line height, a bar cursor, and an 8px background blur. Reset in Terminal settings now returns to these values.",
    ],
  },
  {
    version: "0.2.3",
    date: "November 23, 2026",
    notes: [
      "On macOS, Dispatch has a refreshed icon in the Dock and the Command-Tab switcher. It now shows as a proper rounded square on the latest macOS, instead of shrunk inside a gray tile with thin lines along its edges.",
    ],
  },
  {
    version: "0.2.2",
    date: "November 23, 2026",
    notes: [
      "On macOS, the Dispatch icon in the Dock and the Command-Tab switcher now follows the system's rounded-square shape exactly, so the thin lines along its top and bottom edges are gone.",
    ],
  },
  {
    version: "0.2.1",
    date: "November 23, 2026",
    notes: [
      "On macOS, Dispatch has a new icon in the Dock and the Command-Tab switcher, so it's easier to tell apart from Lockshot, Recon, and Shipyard at a glance. The logo inside the app and everywhere else is unchanged.",
    ],
  },
  {
    version: "0.2.0",
    date: "November 23, 2026",
    notes: [
      "Command+` shows or hides Dispatch from any app (Ctrl+` on Windows and Linux). It opens on the desktop you're on, and hiding it takes you back to the app you were in. Record a different shortcut, or turn it off, under Toggle hotkey in General settings.",
      "Dispatch now has a menu bar icon (in the notification area on Windows and Linux) to show or hide it, open a new tab, open Settings, check for updates, or quit.",
      "Command+Q and the close button now hide Dispatch instead of quitting, and your tabs keep running for next time. Closing the last tab hides it too, with a fresh tab ready. Quit from the menu bar icon to really quit, or turn off Keep running in the background in General settings to quit the usual way.",
      "A new prompt in zsh and bash shows the project's Node version, the current folder, the git branch, and how many files have changed, each in its own chip. Your usual shell setup still loads underneath. Turn it off with Dispatch prompt in Terminal settings to get your own prompt back in new tabs.",
      "The prompt now stays at the bottom of the pane, with output growing upward above it until the pane fills. With the Dispatch prompt on, a thin line separates each command and its output from the next, and each command shows how long it ran at the right end of its prompt line, along with its exit code when it fails. bash shows run times from version 4.4 on; the bash that ships with macOS is older.",
      "Commands are now typed on their own line under the Dispatch prompt, so long commands start at the left edge. To type right after the chips again, turn off Type below the prompt in Terminal settings. Shift+Enter adds a new line to the command instead of running it (in bash 4.4 and later it continues the command on the next line).",
      "At the Dispatch prompt, Command+A selects just the command you're typing, every line of it, and Backspace then deletes it all. With nothing typed, Command+A still selects everything in the terminal.",
      "On macOS, Command+Left and Command+Right jump to the start or end of the line, and Option+Left and Option+Right move a word at a time. Add Shift to either to select instead, as in any Mac text field: each press extends the selection from the cursor, and at the Dispatch prompt Backspace then deletes it. At the Dispatch prompt, words stop at punctuation, so each part of a path is its own word.",
      "The terminal's scrollbar is now slim and rounded like the rest of macOS, and it fades when you stop scrolling. Text no longer runs underneath it, and the line you're typing has a little more room above the bottom edge.",
      "Choose the folder new tabs open in with Starting directory in General settings. It starts as ~, your home folder, which new tabs also fall back to if the folder you pick is missing.",
      "On macOS, the window's corners are rounded cleanly again. The see-through background and its blur no longer show past the rounded corners as faint squares.",
      "The GitHub link and the version number sit in the tab bar next to the settings gear, and the bar along the bottom of the window is gone. Click the version to open the Change Log.",
    ],
  },
  {
    version: "0.1.0",
    date: "November 23, 2026",
    notes: [
      "First version of Dispatch: a plain, fast terminal with tabs.",
      "Open as many tabs as you like with the + button or Command+T, and close them with Command+W. Each tab runs your login shell.",
      "Settings let you pick the theme, the shell new tabs start, and scrollback under General, and the font, font size, line height, and cursor under Terminal.",
      "The terminal background is see-through at 80% opacity by default. Change it with Window opacity in Terminal settings.",
      "What shows through the background gets a mild blur, adjustable with Background blur in Terminal settings. Windows uses its Acrylic blur, which is on or off.",
      "Command+D splits the focused pane side by side and Command+Shift+D splits it top and bottom. Only that pane is divided, so splits nest as deep as you like, up to 8 side by side or 12 stacked in one split. Drag the dividers to resize, move between panes with Command+Option+arrow keys, and Command+W closes the focused pane.",
      "On macOS the close, minimize, and zoom buttons sit in the tab bar instead of a separate title bar. Drag empty space in the tab bar to move the window, and double-click it to zoom.",
      "Dispatch checks GitHub for a newer version on launch and offers to install it. The Change Log has a Check for Updates button, and so does the Dispatch menu.",
      "Command+K clears the screen, and Command+Plus and Command+Minus change the font size.",
    ],
  },
];
