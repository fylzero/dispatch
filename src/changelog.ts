export interface ChangelogRelease {
  version: string;
  date: string;
  notes: string[];
}

export const CHANGELOG: ChangelogRelease[] = [
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
