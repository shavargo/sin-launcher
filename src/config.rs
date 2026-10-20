//! Persistent launcher settings (`launcher.toml` beside the executable).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LauncherConfig {
	/// Absolute path to the SiN Gold install folder (`sin.exe` parent).
	pub game_path: Option<String>,
	/// Last selected sidebar tab (`play`, `browser`, `setup`).
	pub last_tab: Option<String>,
}

impl LauncherConfig {
	pub fn config_path() -> PathBuf {
		std::env::current_exe()
			.ok()
			.and_then(|p| p.parent().map(|d| d.join("launcher.toml")))
			.unwrap_or_else(|| PathBuf::from("launcher.toml"))
	}

	pub fn load() -> Self {
		Self::load_from(&Self::config_path())
	}

	pub fn load_from(path: &Path) -> Self {
		let Ok(raw) = fs::read_to_string(path) else {
			return Self::default();
		};
		toml::from_str(&raw).unwrap_or_default()
	}

	pub fn save(&self) -> Result<(), String> {
		let path = Self::config_path();
		let raw = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
		if let Some(parent) = path.parent() {
			fs::create_dir_all(parent).map_err(|e| e.to_string())?;
		}
		fs::write(&path, raw).map_err(|e| e.to_string())
	}

	pub fn game_dir(&self) -> Option<PathBuf> {
		self.game_path.as_ref().map(PathBuf::from)
	}

	pub fn set_game_dir(&mut self, dir: PathBuf) {
		self.game_path = Some(dir.to_string_lossy().to_string());
	}
}
