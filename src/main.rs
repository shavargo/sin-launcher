//! SiN Launcher — entry point.
//!
//! Boots an egui/eframe native window. On Windows release builds we hide the
//! console (`windows_subsystem = "windows"`).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod icon_png;
mod config;
mod detect;
mod query;
mod servers;
mod theme;
mod ui;

use eframe::egui;
use ui::SinLauncherApp;

pub const WIN_MIN: [f32; 2] = [1000.0, 580.0];
pub const WIN_MAX: [f32; 2] = [1360.0, 860.0];
pub const WIN_DEFAULT: [f32; 2] = WIN_MAX;

fn main() -> eframe::Result<()> {
	let options = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default()
			.with_inner_size(WIN_DEFAULT)
			.with_min_inner_size(WIN_MIN)
			.with_max_inner_size(WIN_MAX)
			.with_maximize_button(false)
			.with_title("SiN Launcher")
			.with_icon(load_app_icon()),
		..Default::default()
	};

	eframe::run_native(
		"SiN Launcher",
		options,
		Box::new(|cc| Ok(Box::new(SinLauncherApp::new(cc)))),
	)
}

fn load_app_icon() -> egui::IconData {
	let image = image::load_from_memory(crate::icon_png::ICON_PNG)
		.expect("assets/icon.png")
		.into_rgba8();
	let (width, height) = image.dimensions();
	egui::IconData {
		rgba: image.into_raw(),
		width,
		height,
	}
}
