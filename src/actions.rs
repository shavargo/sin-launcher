//! Process spawning and external links.
//!
//! Multiplayer join: `sin.exe +connect host:port`
//! (dedicated SiN servers use classic Q2-engine networking; no Steam connect).

use std::path::Path;
use std::process::{Command, Stdio};

/// Community hub (Steam group for SiN Gold multiplayer).
pub const COMMUNITY_URL: &str = "https://steamcommunity.com/groups/singoldmultiplayer";

/// Launch single-player (`sin.exe` with no extra args).
pub fn launch_single_player(game_dir: &Path) -> Result<(), String> {
	let exe = game_dir.join("sin.exe");
	if !exe.is_file() {
		return Err("sin.exe not found — open Setup to locate your SiN Gold install.".into());
	}
	spawn(&exe, game_dir, &[])
}

/// Join a dedicated server via `+connect`.
///
/// `connect_address` must be `ip:gameport` (default SiN hostport is 22450).
pub fn join_server(game_dir: &Path, connect_address: &str) -> Result<(), String> {
	let exe = game_dir.join("sin.exe");
	if !exe.is_file() {
		return Err("sin.exe not found — open Setup to locate your SiN Gold install.".into());
	}
	if connect_address.trim().is_empty() || !connect_address.contains(':') {
		return Err("Invalid server address.".into());
	}

	spawn(
		&exe,
		game_dir,
		&["+connect", connect_address.trim()],
	)
}

/// Open the community hub in the system browser.
pub fn open_community() -> Result<(), String> {
	open::that(COMMUNITY_URL).map_err(|e| format!("Could not open community page: {e}"))
}

/// Reveal the SiN folder in the file manager.
pub fn open_game_folder(game_dir: &Path) -> Result<(), String> {
	if !game_dir.is_dir() {
		return Err("Game folder does not exist.".into());
	}
	open::that(game_dir).map_err(|e| format!("Could not open folder: {e}"))
}

fn spawn(exe: &Path, cwd: &Path, args: &[&str]) -> Result<(), String> {
	Command::new(exe)
		.args(args)
		.current_dir(cwd)
		.stdin(Stdio::null())
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.spawn()
		.map(|_| ())
		.map_err(|e| format!("Failed to start {}: {e}", exe.display()))
}
