# SiN Launcher

Windows GUI launcher for **SiN Gold** (Steam / GOG): launch single-player and browse dedicated servers that do **not** appear on Steam’s master.

## Features (v0.1)

- **Play** — launch `sin.exe`
- **Browser** — ASE-style list from [sin-servers-mirror](https://github.com/shavargo/sin-servers-mirror) + live UDP status (GameSpy1 / Quake2); Join via `+connect`
- **Setup** — auto-detect or browse for the SiN Gold install folder

## Install

1. Download the latest release zip from [Releases](https://github.com/shavargo/sin-launcher/releases)
2. Extract `sin-launcher.exe` anywhere
3. Run it and set your SiN Gold folder under **Setup** if auto-detect misses it

Requires a Steam or GOG SiN Gold install (`sin.exe`). Default dedicated `hostport` is **22450**.

## Build

```bash
cargo run --release
```

Release binary: `target/release/sin-launcher.exe` (Windows).

## License

MIT
