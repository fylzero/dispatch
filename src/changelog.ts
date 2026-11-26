export interface ChangelogRelease {
  version: string;
  date: string;
  notes: string[];
}

export const CHANGELOG: ChangelogRelease[] = [
  {
    version: "1.3.0",
    date: "November 26, 2026",
    notes: [
      "Settings has a new Hotkeys tab, as in Lockshot. Toggle hotkey, the shortcut that shows or hides Dispatch from any app, has moved there from General. Click it to record a new one; Esc cancels and Delete turns it off. Your shortcut is unchanged.",
    ],
  },
  {
    version: "1.2.2",
    date: "November 25, 2026",
    notes: [
      "The cursor in a pane that isn't focused keeps the shape you chose in Cursor style (bar, block or underline) and stops blinking. Before, it always turned into a hollow block.",
    ],
  },
  {
    version: "1.2.1",
    date: "November 25, 2026",
    notes: [
      "In a split tab, a pane name shown as a notch or as just the name keeps the pane's own background when the pane isn't focused. Before, the strip behind the name turned see-through and showed a different color from the rest of the pane. A Bar still fades as its own strip.",
    ],
  },
  {
    version: "1.2.0",
    date: "November 25, 2026",
    notes: [
      "Pane names have a new look: a notch that hangs from the top edge of the pane, with rounded bottom corners, like the camera notch on a MacBook. It's the new default, sitting against the left edge.",
      "Choose how pane names look with Pane name style in Appearance: Notch, Bar (the full-width strip from before), or Clear (just the name, with no strip or border).",
      "Pane name position, also in Appearance, puts the name on the left, in the center, or on the right, for any of the three styles. A notch at an edge rounds only its inside corner; a centered notch rounds both.",
      "On Windows, the Dispatch installer is now signed by RazorKode LLC, so Windows names RazorKode LLC as the publisher instead of an unknown one. For a little while after a new release, SmartScreen may still say Windows protected your PC; click More info, then Run anyway.",
    ],
  },
  {
    version: "1.1.0",
    date: "November 25, 2026",
    notes: [
      "Save your layout from the new ⋮ menu in the tab bar, next to the version number. Save Layout… asks for a name and keeps every tab in the window, with its name and color, and its split panes, with their names, colors, sizes, and the folder each one is in, so they reopen right where you left them. Save up to 20, and save with an existing name to update that one.",
      "Your saved layouts are listed in the same menu, with a checkmark by the one the window is using. Pick one to switch the window to it: its tabs take the place of the ones open now. Click the pencil next to a layout to rename it, change whether it opens at launch, or delete it. Layouts are kept in your settings, so they show in the JSON tab and go along when you export them.",
      "Tick Open at launch when you save or edit a layout to have Dispatch start with it. Only the first window opens it; new windows and tabs still start with a single terminal.",
      "Open more windows with Command+N (Ctrl+Shift+N on Windows and Linux), or New Window in the Shell menu or the menu bar icon. Each window has its own tabs, and opens the size of the one in front, a little down and to the right of it.",
      "Shortcuts like Command+T, Command+W, and the splits act on the window in front. Closing a window, or its last tab, closes just that window and its shells. The last window still only hides when Dispatch keeps running in the background.",
      "The show-or-hide hotkey and the menu bar icon bring every window back at once, with the one you used last in front. A setting changed in one window applies to all of them.",
      "Dispatch checks for updates on its own: when it starts and every few hours after. When a new version is ready, the update icon appears in the tab bar and Dispatch asks whether to install it. Nothing installs without your OK.",
      "Dispatch usually stays running in the background, so when an update turns up while its window is hidden you get a system notification, and the install prompt opens the next time you bring Dispatch up.",
      "An update found while you're working doesn't interrupt you. The tab bar icon appears right away, and the prompt waits until you next come back to the window.",
      "Turn this off with Check for updates automatically in General settings. Check for Updates… in the menu still works either way.",
      "Later is remembered across restarts. Automatic checks wait a day before asking about that version again, while Check for Updates… always asks.",
    ],
  },
  {
    version: "1.0.0",
    date: "November 24, 2026",
    notes: [
      "Double-click a tab to rename it in place. Press Enter or click away to keep the name, or Escape to cancel. Clear the name to go back to the shell's title.",
      "Right-click a tab to rename it, give it a color, or close it. A colored tab gets a soft tint and a colored line along its bottom edge.",
      "Right-click a terminal to name the pane, give it a color, split it right or down, or close it. A named pane shows its name in a slim strip along its top (double-click the strip to rename it), and a tab that hasn't been renamed uses the name of its focused pane. A pane's color is a faint tint over its background that follows your window opacity.",
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
