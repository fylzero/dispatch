import type { IDecoration, IMarker, Terminal } from "@xterm/xterm";
import * as api from "./api";
import { isMac } from "./platform";
import { settings } from "./settings";

/*
 * Suggestions: as you type at a prompt, what Tab would most likely complete
 * (a file, folder, or command; see suggest.rs) shows in grey after the
 * cursor. Only the page draws it, over empty cells, so the shell never sees
 * it until it is accepted, and then it is sent as if typed.
 *
 * The shell marks where the command starts (OSC 6973 "input" from the
 * Dispatch prompt, "line" from PowerShell and cmd) and reports its folder and
 * PATH. The command is read off the screen from that mark to the cursor, and
 * a suggestion shows only while nothing is drawn after the cursor, so a shell
 * that suggests on its own (fish, zsh-autosuggestions, PowerShell's
 * predictions) keeps its own and this one stays out of the way.
 */

interface Shown {
  /** The command line it was suggested for. */
  line: string;
  /** What it adds, as it would be typed. */
  text: string;
  marker: IMarker;
  decoration: IDecoration;
  row: number;
  col: number;
}

export interface Suggestions {
  /** The command starts at the cursor (column `x` of its row). */
  start(x: number): void;
  /** A command is running; nothing to suggest until the next prompt. */
  stop(): void;
  setCwd(cwd: string): void;
  setPath(path: string): void;
  /** Reads the command line again, soon. Cheap; call on every render. */
  schedule(): void;
  /** Accept keys: false when the key filled in the suggestion, undefined to let it through. */
  onKey(event: KeyboardEvent): false | undefined;
  dispose(): void;
}

const WORD = /^[^\p{L}\p{N}_]*[\p{L}\p{N}_]*/u;

export function createSuggestions(
  term: Terminal,
  session: () => string | undefined,
  send: (data: string) => void,
): Suggestions {
  let begin: { marker: IMarker; x: number } | null = null;
  let cwd: string | null = null;
  let path: string | null = null;
  let shown: Shown | null = null;
  /* Esc hides the suggestion until the command changes. */
  let dismissed: string | null = null;
  let frame = 0;
  let asked = 0;
  /* Answers for this prompt's lines; a new prompt may be in a new folder. */
  const answers = new Map<string, string | null>();

  /** The command typed so far, or undefined when the cursor isn't at its end on a prompt. */
  function readLine(): string | undefined {
    const buffer = term.buffer.active;
    if (!begin || begin.marker.line < 0 || buffer.type !== "normal" || term.modes.mouseTrackingMode !== "none") return;
    const startRow = begin.marker.line;
    const cursorRow = buffer.baseY + buffer.cursorY;
    if (cursorRow < startRow) return;
    let text = "";
    for (let row = startRow; row <= cursorRow; row++) {
      const line = buffer.getLine(row);
      /* A command on more than one line (Shift+Enter) is left alone. */
      if (!line || (row > startRow && !line.isWrapped)) return;
      const from = row === startRow ? begin.x : 0;
      const to = row === cursorRow ? buffer.cursorX : term.cols;
      if (to < from) return;
      text += line.translateToString(false, from, to);
    }
    /* Anything after the cursor: it isn't at the end, or the shell is suggesting itself. */
    if (buffer.getLine(cursorRow)?.translateToString(true, buffer.cursorX).trim()) return;
    if (buffer.getLine(cursorRow + 1)?.isWrapped && buffer.getLine(cursorRow + 1)?.translateToString(true).trim()) return;
    return text;
  }

  function hide() {
    if (!shown) return;
    shown.decoration.dispose();
    shown.marker.dispose();
    shown = null;
  }

  function render(el: HTMLElement, text: string, width: number) {
    const style = el.style;
    style.setProperty("--cell", `${parseFloat(style.width) / width}px`);
    style.lineHeight = style.height;
    style.fontFamily = term.options.fontFamily ?? "";
    style.fontSize = `${term.options.fontSize ?? 14}px`;
    style.color = term.options.theme?.brightBlack ?? "";
    if (el.dataset.text === text) return;
    el.dataset.text = text;
    el.classList.add("suggestion");
    el.replaceChildren(
      ...[...text].slice(0, width).map((char) => {
        const span = document.createElement("span");
        span.textContent = char;
        return span;
      }),
    );
  }

  function show(line: string, text: string | null) {
    if (!text || line === dismissed || !settings.value.suggestions) {
      hide();
      return;
    }
    const buffer = term.buffer.active;
    const row = buffer.baseY + buffer.cursorY;
    const col = buffer.cursorX;
    if (shown && shown.line === line && shown.text === text && shown.row === row && shown.col === col) return;
    hide();
    /* What fits on the cursor's row; the rest is still filled in. */
    const width = Math.min([...text].length, term.cols - col);
    if (width <= 0) return;
    const marker = term.registerMarker(0);
    const decoration = term.registerDecoration({ marker, x: col, width });
    if (!decoration) {
      marker.dispose();
      return;
    }
    decoration.onRender((el) => render(el, text, width));
    shown = { line, text, marker, decoration, row, col };
    /* The answer comes after xterm drew the row; draw it again so the suggestion shows now. */
    term.refresh(buffer.cursorY, buffer.cursorY);
  }

  function update() {
    frame = 0;
    const line = readLine();
    if (line === undefined || !line.trim() || /\s$/.test(line) || !settings.value.suggestions || term.hasSelection()) {
      hide();
      return;
    }
    if (line !== dismissed) dismissed = null;
    if (shown?.line === line) {
      show(line, shown.text);
      return;
    }
    /* Typing what the suggestion says keeps the rest of it up while the new answer comes. */
    const typed = shown && line.startsWith(shown.line) ? line.slice(shown.line.length) : null;
    if (shown && typed && shown.text.startsWith(typed) && shown.text.length > typed.length) {
      show(line, shown.text.slice(typed.length));
    } else if (!answers.has(line)) {
      hide();
    }
    if (answers.has(line)) {
      show(line, answers.get(line) ?? null);
      return;
    }
    const id = session();
    if (!id) return;
    const ask = ++asked;
    api
      .suggestCompletion(id, line, cwd, path)
      .catch(() => null)
      .then((answer) => {
        if (answers.size > 200) answers.clear();
        answers.set(line, answer);
        if (ask === asked && readLine() === line) show(line, answer);
      });
  }

  function schedule() {
    /* After xterm draws (onRender), not on the next animation frame, which a hidden window may hold back. */
    if (!frame) frame = window.setTimeout(update, 0);
  }

  /*
   * Returning false stops xterm handling the key, but not the browser: Tab
   * would move focus out of the terminal, leaving it deaf to typing. So the
   * key's default is cancelled too.
   */
  function take(event: KeyboardEvent, text: string): false {
    event.preventDefault();
    event.stopPropagation();
    hide();
    send(text);
    return false;
  }

  function onKey(event: KeyboardEvent): false | undefined {
    if (event.type !== "keydown") return;
    const plain = !event.ctrlKey && !event.altKey && !event.metaKey && !event.shiftKey;
    if (event.key === "Enter" && plain) {
      /* The command runs; nothing to suggest until the next prompt marks itself. */
      hide();
      begin?.marker.dispose();
      begin = null;
      return;
    }
    if (!shown) return;
    if (readLine() !== shown.line) {
      hide();
      return;
    }
    const keys = settings.value.suggestionAcceptKey;
    const byArrow = keys !== "tab";
    const byTab = keys !== "right";
    const all = shown.text;
    if (byArrow && plain && (event.key === "ArrowRight" || event.key === "End")) return take(event, all);
    /* Command+Right goes to the end of the line on macOS, so it takes it all too. */
    const commandRight = isMac && event.metaKey && !event.altKey && !event.ctrlKey && !event.shiftKey;
    if (byArrow && commandRight && event.key === "ArrowRight") return take(event, all);
    if (byTab && plain && event.key === "Tab") return take(event, all);
    /* Option+Right (Ctrl+Right or Alt+Right elsewhere) takes one word, or one folder of a path. */
    const wordKey = isMac
      ? event.altKey && !event.metaKey && !event.ctrlKey && !event.shiftKey
      : (event.ctrlKey || event.altKey) && !event.metaKey && !event.shiftKey;
    if (wordKey && event.key === "ArrowRight") {
      const word = WORD.exec(all)?.[0] || [...all][0];
      return take(event, word);
    }
    if (event.key === "Escape" && plain) {
      dismissed = shown.line;
      hide();
    }
    return;
  }

  return {
    start(x) {
      begin?.marker.dispose();
      begin = { marker: term.registerMarker(0), x };
      dismissed = null;
      answers.clear();
      hide();
    },
    stop() {
      begin?.marker.dispose();
      begin = null;
      hide();
    },
    setCwd(value) {
      if (value !== cwd) answers.clear();
      cwd = value;
    },
    setPath(value) {
      if (value !== path) answers.clear();
      path = value;
    },
    schedule,
    onKey,
    dispose() {
      window.clearTimeout(frame);
      hide();
      begin?.marker.dispose();
      begin = null;
    },
  };
}
