# token-maxxing 🔥

A tiny always-on-top widget that shows live AI token usage across your local
CLI clients. Reads each client's local session logs — no proxy, no client
changes, no API keys.

![status: TOKEN MAXXING](https://img.shields.io/badge/status-TOKEN%20MAXXING-orange)

## What it shows

- **Tokens today** (big, resets at UTC midnight)
- **All-time total** across every session log on disk
- **Per-model breakdown** (Claude Sonnet, GPT-5.5, …)
- A funny status that escalates as you burn tokens: `warming up 🌱` →
  `cooking 🍳` → `TOKEN MAXING 🚀` → `going nuclear 🤯` → `TOKEN MAXXING 🔥`

## Supported clients

| Client      | Source                                 | Status |
| ----------- | -------------------------------------- | ------ |
| Claude Code | `~/.claude/projects/**/*.jsonl`        | ✅     |
| Codex CLI   | `~/.codex/sessions/**/rollout-*.jsonl` | ✅     |

Adding a client = one more glob + parse function in `src-tauri/src/tailer.rs`.
Closed clients (Cursor, ChatGPT desktop) keep no local usage log; they'd need a
local API proxy.

## Install (end users)

Grab a prebuilt package from the [Releases](../../releases) page:

| OS          | File                          |
| ----------- | ----------------------------- |
| Linux (deb) | `token-maxxing_*_amd64.deb`   |
| Linux (rpm) | `token-maxxing-*.x86_64.rpm`  |
| Linux (any) | `token-maxxing_*.AppImage`    |
| macOS       | `token-maxxing_*.dmg`         |
| Windows     | `token-maxxing_*-setup.exe`   |

```bash
# Fedora / RHEL
sudo dnf install ./token-maxxing-*.x86_64.rpm

# Debian / Ubuntu
sudo dpkg -i ./token-maxxing_*_amd64.deb

# AppImage (no install)
chmod +x token-maxxing_*.AppImage && ./token-maxxing_*.AppImage
```

macOS: open the `.dmg`, drag to Applications. Windows: run the `-setup.exe`.

> **Note:** the widget only has data for clients *you* run on *your* machine —
> it reads your own local logs.

## Build from source

```bash
# 1. Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 2. Tauri system deps (Fedora)
sudo dnf install -y webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel gcc gcc-c++ make
#   (Debian/Ubuntu: libwebkit2gtk-4.1-dev build-essential libssl-dev \
#    libayatana-appindicator3-dev librsvg2-dev)

# 3. Run / build
npm install
npm run dev      # live dev window
npm run build    # bundles into src-tauri/target/release/bundle/
```

## Notes

- **Always-on-top on GNOME/Wayland:** the app forces itself onto XWayland
  (`GDK_BACKEND=x11`) because Mutter ignores always-on-top for native Wayland
  clients. This makes it float above every window and follow you across
  workspaces.
- Frameless + transparent; drag by the header bar.
- Icons in `src-tauri/icons/` are flat-color placeholders. Replace with
  `npm run tauri icon path/to/logo.png`.
