import { isMac, isWindows } from "./platform";

/* Recording and showing global hotkeys, from Lockshot's hotkeys.ts. */

const MODIFIER_CODES = new Set([
  "ShiftLeft", "ShiftRight", "ControlLeft", "ControlRight", "AltLeft", "AltRight", "MetaLeft", "MetaRight", "OSLeft", "OSRight",
]);

/* Keys that make sense on their own, without a modifier. */
const STANDALONE = /^(F\d{1,2}|Pause|ScrollLock|Insert)$/;

/** The key part of an accelerator, from KeyboardEvent.code. */
function keyFromCode(code: string) {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit\d$/.test(code)) return code.slice(5);
  return code;
}

export type RecordResult =
  | { kind: "pending"; preview: string }
  | { kind: "done"; accelerator: string }
  | { kind: "invalid"; message: string };

/**
 * Turn a key press into an accelerator the Rust side can register, such as
 * "Cmd+Backquote". Pressing only modifiers returns a live preview.
 */
export function acceleratorFromEvent(event: KeyboardEvent): RecordResult {
  const mods: string[] = [];
  if (event.ctrlKey) mods.push("Ctrl");
  if (event.altKey) mods.push("Alt");
  if (event.shiftKey) mods.push("Shift");
  if (event.metaKey) mods.push(isMac ? "Cmd" : "Super");

  if (MODIFIER_CODES.has(event.code) || !event.code) {
    return { kind: "pending", preview: mods.length ? formatAccelerator(mods.join("+")) : "" };
  }
  const key = keyFromCode(event.code);
  if (!mods.length && !STANDALONE.test(key)) {
    return { kind: "invalid", message: "Add a modifier like Cmd, Ctrl or Alt — single keys would fire while you type." };
  }
  if (mods.length === 1 && mods[0] === "Shift" && !STANDALONE.test(key)) {
    return { kind: "invalid", message: "Shift alone would fire while you type capitals. Add Cmd, Ctrl or Alt." };
  }
  return { kind: "done", accelerator: [...mods, key].join("+") };
}

const MAC_SYMBOLS: Record<string, string> = {
  ctrl: "⌃", control: "⌃", alt: "⌥", option: "⌥", shift: "⇧", cmd: "⌘", command: "⌘", super: "⌘", meta: "⌘",
  commandorcontrol: "⌘", cmdorctrl: "⌘",
};
const KEY_NAMES: Record<string, string> = {
  Space: "Space", Enter: "↩", Backspace: "⌫", Escape: "Esc", Tab: "⇥",
  ArrowUp: "↑", ArrowDown: "↓", ArrowLeft: "←", ArrowRight: "→", Minus: "-", Equal: "=",
  BracketLeft: "[", BracketRight: "]", Backslash: "\\", Semicolon: ";", Quote: "'", Comma: ",",
  Period: ".", Slash: "/", Backquote: "`",
};

/** The pieces of an accelerator as they should appear on key caps. */
export function acceleratorKeys(accelerator: string) {
  if (!accelerator) return [];
  const parts = accelerator.split("+").filter(Boolean);
  return parts.map((part, i) => {
    const lower = part.toLowerCase();
    const isModifier = i < parts.length - 1;
    if (isModifier && isMac) return MAC_SYMBOLS[lower] ?? part;
    if (isModifier) {
      if (lower === "super" || lower === "cmd" || lower === "meta" || lower === "command") return isWindows ? "Win" : "Super";
      if (lower === "commandorcontrol" || lower === "cmdorctrl") return "Ctrl";
      return part;
    }
    return KEY_NAMES[part] ?? part;
  });
}

export function formatAccelerator(accelerator: string) {
  const keys = acceleratorKeys(accelerator);
  return isMac ? keys.join("") : keys.join("+");
}
