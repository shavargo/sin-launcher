//! Main egui shell — Play, Browser, Setup.

use crate::actions;
use crate::config::LauncherConfig;
use crate::detect::{self, is_sin_dir};
use crate::query;
use crate::servers::{self, GameServer};
use crate::theme;
use eframe::egui::{self, Align, Color32, Layout, RichText};
use egui_extras::{Column, TableBuilder};
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
	Play,
	Browser,
	Setup,
}

impl Tab {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Play => "play",
			Self::Browser => "browser",
			Self::Setup => "setup",
		}
	}

	pub fn from_str(s: &str) -> Self {
		match s {
			"browser" => Self::Browser,
			"setup" => Self::Setup,
			_ => Self::Play,
		}
	}

	pub fn label(self) -> &'static str {
		match self {
			Self::Play => "PLAY",
			Self::Browser => "BROWSER",
			Self::Setup => "SETUP",
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortCol {
	Ping,
	Players,
	Name,
	Map,
	Game,
	Address,
}

pub struct SinLauncherApp {
	config: LauncherConfig,
	tab: Tab,
	status_message: String,
	status_is_error: bool,
	busy: bool,
	servers: Vec<GameServer>,
	servers_loaded: bool,
	selected_address: Option<String>,
	sort_col: SortCol,
	sort_asc: bool,
	hide_empty: bool,
	hide_full: bool,
	online_only: bool,
	auto_refresh: bool,
	server_search: String,
	last_refresh: Option<Instant>,
	refresh_rx: Option<mpsc::Receiver<Result<Vec<GameServer>, String>>>,
}

impl SinLauncherApp {
	pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
		theme::apply(&cc.egui_ctx);
		egui_extras::install_image_loaders(&cc.egui_ctx);

		let mut config = LauncherConfig::load();
		let saved = config.game_dir();
		if let Some(dir) = detect::resolve_game_dir(saved.as_deref()) {
			config.set_game_dir(dir);
			let _ = config.save();
		}

		let tab = config
			.last_tab
			.as_deref()
			.map(Tab::from_str)
			.unwrap_or(Tab::Play);

		let mut app = Self {
			config,
			tab,
			status_message: String::new(),
			status_is_error: false,
			busy: false,
			servers: Vec::new(),
			servers_loaded: false,
			selected_address: None,
			sort_col: SortCol::Ping,
			sort_asc: true,
			hide_empty: false,
			hide_full: false,
			online_only: true,
			auto_refresh: true,
			server_search: String::new(),
			last_refresh: None,
			refresh_rx: None,
		};
		std::thread::spawn(|| crate::servers::sync_list_cache());
		if app.tab == Tab::Browser {
			app.start_server_refresh();
		}
		app
	}

	fn set_ok(&mut self, msg: impl Into<String>) {
		self.status_message = msg.into();
		self.status_is_error = false;
	}

	fn set_err(&mut self, msg: impl Into<String>) {
		self.status_message = msg.into();
		self.status_is_error = true;
	}

	fn persist_tab(&mut self) {
		self.config.last_tab = Some(self.tab.as_str().into());
		let _ = self.config.save();
	}

	fn start_server_refresh(&mut self) {
		if self.busy {
			return;
		}
		self.busy = true;
		self.set_ok("Refreshing servers…");
		let (tx, rx) = mpsc::channel();
		self.refresh_rx = Some(rx);
		query::refresh_live_async(tx);
	}

	fn poll_refresh(&mut self) {
		let Some(rx) = &self.refresh_rx else {
			return;
		};
		match rx.try_recv() {
			Ok(Ok(list)) => {
				let n_online = list.iter().filter(|s| s.online).count();
				self.servers = list;
				self.servers_loaded = true;
				self.busy = false;
				self.refresh_rx = None;
				self.last_refresh = Some(Instant::now());
				self.set_ok(format!(
					"Refreshed — {n_online} online / {} listed",
					self.servers.len()
				));
			}
			Ok(Err(e)) => {
				self.busy = false;
				self.refresh_rx = None;
				self.set_err(e);
			}
			Err(mpsc::TryRecvError::Empty) => {}
			Err(mpsc::TryRecvError::Disconnected) => {
				self.busy = false;
				self.refresh_rx = None;
				self.set_err("Server refresh failed (channel closed).");
			}
		}
	}

	fn filtered_sorted_servers(&self) -> Vec<&GameServer> {
		let q = self.server_search.trim().to_ascii_lowercase();
		let mut list: Vec<&GameServer> = self
			.servers
			.iter()
			.filter(|s| {
				if self.online_only && !s.online {
					return false;
				}
				if self.hide_empty && s.is_empty() {
					return false;
				}
				if self.hide_full && s.is_full() {
					return false;
				}
				if q.is_empty() {
					return true;
				}
				s.name.to_ascii_lowercase().contains(&q)
					|| s.map.to_ascii_lowercase().contains(&q)
					|| s.address.to_ascii_lowercase().contains(&q)
					|| s.game_label().to_ascii_lowercase().contains(&q)
			})
			.collect();

		list.sort_by(|a, b| {
			let ord = match self.sort_col {
				SortCol::Ping => a
					.ping_ms
					.unwrap_or(u32::MAX)
					.cmp(&b.ping_ms.unwrap_or(u32::MAX)),
				SortCol::Players => a.players.cmp(&b.players),
				SortCol::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
				SortCol::Map => a.map.to_lowercase().cmp(&b.map.to_lowercase()),
				SortCol::Game => a
					.game_label()
					.to_lowercase()
					.cmp(&b.game_label().to_lowercase()),
				SortCol::Address => a.address.cmp(&b.address),
			};
			if self.sort_asc {
				ord
			} else {
				ord.reverse()
			}
		});
		list
	}

	fn join_selected(&mut self) {
		let Some(address) = self.selected_address.clone() else {
			self.set_err("Select a server first.");
			return;
		};
		let server = self.servers.iter().find(|s| s.address == address);
		let name = server
			.map(|s| s.name.clone())
			.unwrap_or_else(|| address.clone());
		let connect = server
			.map(|s| s.connect_address.as_str())
			.unwrap_or(address.as_str());
		let Some(dir) = self.config.game_dir() else {
			self.set_err("No game folder — open Setup.");
			self.tab = Tab::Setup;
			return;
		};
		match actions::join_server(&dir, connect) {
			Ok(()) => self.set_ok(format!("Joining {name} ({connect})…")),
			Err(e) => {
				self.set_err(e);
				self.tab = Tab::Setup;
			}
		}
	}

	fn launch_sp(&mut self) {
		let Some(dir) = self.config.game_dir() else {
			self.set_err("No game folder — open Setup.");
			self.tab = Tab::Setup;
			return;
		};
		match actions::launch_single_player(&dir) {
			Ok(()) => self.set_ok("Launching SiN Gold…"),
			Err(e) => {
				self.set_err(e);
				self.tab = Tab::Setup;
			}
		}
	}

	fn browse_for_game(&mut self) {
		let picked = rfd::FileDialog::new()
			.set_title("Select SiN Gold folder (contains sin.exe)")
			.pick_folder();
		if let Some(path) = picked {
			if is_sin_dir(&path) {
				self.config.set_game_dir(path);
				let _ = self.config.save();
				self.set_ok("Game folder set.");
			} else {
				self.set_err("That folder has no sin.exe.");
			}
		}
	}
}

impl eframe::App for SinLauncherApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		theme::paint_backdrop(ctx);
		self.poll_refresh();

		// Soft size clamp
		let screen = ctx.input(|i| i.screen_rect());
		let w = screen.width().clamp(crate::WIN_MIN[0], crate::WIN_MAX[0]);
		let h = screen.height().clamp(crate::WIN_MIN[1], crate::WIN_MAX[1]);
		if (screen.width() - w).abs() > 1.0 || (screen.height() - h).abs() > 1.0 {
			ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(w, h)));
		}

		if self.tab == Tab::Browser && self.auto_refresh && !self.busy {
			let due = self
				.last_refresh
				.is_none_or(|t| t.elapsed() > Duration::from_secs(30));
			if due && self.servers_loaded {
				self.start_server_refresh();
			}
		}

		egui::TopBottomPanel::top("header").show(ctx, |ui| {
			ui.add_space(6.0);
			ui.horizontal(|ui| {
				ui.heading(RichText::new("SiN").color(theme::amber()).strong());
				ui.label(RichText::new("LAUNCHER").color(theme::teal()));
				ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
					if ui.button("Community").clicked() {
						if let Err(e) = actions::open_community() {
							self.set_err(e);
						}
					}
					if let Some(dir) = self.config.game_dir() {
						ui.label(
							RichText::new(dir.display().to_string())
								.small()
								.color(Color32::from_rgb(120, 150, 148)),
						);
					} else {
						ui.label(
							RichText::new("No install selected")
								.small()
								.color(Color32::from_rgb(210, 80, 70)),
						);
					}
				});
			});
			ui.add_space(4.0);
		});

		egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
			ui.horizontal(|ui| {
				let ready = self.config.game_dir().is_some();
				ui.label(
					RichText::new(if ready { "READY" } else { "SETUP NEEDED" })
						.strong()
						.color(if ready {
							theme::ready_green()
						} else {
							theme::amber()
						}),
				);
				ui.separator();
				if !self.status_message.is_empty() {
					ui.label(
						RichText::new(&self.status_message).color(if self.status_is_error {
							Color32::from_rgb(210, 80, 70)
						} else {
							Color32::from_rgb(180, 200, 198)
						}),
					);
				}
			});
		});

		egui::SidePanel::left("nav")
			.exact_width(140.0)
			.show(ctx, |ui| {
				ui.add_space(8.0);
				for tab in [Tab::Play, Tab::Browser, Tab::Setup] {
					let selected = self.tab == tab;
					let text = RichText::new(tab.label()).strong();
					if ui
						.selectable_label(selected, text)
						.clicked()
					{
						self.tab = tab;
						self.persist_tab();
						if tab == Tab::Browser && !self.servers_loaded && !self.busy {
							self.start_server_refresh();
						}
					}
				}
			});

		egui::CentralPanel::default().show(ctx, |ui| match self.tab {
			Tab::Play => self.ui_play(ui),
			Tab::Browser => self.ui_browser(ui),
			Tab::Setup => self.ui_setup(ui),
		});

		if self.busy {
			ctx.request_repaint_after(Duration::from_millis(100));
		}
	}
}

impl SinLauncherApp {
	fn ui_play(&mut self, ui: &mut egui::Ui) {
		ui.add_space(20.0);
		ui.heading(RichText::new("Play").color(theme::amber()));
		ui.label("Launch SiN Gold single-player, or open the dedicated Browser to Join.");
		ui.add_space(16.0);
		ui.horizontal(|ui| {
			if ui
				.add_sized([180.0, 44.0], egui::Button::new("Launch SiN Gold"))
				.clicked()
			{
				self.launch_sp();
			}
			if ui
				.add_sized([180.0, 44.0], egui::Button::new("Open Browser"))
				.clicked()
			{
				self.tab = Tab::Browser;
				self.persist_tab();
				if !self.servers_loaded && !self.busy {
					self.start_server_refresh();
				}
			}
		});
		ui.add_space(24.0);
		ui.label(
			RichText::new(
				"Dedicated servers use classic networking (default port 22450) and usually \
do not appear in Steam’s server browser. Use this launcher’s Browser tab to Join.",
			)
			.weak(),
		);
	}

	fn ui_setup(&mut self, ui: &mut egui::Ui) {
		ui.add_space(12.0);
		ui.heading(RichText::new("Setup").color(theme::amber()));
		ui.label("Point the launcher at your Steam or GOG SiN Gold folder (`sin.exe`).");
		ui.add_space(12.0);

		if let Some(dir) = self.config.game_dir() {
			ui.label(format!("Current: {}", dir.display()));
			ui.horizontal(|ui| {
				if ui.button("Open folder").clicked() {
					if let Err(e) = actions::open_game_folder(&dir) {
						self.set_err(e);
					}
				}
				if ui.button("Clear").clicked() {
					self.config.game_path = None;
					let _ = self.config.save();
					self.set_ok("Cleared game folder.");
				}
			});
		} else {
			ui.label(RichText::new("No folder set.").color(Color32::from_rgb(210, 80, 70)));
		}

		ui.add_space(12.0);
		ui.horizontal(|ui| {
			if ui.button("Browse…").clicked() {
				self.browse_for_game();
			}
			if ui.button("Auto-detect").clicked() {
				if let Some(dir) = detect::resolve_game_dir(None) {
					self.config.set_game_dir(dir.clone());
					let _ = self.config.save();
					self.set_ok(format!("Found {}", dir.display()));
				} else {
					self.set_err("Could not find SiN Gold — browse manually.");
				}
			}
		});
	}

	fn ui_browser(&mut self, ui: &mut egui::Ui) {
		ui.horizontal(|ui| {
			if ui
				.add_enabled(!self.busy, egui::Button::new("Refresh"))
				.clicked()
			{
				self.start_server_refresh();
			}
			if ui
				.add_enabled(self.selected_address.is_some(), egui::Button::new("Join"))
				.clicked()
			{
				self.join_selected();
			}
			if ui.link("Web list").clicked() {
				let _ = open::that(servers::SERVERS_PAGE_URL);
			}
			ui.checkbox(&mut self.auto_refresh, "Auto");
			ui.checkbox(&mut self.online_only, "Online");
			ui.checkbox(&mut self.hide_empty, "Hide empty");
			ui.checkbox(&mut self.hide_full, "Hide full");
			ui.label("Search:");
			ui.add(
				egui::TextEdit::singleline(&mut self.server_search)
					.desired_width(160.0)
					.hint_text("name / map / ip"),
			);
		});
		ui.add_space(6.0);

		let list = self.filtered_sorted_servers();
		let row_height = 28.0;
		let mut select_addr: Option<String> = None;
		let mut join_addr: Option<String> = None;
		let mut sort_click: Option<SortCol> = None;

		egui::Frame::NONE
			.fill(theme::ase_panel())
			.inner_margin(8.0)
			.show(ui, |ui| {
				TableBuilder::new(ui)
					.striped(true)
					.sense(egui::Sense::click())
					.cell_layout(Layout::left_to_right(Align::Center))
					.column(Column::initial(70.0).at_least(50.0))
					.column(Column::initial(70.0).at_least(50.0))
					.column(Column::remainder().at_least(160.0))
					.column(Column::initial(120.0).at_least(80.0))
					.column(Column::initial(110.0).at_least(70.0))
					.column(Column::initial(160.0).at_least(100.0))
					.header(24.0, |mut header| {
						for (col, label) in [
							(SortCol::Ping, "Ping"),
							(SortCol::Players, "Players"),
							(SortCol::Name, "Name"),
							(SortCol::Map, "Map"),
							(SortCol::Game, "Game"),
							(SortCol::Address, "Address"),
						] {
							header.col(|ui| {
								let mark = if self.sort_col == col {
									if self.sort_asc {
										" ▲"
									} else {
										" ▼"
									}
								} else {
									""
								};
								if ui
									.button(RichText::new(format!("{label}{mark}")).strong())
									.clicked()
								{
									sort_click = Some(col);
								}
							});
						}
					})
					.body(|mut body| {
						for server in &list {
							let addr = server.address.clone();
							let selected = self.selected_address.as_deref() == Some(addr.as_str());
							body.row(row_height, |mut row| {
								row.set_selected(selected);
								row.col(|ui| {
									ui.label(
										RichText::new(server.ping_label()).color(
											theme::ase_ping_color(server.online, server.ping_ms),
										),
									);
								});
								row.col(|ui| {
									ui.label(server.players_label());
								});
								row.col(|ui| {
									ui.label(&server.name);
								});
								row.col(|ui| {
									ui.label(if server.map.is_empty() {
										"—"
									} else {
										&server.map
									});
								});
								row.col(|ui| {
									let g = server.game_label();
									ui.label(if g.is_empty() { "—" } else { g });
								});
								row.col(|ui| {
									ui.label(
										RichText::new(&server.address)
											.monospace()
											.color(Color32::from_rgb(140, 170, 168)),
									);
								});
								let resp = row.response();
								if resp.clicked() {
									select_addr = Some(addr.clone());
								}
								if resp.double_clicked() {
									join_addr = Some(addr);
								}
							});
						}
					});
			});

		if let Some(col) = sort_click {
			if self.sort_col == col {
				self.sort_asc = !self.sort_asc;
			} else {
				self.sort_col = col;
				self.sort_asc = true;
			}
		}
		if let Some(addr) = select_addr {
			self.selected_address = Some(addr);
		}
		if let Some(addr) = join_addr {
			self.selected_address = Some(addr);
			self.join_selected();
		}

		ui.add_space(8.0);
		if let Some(addr) = &self.selected_address {
			if let Some(server) = self.servers.iter().find(|s| &s.address == addr) {
				ui.group(|ui| {
					ui.label(RichText::new(&server.name).strong());
					ui.label(format!(
						"Connect: {}   Map: {}   Game: {}",
						server.connect_address,
						if server.map.is_empty() {
							"—"
						} else {
							&server.map
						},
						{
							let g = server.game_label();
							if g.is_empty() {
								"—"
							} else {
								g
							}
						}
					));
					if !server.player_rows.is_empty() {
						ui.label("Players:");
						for p in &server.player_rows {
							ui.label(format!("  {}  score {}  ping {}", p.name, p.score, p.ping));
						}
					}
				});
			}
		}
	}
}
