# Linux releases

The Release workflow's `linux` job builds the Linux packages after the `macos` job
succeeds and uploads them to the same GitHub release. It needs no secrets beyond the
`TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` the macOS job
already uses. Linux has no code signing or notarization step.

## What gets published

The job runs once on `ubuntu-22.04` (x64) and once on `ubuntu-22.04-arm` (ARM64).
Building on 22.04 keeps the binaries working on distros with glibc 2.35 or newer.

| Asset | Contents |
| --- | --- |
| `Dispatch-linux-x64.AppImage`, `Dispatch-linux-arm64.AppImage` | Self-contained app for any distro |
| `Dispatch-linux-x64.deb`, `Dispatch-linux-arm64.deb` | Debian and Ubuntu package |
| `Dispatch-linux-x64.rpm`, `Dispatch-linux-arm64.rpm` | Fedora, RHEL, and openSUSE package |
| `*.sig` | Updater signatures for each package |

The names stay the same from release to release, so the README links to
`releases/latest/download/…` keep working.

## Updates

Each architecture adds `linux-<arch>-appimage`, `linux-<arch>-deb`, and
`linux-<arch>-rpm` entries to the release's `latest.json`, and the updater picks the
one matching how Dispatch was installed. The AppImage replaces itself in place. The
`.deb` and `.rpm` are installed with `pkexec`, so the user is asked for their
password.

The jobs run one after another (`needs: macos` and `max-parallel: 1`) because each
one downloads `latest.json`, adds its entries, and uploads it again. Running them at
the same time would let one job overwrite the entries another just added.

If the `macos` job fails, the Linux job doesn't run. Re-run the failed jobs from the
Actions page once the problem is fixed.

## Verification

After the first Linux release, install the `.deb` on Ubuntu and the AppImage on
Fedora or another non-Debian distro, then confirm:

- Dispatch opens and the first tab starts your login shell.
- New tabs, split panes, and ⌘-click (Ctrl-click) links work.
- **Help → Check for Updates…** reports that Dispatch is up to date.
