# SiN Launcher — Design

## Summary

Windows-native (egui/eframe) launcher for *SiN Gold*: single-player launch and ASE-style dedicated server browser. Steam’s master does not list typical dedicated hosts, so discovery uses a GitHub mirror + UDP status.

## UI

| Tab | Role |
|-----|------|
| Play | Launch `sin.exe` |
| Browser | Seed from [sin-servers-mirror](https://github.com/shavargo/sin-servers-mirror), live UDP status, Join via `+connect` |
| Setup | Locate / verify SiN Gold install |

**Look:** Industrial teal + amber on dark void (not Kingpin noir).

## Non-goals (v0.1)

- Dominatrix sourceport
- Mods / Controls / Patches catalogs
- Steamworks API / Steam connect strings
- Hosting wizard

## Architecture

| Module | Responsibility |
|--------|----------------|
| `detect` | Steam/GOG path discovery (`sin.exe`) |
| `actions` | Launch SP / join / community / open folder |
| `servers` / `query` | Mirror list cache + GS1/Q2 UDP status |
| `ui` / `theme` | egui shell |

## Data (next to the exe)

- `launcher.toml` — game path, last tab
- `cache/servers.txt` + `cache/servers.sha` — mirrored address list

No accounts, no telemetry, no GitHub credentials.
