//! SiN industrial teal / amber visual theme.

use egui::{
	Color32, FontFamily, FontId, Stroke, Style, TextStyle, Visuals,
};

const VOID: Color32 = Color32::from_rgb(8, 18, 22);

pub fn apply(ctx: &egui::Context) {
	let mut style = Style::default();
	let mut visuals = Visuals::dark();

	let panel = Color32::from_rgba_unmultiplied(14, 36, 42, 220);
	let raised = Color32::from_rgb(18, 48, 54);
	let teal = Color32::from_rgb(40, 160, 150);
	let teal_hi = Color32::from_rgb(64, 190, 178);
	let amber = Color32::from_rgb(210, 150, 60);
	let text = Color32::from_rgb(220, 235, 232);
	let muted = Color32::from_rgb(120, 150, 148);

	visuals.window_fill = VOID;
	visuals.panel_fill = panel;
	visuals.extreme_bg_color = VOID;
	visuals.faint_bg_color = raised;
	visuals.override_text_color = Some(text);
	visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, muted);
	visuals.widgets.inactive.bg_fill = raised;
	visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, text);
	visuals.widgets.hovered.bg_fill = Color32::from_rgb(28, 70, 78);
	visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, amber);
	visuals.widgets.active.bg_fill = teal;
	visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, text);
	visuals.selection.bg_fill = teal_hi;
	visuals.hyperlink_color = amber;
	visuals.warn_fg_color = amber;
	visuals.error_fg_color = Color32::from_rgb(210, 80, 70);

	style.visuals = visuals;
	style.spacing.item_spacing = egui::vec2(10.0, 8.0);
	style.spacing.button_padding = egui::vec2(14.0, 10.0);
	style.text_styles.insert(
		TextStyle::Small,
		FontId::new(14.0, FontFamily::Proportional),
	);
	style.text_styles.insert(
		TextStyle::Body,
		FontId::new(17.0, FontFamily::Proportional),
	);
	style.text_styles.insert(
		TextStyle::Button,
		FontId::new(17.0, FontFamily::Proportional),
	);
	style.text_styles.insert(
		TextStyle::Heading,
		FontId::new(24.0, FontFamily::Proportional),
	);
	style.text_styles.insert(
		TextStyle::Monospace,
		FontId::new(16.0, FontFamily::Monospace),
	);
	ctx.set_style(style);
}

pub fn paint_backdrop(ctx: &egui::Context) {
	let screen = ctx.screen_rect();
	let painter = ctx.layer_painter(egui::LayerId::background());
	painter.rect_filled(screen, 0.0, VOID);
	// Subtle diagonal hatch
	let amber = Color32::from_rgba_unmultiplied(210, 150, 60, 18);
	let teal = Color32::from_rgba_unmultiplied(40, 160, 150, 14);
	let mut x = -screen.height();
	while x < screen.width() + screen.height() {
		painter.line_segment(
			[
				egui::pos2(x, 0.0),
				egui::pos2(x + screen.height(), screen.height()),
			],
			Stroke::new(1.0_f32, if ((x as i32) / 40) % 2 == 0 { amber } else { teal }),
		);
		x += 28.0;
	}
}

pub fn amber() -> Color32 {
	Color32::from_rgb(210, 150, 60)
}

pub fn teal() -> Color32 {
	Color32::from_rgb(40, 160, 150)
}

pub fn ready_green() -> Color32 {
	Color32::from_rgb(72, 180, 120)
}

pub fn ase_ping_color(online: bool, ping_ms: Option<u32>) -> Color32 {
	if !online {
		return Color32::from_rgb(100, 120, 118);
	}
	match ping_ms.unwrap_or(999) {
		0..=79 => Color32::from_rgb(72, 200, 120),
		80..=149 => Color32::from_rgb(210, 190, 60),
		150..=249 => Color32::from_rgb(210, 120, 40),
		_ => Color32::from_rgb(200, 55, 45),
	}
}

pub fn ase_panel() -> Color32 {
	Color32::from_rgba_unmultiplied(10, 28, 32, 230)
}
