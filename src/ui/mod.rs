//! Shared look & feel: fonts, palette, riveted panels and chunky pixel widgets.

pub mod cockpit;
pub mod hangar;

use std::sync::Arc;

use bevy::prelude::*;
use bevy_egui::egui::{
    self, Align2, Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2,
};

use crate::art::SpriteBank;
use crate::campaign::Maker;

pub const DESIGN_W: f32 = 1600.0;
pub const DESIGN_H: f32 = 900.0;

pub const BG: Color32 = Color32::from_rgb(9, 10, 13);
pub const PANEL: Color32 = Color32::from_rgb(20, 23, 28);
pub const PANEL_DARK: Color32 = Color32::from_rgb(12, 14, 17);
pub const EDGE_HI: Color32 = Color32::from_rgb(58, 64, 72);
pub const EDGE_LO: Color32 = Color32::from_rgb(5, 6, 8);
pub const TEXT: Color32 = Color32::from_rgb(206, 212, 200);
pub const DIM: Color32 = Color32::from_rgb(112, 120, 128);
pub const AMBER: Color32 = Color32::from_rgb(232, 168, 56);
pub const GREEN: Color32 = Color32::from_rgb(126, 240, 138);
pub const GREEN_DIM: Color32 = Color32::from_rgb(36, 84, 46);
pub const RED: Color32 = Color32::from_rgb(232, 72, 58);
pub const CYAN: Color32 = Color32::from_rgb(95, 211, 230);
pub const PURPLE: Color32 = Color32::from_rgb(178, 110, 240);
pub const YELLOW: Color32 = Color32::from_rgb(242, 200, 74);

pub fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

pub fn health_color(f: f32) -> Color32 {
    if f <= 0.0 {
        Color32::from_rgb(70, 30, 30)
    } else if f < 0.35 {
        RED
    } else if f < 0.7 {
        AMBER
    } else {
        GREEN
    }
}

pub fn pix(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("pix".into()))
}

pub fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

/// The pixel-art sprite bank and baked backdrops (egui textures are created lazily).
#[derive(Resource)]
pub struct Art {
    pub sprites: SpriteBank,
    pub hull: Vec<u8>,
    pub pano: Option<egui::TextureHandle>,
    pub hangar: Option<egui::TextureHandle>,
}

impl Default for Art {
    fn default() -> Self {
        Self { sprites: SpriteBank::new(), hull: crate::art::hull_map(), pano: None, hangar: None }
    }
}

impl Art {
    pub fn ensure_textures(&mut self, ctx: &egui::Context) {
        if self.pano.is_none() {
            let opts = egui::TextureOptions {
                magnification: egui::TextureFilter::Nearest,
                minification: egui::TextureFilter::Nearest,
                wrap_mode: egui::TextureWrapMode::Repeat,
                mipmap_mode: None,
            };
            self.pano = Some(ctx.load_texture("city_pano", crate::art::city_panorama(), opts));
            self.hangar = Some(ctx.load_texture("hangar", crate::art::hangar_backdrop(), egui::TextureOptions::NEAREST));
        }
    }
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "vt323".into(),
        Arc::new(egui::FontData::from_static(include_bytes!("../../assets/fonts/VT323-Regular.ttf"))),
    );
    fonts.font_data.insert(
        "silk".into(),
        Arc::new(egui::FontData::from_static(include_bytes!("../../assets/fonts/Silkscreen-Regular.ttf"))),
    );
    fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "vt323".into());
    fonts.families.entry(FontFamily::Monospace).or_default().insert(0, "vt323".into());
    fonts.families.insert(FontFamily::Name("pix".into()), vec!["silk".into(), "vt323".into()]);
    ctx.set_fonts(fonts);

    ctx.global_style_mut(|style| {
        use egui::TextStyle::*;
        style.text_styles = [
            (Small, FontId::new(16.0, FontFamily::Proportional)),
            (Body, FontId::new(20.0, FontFamily::Proportional)),
            (Button, FontId::new(20.0, FontFamily::Proportional)),
            (Heading, FontId::new(28.0, FontFamily::Proportional)),
            (Monospace, FontId::new(20.0, FontFamily::Monospace)),
        ]
        .into();
        style.visuals = egui::Visuals::dark();
        style.visuals.override_text_color = Some(TEXT);
        style.visuals.panel_fill = BG;
        style.visuals.window_fill = PANEL;
        style.spacing.item_spacing = Vec2::new(6.0, 4.0);
        style.spacing.scroll.bar_width = 6.0;
    });
}

/// Paint a single line of text with the painter.
pub fn text(p: &egui::Painter, pos: Pos2, align: Align2, s: impl ToString, font: FontId, color: Color32) -> Rect {
    p.text(pos, align, s.to_string(), font, color)
}

/// Riveted cockpit panel with a stamped title plate; returns the inner content rect.
pub fn panel_frame(ui: &Ui, rect: Rect, title: &str, maker: Option<Maker>) -> Rect {
    let p = ui.painter();
    let accent = maker.map(|m| rgb(m.accent())).unwrap_or(AMBER);
    p.rect_filled(rect, 0.0, PANEL);
    // Bevel
    p.line_segment([rect.left_top(), rect.right_top()], Stroke::new(2.0, EDGE_HI));
    p.line_segment([rect.left_top(), rect.left_bottom()], Stroke::new(2.0, EDGE_HI));
    p.line_segment([rect.left_bottom(), rect.right_bottom()], Stroke::new(2.0, EDGE_LO));
    p.line_segment([rect.right_top(), rect.right_bottom()], Stroke::new(2.0, EDGE_LO));
    // Hatching on the header strip
    let header = Rect::from_min_size(rect.min + Vec2::new(2.0, 2.0), Vec2::new(rect.width() - 4.0, 20.0));
    p.rect_filled(header, 0.0, PANEL_DARK);
    // Title plate
    let galley_w = title.len() as f32 * 8.6 + 16.0;
    let plate = Rect::from_min_size(header.min + Vec2::new(14.0, 2.0), Vec2::new(galley_w, 16.0));
    p.rect_filled(plate, 0.0, accent);
    p.rect_filled(Rect::from_min_size(plate.min + Vec2::new(0.0, 14.0), Vec2::new(plate.width(), 2.0)), 0.0, accent.gamma_multiply(0.6));
    text(p, plate.left_center() + Vec2::new(8.0, 0.0), Align2::LEFT_CENTER, title, pix(11.0), Color32::from_rgb(16, 14, 12));
    if let Some(m) = maker {
        text(p, header.right_center() - Vec2::new(16.0, 0.0), Align2::RIGHT_CENTER, m.name(), pix(9.0), accent.gamma_multiply(0.8));
    }
    // Rivets
    for c in [rect.left_top() + Vec2::new(5.0, 5.0), rect.right_top() + Vec2::new(-8.0, 5.0), rect.left_bottom() + Vec2::new(5.0, -8.0), rect.right_bottom() + Vec2::new(-8.0, -8.0)] {
        let r = Rect::from_min_size(c, Vec2::splat(3.0));
        p.rect_filled(r, 0.0, EDGE_HI);
        p.rect_filled(Rect::from_min_size(c + Vec2::new(1.0, 1.0), Vec2::splat(2.0)), 0.0, EDGE_LO);
    }
    Rect::from_min_max(rect.min + Vec2::new(10.0, 28.0), rect.max - Vec2::new(10.0, 10.0))
}

/// Run `add` inside a child ui confined to `rect`.
pub fn child<R>(ui: &mut Ui, rect: Rect, add: impl FnOnce(&mut Ui) -> R) -> R {
    let mut c = ui.new_child(UiBuilder::new().max_rect(rect).layout(egui::Layout::top_down(egui::Align::Min)));
    c.set_clip_rect(rect.intersect(ui.clip_rect()));
    add(&mut c)
}

/// Chunky bevelled push button. `lit` keeps it glowing (toggle state).
pub fn pbutton(ui: &mut Ui, rect: Rect, label: &str, color: Color32, enabled: bool, lit: bool) -> Response {
    let id = ui.id().with(("pbtn", label, rect.min.x as i32, rect.min.y as i32));
    let resp = ui.interact(rect, id, if enabled { Sense::click() } else { Sense::hover() });
    let p = ui.painter();
    let down = resp.is_pointer_button_down_on();
    let hovered = resp.hovered() && enabled;
    let base = if !enabled {
        Color32::from_rgb(34, 36, 40)
    } else if lit {
        color
    } else {
        color.gamma_multiply(0.32)
    };
    let face = if hovered && !down { base.gamma_multiply(1.25) } else { base };
    p.rect_filled(rect, 0.0, EDGE_LO);
    let inner = rect.shrink(2.0);
    p.rect_filled(inner, 0.0, face);
    let (hi, lo) = if down { (EDGE_LO, Color32::from_white_alpha(60)) } else { (Color32::from_white_alpha(70), Color32::from_black_alpha(120)) };
    p.rect_filled(Rect::from_min_size(inner.min, Vec2::new(inner.width(), 2.0)), 0.0, hi);
    p.rect_filled(Rect::from_min_size(inner.min, Vec2::new(2.0, inner.height())), 0.0, hi);
    p.rect_filled(Rect::from_min_max(Pos2::new(inner.min.x, inner.max.y - 2.0), inner.max), 0.0, lo);
    p.rect_filled(Rect::from_min_max(Pos2::new(inner.max.x - 2.0, inner.min.y), inner.max), 0.0, lo);
    let tc = if !enabled {
        DIM
    } else if lit {
        Color32::from_rgb(14, 12, 10)
    } else {
        color.gamma_multiply(1.0)
    };
    let off = if down { Vec2::new(1.0, 1.0) } else { Vec2::ZERO };
    let base: f32 = if rect.height() > 30.0 { 13.0 } else { 10.0 };
    // Silkscreen is wide: shrink long labels to fit the button.
    let size = base.min((rect.width() - 12.0) / (label.chars().count() as f32 * 0.86)).max(7.0);
    text(p, rect.center() + off, Align2::CENTER_CENTER, label, pix(size), tc);
    resp
}

/// Horizontal LED segment bar.
pub fn seg_bar(p: &egui::Painter, rect: Rect, frac: f32, segs: usize, on: Color32) {
    p.rect_filled(rect, 0.0, EDGE_LO);
    let gap = 2.0;
    let w = (rect.width() - gap * (segs as f32 + 1.0)) / segs as f32;
    let lit = (frac.clamp(0.0, 1.0) * segs as f32).round() as usize;
    for i in 0..segs {
        let r = Rect::from_min_size(
            Pos2::new(rect.min.x + gap + i as f32 * (w + gap), rect.min.y + gap),
            Vec2::new(w, rect.height() - gap * 2.0),
        );
        let c = if i < lit { on } else { on.gamma_multiply(0.12) };
        p.rect_filled(r, 0.0, c);
    }
}

/// A blinking indicator lamp with label.
pub fn lamp(p: &egui::Painter, rect: Rect, label: &str, on: bool, color: Color32, t: f64) {
    let blink = on && (t * 3.0).fract() < 0.6;
    p.rect_filled(rect, 0.0, EDGE_LO);
    let inner = rect.shrink(2.0);
    p.rect_filled(inner, 0.0, if blink { color } else { color.gamma_multiply(0.14) });
    p.rect_stroke(inner, CornerRadius::ZERO, Stroke::new(1.0, Color32::from_black_alpha(140)), StrokeKind::Inside);
    text(p, rect.center(), Align2::CENTER_CENTER, label, pix(9.0), if blink { Color32::from_rgb(16, 12, 10) } else { color.gamma_multiply(0.6) });
}

/// CRT scanlines over a rect.
pub fn scanlines(p: &egui::Painter, rect: Rect, alpha: u8) {
    let mut y = rect.min.y;
    while y < rect.max.y {
        p.rect_filled(Rect::from_min_size(Pos2::new(rect.min.x, y), Vec2::new(rect.width(), 1.0)), 0.0, Color32::from_black_alpha(alpha));
        y += 3.0;
    }
}

pub fn fmt_clock(t: f32) -> String {
    let s = t.max(0.0) as u32;
    format!("{:02}:{:02}", s / 60, s % 60)
}
