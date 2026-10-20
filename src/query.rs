//! Live SiN dedicated queries over UDP.
//!
//! Tries GameSpy1 `\status\` first (common for SiN), then Quake2
//! `\xff\xff\xff\xffstatus` on the game port. Also probes a few query-port
//! offsets when the first attempt times out.

use crate::servers::{GameServer, PlayerRow, ServerKind};
use std::collections::BTreeMap;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const OOB: [u8; 4] = [0xff, 0xff, 0xff, 0xff];
const QUERY_TIMEOUT: Duration = Duration::from_millis(1200);

/// Query-port offsets relative to the seed game port (GameDig-style).
const QUERY_PORT_OFFSETS: &[i32] = &[0, 1, -1, 10, -10];

pub fn refresh_live_async(tx: mpsc::Sender<Result<Vec<GameServer>, String>>) {
	thread::spawn(move || {
		let result = (|| {
			let seeds = crate::servers::fetch_address_list()?;
			Ok(query_many(&seeds))
		})();
		let _ = tx.send(result);
	});
}

pub fn query_many(addresses: &[String]) -> Vec<GameServer> {
	let (tx, rx) = mpsc::channel();
	let timeout = QUERY_TIMEOUT;

	thread::scope(|scope| {
		for addr in addresses {
			let addr = addr.clone();
			let tx = tx.clone();
			scope.spawn(move || {
				let result = query_one(&addr, timeout);
				let _ = tx.send(result);
			});
		}
		drop(tx);
	});

	let mut out: Vec<GameServer> = rx.into_iter().collect();
	out.sort_by(|a, b| {
		b.online
			.cmp(&a.online)
			.then_with(|| {
				a.ping_ms
					.unwrap_or(u32::MAX)
					.cmp(&b.ping_ms.unwrap_or(u32::MAX))
			})
			.then_with(|| b.players.cmp(&a.players))
			.then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
	});
	out
}

pub fn query_one(address: &str, timeout: Duration) -> GameServer {
	let kind = crate::servers::classify_address(address);
	let fallback = offline_stub(address, kind);

	let Ok(base) = parse_host_port(address) else {
		return fallback;
	};

	let probes = build_probes(base);
	for (host, port, packet) in probes {
		if let Some(server) = try_probe(&host, port, &packet, address, kind, timeout) {
			return server;
		}
	}
	fallback
}

fn offline_stub(address: &str, kind: ServerKind) -> GameServer {
	GameServer {
		address: address.to_string(),
		connect_address: address.to_string(),
		name: address.to_string(),
		map: String::new(),
		players: 0,
		max_players: 0,
		ping_ms: None,
		kind,
		online: false,
		game: String::new(),
		rules: BTreeMap::new(),
		player_rows: Vec::new(),
	}
}

fn parse_host_port(address: &str) -> Result<(String, u16), ()> {
	let (host, port_s) = address.rsplit_once(':').ok_or(())?;
	let port: u16 = port_s.parse().map_err(|_| ())?;
	Ok((host.to_string(), port))
}

fn build_probes(base: (String, u16)) -> Vec<(String, u16, Vec<u8>)> {
	let (host, game_port) = base;
	let mut out = Vec::new();
	let mut seen = std::collections::HashSet::new();

	let mut q2 = Vec::with_capacity(16);
	q2.extend_from_slice(&OOB);
	q2.extend_from_slice(b"status");
	let gs = b"\\status\\".to_vec();

	for &off in QUERY_PORT_OFFSETS {
		let p = i32::from(game_port) + off;
		if !(1..=65535).contains(&p) {
			continue;
		}
		let port = p as u16;
		for packet in [gs.clone(), q2.clone()] {
			let key = (port, packet.clone());
			if seen.insert(key.clone()) {
				out.push((host.clone(), port, packet));
			}
		}
	}
	out
}

fn try_probe(
	host: &str,
	port: u16,
	packet: &[u8],
	seed_address: &str,
	kind: ServerKind,
	timeout: Duration,
) -> Option<GameServer> {
	let Ok(sock) = UdpSocket::bind("0.0.0.0:0") else {
		return None;
	};
	let _ = sock.set_read_timeout(Some(timeout));
	let _ = sock.set_write_timeout(Some(timeout));

	let dest_s = format!("{host}:{port}");
	let Ok(mut addrs) = dest_s.to_socket_addrs() else {
		return None;
	};
	let dest: SocketAddr = addrs.next()?;

	let t0 = Instant::now();
	if sock.send_to(packet, dest).is_err() {
		return None;
	}
	let mut buf = [0u8; 16_384];
	let Ok((n, _)) = sock.recv_from(&mut buf) else {
		return None;
	};
	let ping_ms = t0.elapsed().as_millis().min(u128::from(u32::MAX)) as u32;
	parse_status_response(seed_address, kind, ping_ms, &buf[..n])
}

/// Parse GameSpy1 or Quake2 status payloads into a [`GameServer`].
pub fn parse_status_response(
	seed_address: &str,
	kind: ServerKind,
	ping_ms: u32,
	raw: &[u8],
) -> Option<GameServer> {
	let payload = strip_oob(raw);
	let text = String::from_utf8_lossy(payload);
	let text = text.trim_start_matches('\0').trim();

	let body = if let Some(rest) = text.strip_prefix("print\n") {
		rest
	} else if let Some(rest) = text.strip_prefix("print\r\n") {
		rest
	} else if text.contains('\\') {
		text
	} else {
		return None;
	};

	// GameSpy split packets may include queryid / final — keep info pairs.
	let mut lines = body.lines();
	let info_line = lines.next().unwrap_or("").trim();
	if !info_line.contains('\\') {
		return None;
	}
	let rules = parse_infostring(info_line);

	let mut player_rows = Vec::new();
	for line in lines {
		let line = line.trim();
		if line.is_empty() || line.starts_with('\\') {
			continue;
		}
		if let Some(row) = parse_player_line(line) {
			player_rows.push(row);
		} else if let Some(row) = parse_gamespy_player_line(line) {
			player_rows.push(row);
		}
	}

	let name = rules
		.get("hostname")
		.or_else(|| rules.get("sv_hostname"))
		.cloned()
		.unwrap_or_else(|| seed_address.to_string());
	let map = rules
		.get("mapname")
		.or_else(|| rules.get("map"))
		.cloned()
		.unwrap_or_default();
	let max_players = rules
		.get("maxplayers")
		.or_else(|| rules.get("maxclients"))
		.and_then(|v| v.parse().ok())
		.unwrap_or(0);
	let players = if !player_rows.is_empty() {
		player_rows.len() as u32
	} else {
		rules
			.get("numplayers")
			.or_else(|| rules.get("clients"))
			.or_else(|| rules.get("players"))
			.and_then(|v| v.parse().ok())
			.unwrap_or(0)
	};
	let game = rules
		.get("gamename")
		.or_else(|| rules.get("gametype"))
		.cloned()
		.unwrap_or_default();

	let connect_address = connect_address_from_rules(seed_address, &rules);

	Some(GameServer {
		address: seed_address.to_string(),
		connect_address,
		name,
		map,
		players,
		max_players,
		ping_ms: Some(ping_ms),
		kind,
		online: true,
		game,
		rules,
		player_rows,
	})
}

/// Prefer `hostport` on the same host as the seed for `+connect`.
fn connect_address_from_rules(seed_address: &str, rules: &BTreeMap<String, String>) -> String {
	let host = seed_address
		.rsplit_once(':')
		.map(|(h, _)| h)
		.unwrap_or(seed_address);
	if let Some(port) = rules
		.get("hostport")
		.or_else(|| rules.get("host_port"))
		.and_then(|p| p.parse::<u16>().ok())
	{
		return format!("{host}:{port}");
	}
	seed_address.to_string()
}

fn strip_oob(raw: &[u8]) -> &[u8] {
	if raw.len() >= 4 && raw[..4] == OOB {
		&raw[4..]
	} else {
		raw
	}
}

fn parse_infostring(s: &str) -> BTreeMap<String, String> {
	let mut map = BTreeMap::new();
	let s = s.trim().trim_start_matches('\\');
	if s.is_empty() {
		return map;
	}
	let parts: Vec<&str> = s.split('\\').collect();
	let mut i = 0;
	while i + 1 < parts.len() {
		let key = parts[i].to_ascii_lowercase();
		let val = parts[i + 1].to_string();
		if !key.is_empty() && key != "queryid" && key != "final" {
			map.insert(key, val);
		}
		i += 2;
	}
	map
}

fn parse_player_line(line: &str) -> Option<PlayerRow> {
	let mut parts = line.splitn(3, char::is_whitespace);
	let score: i32 = parts.next()?.parse().ok()?;
	let ping: u32 = parts.next()?.parse().ok()?;
	let rest = parts.next()?.trim();
	let name = if rest.starts_with('"') && rest.ends_with('"') && rest.len() >= 2 {
		rest[1..rest.len() - 1].to_string()
	} else {
		rest.trim_matches('"').to_string()
	};
	Some(PlayerRow { name, score, ping })
}

fn parse_gamespy_player_line(line: &str) -> Option<PlayerRow> {
	// Some GS responses interleave player_ keys in the infostring instead.
	let _ = line;
	None
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_q2_status() {
		let raw = b"\xff\xff\xff\xffprint\n\\hostname\\SiN DM\\mapname\\spry\\maxclients\\16\\hostport\\22450\\gamename\\sin\n10 45 \"Blade\"\n";
		let s = parse_status_response("1.2.3.4:22450", ServerKind::Sin, 40, raw).unwrap();
		assert!(s.online);
		assert_eq!(s.name, "SiN DM");
		assert_eq!(s.map, "spry");
		assert_eq!(s.connect_address, "1.2.3.4:22450");
		assert_eq!(s.players, 1);
	}

	#[test]
	fn parses_gamespy_status() {
		let raw = br"\hostname\Night City\mapname\sincity\numplayers\2\maxplayers\8\hostport\22450\gamename\sin\final\";
		let s = parse_status_response("9.9.9.9:22451", ServerKind::Sin, 55, raw).unwrap();
		assert_eq!(s.connect_address, "9.9.9.9:22450");
		assert_eq!(s.players, 2);
		assert_eq!(s.max_players, 8);
	}
}
