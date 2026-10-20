//! Server list seed + shared server model for the ASE-style browser.
//!
//! Mirror: [sin-servers-mirror](https://github.com/shavargo/sin-servers-mirror)
//! (Contents API SHA cache beside the exe — same pattern as Kingpin).

use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

pub const SERVERS_PAGE_URL: &str = "https://github.com/shavargo/sin-servers-mirror";

const SERVERS_API_URL: &str =
	"https://api.github.com/repos/shavargo/sin-servers-mirror/contents/servers.txt?ref=main";
const SERVERS_JSDELIVR_URL: &str =
	"https://cdn.jsdelivr.net/gh/shavargo/sin-servers-mirror@main/servers.txt";
const SERVERS_RAW_URL: &str =
	"https://raw.githubusercontent.com/shavargo/sin-servers-mirror/main/servers.txt";

const USER_AGENT: &str = "sin-launcher/0.1";

#[derive(Debug, Deserialize)]
struct GithubContents {
	sha: String,
	encoding: Option<String>,
	content: Option<String>,
	download_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerKind {
	Sin,
	Other,
}

#[derive(Debug, Clone)]
pub struct PlayerRow {
	pub name: String,
	pub score: i32,
	pub ping: u32,
}

#[derive(Debug, Clone)]
pub struct GameServer {
	/// Seed address from the mirror (`ip:port`) — often the game/connect port.
	pub address: String,
	/// Address used for `+connect` (may differ if status reports `hostport`).
	pub connect_address: String,
	pub name: String,
	pub map: String,
	pub players: u32,
	pub max_players: u32,
	pub ping_ms: Option<u32>,
	#[allow(dead_code)]
	pub kind: ServerKind,
	pub online: bool,
	/// Display string (`gamename` / `gametype`).
	pub game: String,
	pub rules: BTreeMap<String, String>,
	pub player_rows: Vec<PlayerRow>,
}

impl GameServer {
	pub fn game_label(&self) -> &str {
		if !self.game.is_empty() {
			return self.game.as_str();
		}
		if let Some(name) = self.rules.get("gamename") {
			let name = name.trim();
			if !name.is_empty() {
				return name;
			}
		}
		if let Some(name) = self.rules.get("gametype") {
			let name = name.trim();
			if !name.is_empty() {
				return name;
			}
		}
		""
	}

	pub fn players_label(&self) -> String {
		if self.max_players > 0 {
			format!("{}/{}", self.players, self.max_players)
		} else {
			format!("{}", self.players)
		}
	}

	pub fn ping_label(&self) -> String {
		if !self.online {
			return "timeout".into();
		}
		match self.ping_ms {
			Some(ms) => format!("{ms}"),
			None => "—".into(),
		}
	}

	pub fn is_full(&self) -> bool {
		self.max_players > 0 && self.players >= self.max_players
	}

	pub fn is_empty(&self) -> bool {
		self.players == 0
	}
}

fn cache_dir() -> PathBuf {
	std::env::current_exe()
		.ok()
		.and_then(|p| p.parent().map(|d| d.join("cache")))
		.unwrap_or_else(|| PathBuf::from("cache"))
}

fn cache_paths() -> (PathBuf, PathBuf) {
	let dir = cache_dir();
	(dir.join("servers.txt"), dir.join("servers.sha"))
}

fn read_cached_sha() -> Option<String> {
	let (_, sha_path) = cache_paths();
	fs::read_to_string(sha_path)
		.ok()
		.map(|s| s.trim().to_string())
		.filter(|s| !s.is_empty())
}

fn read_cached_body() -> Option<String> {
	let (body_path, _) = cache_paths();
	fs::read_to_string(body_path)
		.ok()
		.filter(|s| !s.trim().is_empty())
}

fn write_cache(body: &str, sha: &str) -> Result<(), String> {
	let dir = cache_dir();
	fs::create_dir_all(&dir).map_err(|e| format!("cache dir: {e}"))?;
	let (body_path, sha_path) = cache_paths();
	fs::write(&body_path, body).map_err(|e| format!("cache write: {e}"))?;
	fs::write(&sha_path, sha.trim()).map_err(|e| format!("sha write: {e}"))?;
	Ok(())
}

fn http_get_text(url: &str) -> Result<String, String> {
	ureq::get(url)
		.set("User-Agent", USER_AGENT)
		.set("Accept", "application/vnd.github+json, text/plain, */*")
		.call()
		.map_err(|e| format!("{url}: {e}"))?
		.into_string()
		.map_err(|e| format!("{url}: {e}"))
}

fn decode_github_contents(meta: &GithubContents) -> Result<String, String> {
	if let (Some(encoding), Some(content)) = (&meta.encoding, &meta.content) {
		if encoding == "base64" {
			let cleaned: String = content.chars().filter(|c| !c.is_whitespace()).collect();
			let bytes = decode_base64(&cleaned)?;
			return String::from_utf8(bytes).map_err(|e| format!("utf-8: {e}"));
		}
	}
	if let Some(url) = &meta.download_url {
		return http_get_text(url);
	}
	Err("GitHub contents response had no usable body.".into())
}

fn decode_base64(input: &str) -> Result<Vec<u8>, String> {
	fn val(c: u8) -> Result<u8, String> {
		Ok(match c {
			b'A'..=b'Z' => c - b'A',
			b'a'..=b'z' => c - b'a' + 26,
			b'0'..=b'9' => c - b'0' + 52,
			b'+' => 62,
			b'/' => 63,
			_ => return Err(format!("invalid base64 byte {c}")),
		})
	}
	let bytes = input.as_bytes();
	let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
	let mut i = 0;
	while i + 3 < bytes.len() {
		let (a, b, c, d) = (bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]);
		i += 4;
		if a == b'=' {
			break;
		}
		let av = val(a)?;
		let bv = val(b)?;
		out.push((av << 2) | (bv >> 4));
		if c != b'=' {
			let cv = val(c)?;
			out.push(((bv & 0xf) << 4) | (cv >> 2));
			if d != b'=' {
				let dv = val(d)?;
				out.push(((cv & 0x3) << 6) | dv);
			}
		}
	}
	Ok(out)
}

fn fetch_via_github_api() -> Result<(String, String), String> {
	let raw = http_get_text(SERVERS_API_URL)?;
	let meta: GithubContents =
		serde_json::from_str(&raw).map_err(|e| format!("GitHub API JSON: {e}"))?;
	if meta.sha.is_empty() {
		return Err("GitHub API returned empty sha.".into());
	}
	let body = decode_github_contents(&meta)?;
	if body.trim().is_empty() {
		return Err("GitHub API returned empty servers.txt.".into());
	}
	Ok((meta.sha, body))
}

fn fetch_via_plain_url(url: &str) -> Result<String, String> {
	let body = http_get_text(url)?;
	if body.trim().is_empty() {
		return Err(format!("{url}: empty body"));
	}
	Ok(body)
}

pub fn load_servers_txt() -> Result<String, String> {
	let cached_sha = read_cached_sha();
	let cached_body = read_cached_body();

	match fetch_via_github_api() {
		Ok((sha, body)) => {
			if cached_sha.as_deref() == Some(sha.as_str()) {
				if let Some(body) = cached_body {
					return Ok(body);
				}
			}
			let _ = write_cache(&body, &sha);
			Ok(body)
		}
		Err(api_err) => {
			for url in [SERVERS_JSDELIVR_URL, SERVERS_RAW_URL] {
				if let Ok(body) = fetch_via_plain_url(url) {
					let tag = cached_sha
						.clone()
						.unwrap_or_else(|| format!("fallback-{}", body.len()));
					let _ = write_cache(&body, &tag);
					return Ok(body);
				}
			}
			if let Some(body) = cached_body {
				return Ok(body);
			}
			Err(format!("Could not fetch server list: {api_err}"))
		}
	}
}

fn parse_address_list(text: &str) -> Vec<String> {
	let mut addrs = Vec::new();
	for line in text.lines() {
		let line = line.trim();
		if line.is_empty() || line.starts_with('#') {
			continue;
		}
		if let Some(addr) = line.split_whitespace().next() {
			if addr.contains(':') && !addrs.iter().any(|a| a == addr) {
				addrs.push(addr.to_string());
			}
		}
	}
	addrs
}

pub fn fetch_address_list() -> Result<Vec<String>, String> {
	let text = load_servers_txt()?;
	let addrs = parse_address_list(&text);
	if addrs.is_empty() {
		return Err("Server list was empty.".into());
	}
	Ok(addrs)
}

pub fn sync_list_cache() {
	let _ = load_servers_txt();
}

pub fn classify_address(address: &str) -> ServerKind {
	let port = address
		.rsplit_once(':')
		.and_then(|(_, p)| p.parse::<u16>().ok())
		.unwrap_or(0);
	if (22400..=22500).contains(&port) || port == 22450 {
		ServerKind::Sin
	} else if port > 0 {
		ServerKind::Sin
	} else {
		ServerKind::Other
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn decodes_github_base64() {
		let out = decode_base64("aGkK").unwrap();
		assert_eq!(out, b"hi\n");
	}

	#[test]
	fn parses_list() {
		let text = "# comment\n95.179.197.192:22450\n\n";
		assert_eq!(parse_address_list(text), vec!["95.179.197.192:22450"]);
	}
}
