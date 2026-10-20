//! Locate a SiN Gold install on disk.
//!
//! Resolution order: saved path → Steam libraries → GOG folders → known defaults.

use std::fs;
use std::path::{Path, PathBuf};

const STEAM_VDF_CANDIDATES: &[&str] = &[
	r"C:\Program Files (x86)\Steam\steamapps\libraryfolders.vdf",
	r"C:\Program Files\Steam\steamapps\libraryfolders.vdf",
];

/// Folder names under `steamapps/common` that may hold SiN Gold.
const STEAM_COMMON_NAMES: &[&str] = &["SiN 1", "SiN Gold", "SiN", "Sin Gold", "Sin"];

const GOG_SIN_CANDIDATES: &[&str] = &[
	r"C:\GOG Games\SiN Gold",
	r"C:\GOG Games\Sin Gold",
	r"C:\Program Files (x86)\GOG Galaxy\Games\SiN Gold",
	r"C:\Program Files (x86)\GOG Galaxy\Games\Sin Gold",
	r"C:\Program Files\GOG Galaxy\Games\SiN Gold",
	r"C:\Program Files\GOG Galaxy\Games\Sin Gold",
];

/// True if `dir` looks like a SiN Gold root (`sin.exe` present).
pub fn is_sin_dir(dir: &Path) -> bool {
	dir.join("sin.exe").is_file()
}

/// Resolve a usable SiN directory: saved → Steam → GOG → known defaults.
pub fn resolve_game_dir(saved: Option<&Path>) -> Option<PathBuf> {
	if let Some(path) = saved {
		if is_sin_dir(path) {
			return Some(path.to_path_buf());
		}
	}

	if let Some(found) = find_steam_sin() {
		return Some(found);
	}

	if let Some(found) = find_gog_sin() {
		return Some(found);
	}

	for fallback in GOG_SIN_CANDIDATES {
		let path = PathBuf::from(fallback);
		if is_sin_dir(&path) {
			return Some(path);
		}
	}

	None
}

pub fn find_gog_sin() -> Option<PathBuf> {
	for candidate in GOG_SIN_CANDIDATES {
		let path = PathBuf::from(candidate);
		if is_sin_dir(&path) {
			return Some(path);
		}
	}

	if let Some(home) = std::env::var_os("USERPROFILE") {
		for name in ["SiN Gold", "Sin Gold", "SiN", "Sin"] {
			let path = PathBuf::from(&home).join("GOG Games").join(name);
			if is_sin_dir(&path) {
				return Some(path);
			}
		}
	}

	None
}

pub fn find_steam_sin() -> Option<PathBuf> {
	for vdf in STEAM_VDF_CANDIDATES {
		let vdf_path = Path::new(vdf);
		if !vdf_path.is_file() {
			continue;
		}
		let Ok(text) = fs::read_to_string(vdf_path) else {
			continue;
		};
		for library_root in parse_library_paths(&text) {
			for name in STEAM_COMMON_NAMES {
				let candidate = library_root
					.join("steamapps")
					.join("common")
					.join(name);
				if is_sin_dir(&candidate) {
					return Some(candidate);
				}
				let alt = library_root.join("common").join(name);
				if is_sin_dir(&alt) {
					return Some(alt);
				}
			}
		}
	}
	None
}

fn parse_library_paths(vdf: &str) -> Vec<PathBuf> {
	let mut paths = Vec::new();
	for line in vdf.lines() {
		let trimmed = line.trim();
		if !trimmed.to_ascii_lowercase().contains("\"path\"") {
			continue;
		}
		let parts: Vec<&str> = trimmed.split('"').collect();
		if parts.len() >= 4 {
			let raw = parts[3].replace("\\\\", "\\");
			if !raw.is_empty() {
				paths.push(PathBuf::from(raw));
			}
		}
	}
	paths
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_library_paths() {
		let sample = r#"
"libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
	}
}
"#;
		let paths = parse_library_paths(sample);
		assert_eq!(paths.len(), 1);
		assert!(paths[0].to_string_lossy().contains("Steam"));
	}
}
