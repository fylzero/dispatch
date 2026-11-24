#!/usr/bin/env python3
"""Save and restore Dispatch's webview localStorage around a capture run.

Dispatch's webview keeps the theme in localStorage under
~/Library/WebKit/com.dispatch.app. Setting HOME for the capture does not
redirect it, and the capture runs dark, so capture.sh saves every row first
and puts them back exactly as they were afterwards.

Usage:
  localstorage.py backup <file>    save every row
  localstorage.py restore <file>   put the saved rows back
"""
import base64, glob, json, os, sqlite3, sys

PATTERN = os.path.expanduser(
    "~/Library/WebKit/com.dispatch.app/WebsiteData/Default/*/*/LocalStorage/localstorage.sqlite3")


def backup(out):
    saved = {}
    for path in glob.glob(PATTERN):
        con = sqlite3.connect(path)
        rows = con.execute("SELECT key, value FROM ItemTable").fetchall()
        saved[path] = [[k, base64.b64encode(v).decode()] for k, v in rows]
        con.close()
    with open(out, "w") as f:
        json.dump(saved, f)


def restore(src):
    with open(src) as f:
        saved = json.load(f)
    for path in glob.glob(PATTERN):
        con = sqlite3.connect(path)
        con.execute("DELETE FROM ItemTable")
        con.executemany("INSERT INTO ItemTable (key, value) VALUES (?, ?)",
                        [(k, base64.b64decode(v)) for k, v in saved.get(path, [])])
        con.commit()
        con.close()


def main():
    if len(sys.argv) != 3 or sys.argv[1] not in ("backup", "restore"):
        sys.exit(__doc__)
    (backup if sys.argv[1] == "backup" else restore)(sys.argv[2])


if __name__ == "__main__":
    main()
