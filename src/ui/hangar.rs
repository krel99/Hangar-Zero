//! Between operations: the ruined hangar, five salvage choices, and the slow
//! transformation of the machine.

use bevy::prelude::*;
use bevy_egui::{
    EguiContexts,
    egui::{self, Align2, Color32, CornerRadius, Pos2, Rect, Stroke, StrokeKind, Vec2},
};

use super::cockpit::root_ui;
use super::*;
use crate::art;
use crate::campaign::{Campaign, PartId, part};
use crate::rng::hash2;
use crate::sim::Section;
use crate::{Flow, FlowRequest};

#[derive(Resource, Default)]
pub struct HangarState {
    pub chosen: Option<PartId>,
}

pub fn hangar_ui(
    mut contexts: EguiContexts,
    mut art: ResMut<Art>,
    mut campaign: ResMut<Campaign>,
    mut hs: ResMut<HangarState>,
    mut flow: ResMut<Flow>,
) -> Result {
    let ctx = contexts.ctx_mut()?.clone();
    art.ensure_textures(&ctx);
    let mut ui = root_ui(&ctx, "hangar");
    let r = ui.max_rect();
    let t = ctx.input(|i| i.time);
    let p = ui.painter().clone();
    p.rect_filled(r, 0.0, BG);

    // Backdrop, scaled to cover
    let s = (r.width() / art::HANGAR_W as f32).max(r.height() / art::HANGAR_H as f32);
    let bg = Rect::from_center_size(r.center(), Vec2::new(art::HANGAR_W as f32 * s, art::HANGAR_H as f32 * s));
    if let Some(tex) = &art.hangar {
        p.image(tex.id(), bg, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::from_gray(200));
    }

    // The mech on its maintenance cradle, each section painted in its maker's colours.
    let cradle_y = r.max.y - 372.0;
    let ms = ((cradle_y - r.min.y - 110.0) / art::HULL_H as f32).floor();
    let mech_rect = Rect::from_min_size(
        Pos2::new(r.center().x - art::HULL_W as f32 * ms / 2.0, cradle_y - art::HULL_H as f32 * ms),
        Vec2::new(art::HULL_W as f32 * ms, art::HULL_H as f32 * ms),
    );
    paint_cradle(&p, mech_rect, cradle_y, r.max.y, s);
    paint_mech(&p, mech_rect, ms, &art.hull, &art.hull_detail, &campaign, t);
    // Welding sparks
    for i in 0..10 {
        let h = hash2(i, (t * 14.0) as i32, 3);
        if h.is_multiple_of(3) {
            let x = mech_rect.min.x + (h % mech_rect.width() as u32) as f32;
            let y = mech_rect.min.y + ((h >> 8) % mech_rect.height() as u32) as f32;
            p.rect_filled(Rect::from_min_size(Pos2::new(x, y), Vec2::splat(s * 0.6)), 0.0, YELLOW);
        }
    }
    scanlines(&p, r, 18);

    // Title
    let title = Rect::from_min_size(r.min + Vec2::new(20.0, 16.0), Vec2::new(r.width() - 40.0, 48.0));
    p.rect_filled(title, 0.0, Color32::from_black_alpha(200));
    p.rect_filled(Rect::from_min_size(title.min, Vec2::new(6.0, title.height())), 0.0, AMBER);
    text(&p, title.left_center() + Vec2::new(18.0, 0.0), Align2::LEFT_CENTER, "THE HANGAR", pix(22.0), AMBER);
    text(&p, title.left_center() + Vec2::new(250.0, 0.0), Align2::LEFT_CENTER, "Depot 9, under the old tram viaduct. Rebuild what you can. Everything else rusts.", mono(24.0), TEXT);

    // --- Left: report + frame condition -----------------------------------------------
    let left = Rect::from_min_size(Pos2::new(r.min.x + 20.0, r.min.y + 80.0), Vec2::new(420.0, 470.0));
    let inner = panel_frame(&ui, left, "OPERATION REPORT", None);
    let mut y = inner.min.y;
    if let Some(rep) = &campaign.last_report {
        for (k, v) in [
            ("OPERATION", format!("{:02} — UPLINK RESTORED", rep.number)),
            ("DURATION", fmt_clock(rep.time)),
            ("KILLS", format!("{}", rep.kills)),
            ("PEAK HEAT", format!("{:.0}", rep.heat_peak)),
        ] {
            text(&p, Pos2::new(inner.min.x, y), Align2::LEFT_TOP, k, mono(20.0), DIM);
            text(&p, Pos2::new(inner.max.x, y), Align2::RIGHT_TOP, v, mono(20.0), TEXT);
            y += 20.0;
        }
        y += 6.0;
        text(&p, Pos2::new(inner.min.x, y), Align2::LEFT_TOP, "PERMANENT SCARS", pix(10.0), RED);
        y += 16.0;
        if rep.scars.is_empty() {
            text(&p, Pos2::new(inner.min.x, y), Align2::LEFT_TOP, "none — a clean run", mono(19.0), DIM);
            y += 18.0;
        }
        for (sec, amt) in rep.scars.iter().take(5) {
            text(&p, Pos2::new(inner.min.x, y), Align2::LEFT_TOP, format!("{:<18} max -{:.0}", sec.name(), amt), mono(19.0), Color32::from_rgb(230, 140, 120));
            y += 18.0;
        }
    }
    y += 8.0;
    text(&p, Pos2::new(inner.min.x, y), Align2::LEFT_TOP, "FRAME CONDITION (MAX INTEGRITY)", pix(10.0), AMBER);
    y += 18.0;
    for sec in Section::ALL.iter().copied().filter(|s| *s != Section::Uplink) {
        let m = campaign.makers[sec.idx()];
        let mx = campaign.max_hp[sec.idx()];
        text(&p, Pos2::new(inner.min.x, y), Align2::LEFT_TOP, sec.short(), pix(9.0), rgb(m.accent()));
        let bar = Rect::from_min_size(Pos2::new(inner.min.x + 56.0, y + 3.0), Vec2::new(150.0, 9.0));
        seg_bar(&p, bar, mx / 120.0, 12, health_color(mx / 100.0));
        text(&p, Pos2::new(inner.min.x + 214.0, y - 2.0), Align2::LEFT_TOP, format!("{:3.0}", mx), mono(18.0), TEXT);
        text(&p, Pos2::new(inner.max.x, y - 2.0), Align2::RIGHT_TOP, m.name(), mono(16.0), rgb(m.accent()).gamma_multiply(0.85));
        y += 18.0;
    }

    // --- Right: identity -----------------------------------------------------------------
    let right = Rect::from_min_size(Pos2::new(r.max.x - 440.0, r.min.y + 80.0), Vec2::new(420.0, 470.0));
    let inner = panel_frame(&ui, right, "WHAT IS LEFT OF HER", None);
    let mut y = inner.min.y;
    text(&p, Pos2::new(inner.min.x, y), Align2::LEFT_TOP, "ORIGINAL FRAME", pix(10.0), AMBER);
    text(&p, Pos2::new(inner.max.x, y - 6.0), Align2::RIGHT_TOP, format!("{:.0}%", campaign.originality), mono(34.0), AMBER);
    y += 26.0;
    seg_bar(&p, Rect::from_min_size(Pos2::new(inner.min.x, y), Vec2::new(inner.width(), 18.0)), campaign.originality / 100.0, 25, AMBER);
    y += 26.0;
    text(&p, Pos2::new(inner.min.x, y), Align2::LEFT_TOP, campaign.identity(), mono(22.0), TEXT);
    y += 30.0;
    text(&p, Pos2::new(inner.min.x, y), Align2::LEFT_TOP, "INSTALLED SALVAGE", pix(10.0), DIM);
    y += 18.0;
    if campaign.installed.is_empty() {
        text(&p, Pos2::new(inner.min.x, y), Align2::LEFT_TOP, "Nothing yet. Every bolt is factory.", mono(19.0), DIM);
        y += 18.0;
    }
    for id in campaign.installed.iter().rev().take(7) {
        let info = part(*id);
        text(&p, Pos2::new(inner.min.x, y), Align2::LEFT_TOP, format!("· {}", info.name), mono(19.0), rgb(info.maker.accent()));
        y += 18.0;
    }
    y += 8.0;
    let lore = "Each graft rewires a station in the cockpit: new faceplates, new readouts. Keep her original, or turn her into whatever survives.";
    child(&mut ui, Rect::from_min_max(Pos2::new(inner.min.x, y), inner.max), |ui| {
        ui.add(egui::Label::new(egui::RichText::new(lore).size(19.0).color(DIM)).wrap());
    });

    // --- Bottom: salvage choices --------------------------------------------------------------
    let cards_r = Rect::from_min_max(Pos2::new(r.min.x + 20.0, r.max.y - 300.0), Pos2::new(r.max.x - 20.0, r.max.y - 16.0));
    p.rect_filled(cards_r.expand(4.0), 0.0, Color32::from_black_alpha(150));
    text(&p, cards_r.left_top() + Vec2::new(4.0, -26.0), Align2::LEFT_TOP, "SALVAGE — CHOOSE ONE", pix(12.0), AMBER);
    let n = campaign.salvage.len().max(1) as f32;
    let gap = 12.0;
    let cw = (cards_r.width() - gap * (n - 1.0)) / n;
    let mut install: Option<PartId> = None;
    for (i, id) in campaign.salvage.clone().iter().enumerate() {
        let info = part(*id);
        let cr = Rect::from_min_size(Pos2::new(cards_r.min.x + i as f32 * (cw + gap), cards_r.min.y), Vec2::new(cw, cards_r.height()));
        let chosen = hs.chosen == Some(*id);
        let accent = rgb(info.maker.accent());
        p.rect_filled(cr, 0.0, PANEL);
        p.rect_stroke(cr, CornerRadius::ZERO, Stroke::new(if chosen { 3.0 } else { 1.0 }, if chosen { accent } else { EDGE_HI }), StrokeKind::Inside);
        p.rect_filled(Rect::from_min_size(cr.min, Vec2::new(cr.width(), 24.0)), 0.0, accent.gamma_multiply(0.22));
        text(&p, cr.min + Vec2::new(8.0, 12.0), Align2::LEFT_CENTER, info.maker.name(), pix(9.0), accent);
        text(&p, Pos2::new(cr.max.x - 8.0, cr.min.y + 12.0), Align2::RIGHT_CENTER, info.section.short(), pix(9.0), DIM);
        // Icon plinth
        let ir = Rect::from_min_size(cr.min + Vec2::new(10.0, 32.0), Vec2::splat(88.0));
        p.rect_filled(ir, 0.0, PANEL_DARK);
        let bob = ((t * 2.0 + i as f64).sin() * 2.0) as f32;
        art.sprites.icons[info.icon].paint(&p, ir.min + Vec2::new(4.0, 4.0 + bob.round()), 4.0, 0.0, None, false);
        child(&mut ui, Rect::from_min_max(Pos2::new(ir.max.x + 10.0, ir.min.y - 2.0), Pos2::new(cr.max.x - 8.0, ir.max.y + 8.0)), |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.add(egui::Label::new(egui::RichText::new(info.name).size(22.0).color(Color32::WHITE)).wrap());
            ui.add(egui::Label::new(egui::RichText::new(info.flavor).size(17.0).color(DIM)).wrap());
        });
        child(&mut ui, Rect::from_min_max(Pos2::new(cr.min.x + 10.0, ir.max.y + 12.0), Pos2::new(cr.max.x - 10.0, cr.max.y - 50.0)), |ui| {
            ui.add(egui::Label::new(egui::RichText::new(format!("+ {}", info.pros)).size(19.0).color(GREEN)).wrap());
            ui.add_space(2.0);
            ui.add(egui::Label::new(egui::RichText::new(format!("- {}", info.cons)).size(19.0).color(Color32::from_rgb(240, 120, 100))).wrap());
            if !info.maker.original() {
                ui.add_space(2.0);
                ui.label(egui::RichText::new("−11% ORIGINAL").size(16.0).color(AMBER));
            }
        });
        let br = Rect::from_min_size(Pos2::new(cr.min.x + 10.0, cr.max.y - 44.0), Vec2::new(cw - 20.0, 34.0));
        let label = if chosen { "INSTALLED" } else { "INSTALL" };
        if pbutton(&mut ui, br, label, accent, hs.chosen.is_none(), chosen).clicked() {
            install = Some(*id);
        }
    }
    if let Some(id) = install {
        campaign.install(id);
        hs.chosen = Some(id);
    }

    // Deploy
    let dr = Rect::from_min_size(Pos2::new(r.max.x - 380.0, cards_r.min.y - 58.0), Vec2::new(360.0, 44.0));
    let label = if hs.chosen.is_some() { format!("DEPLOY: OPERATION {:02}", campaign.mission) } else { "SKIP SALVAGE & DEPLOY".into() };
    if pbutton(&mut ui, dr, &label, AMBER, true, hs.chosen.is_some() && (t * 1.5).fract() < 0.7).clicked() {
        flow.request = Some(FlowRequest::Deploy);
    }
    Ok(())
}

/// Gantry platform the mech stands on, with hazard edge and support legs.
fn paint_cradle(p: &egui::Painter, mech: Rect, y: f32, floor: f32, s: f32) {
    let deck = Rect::from_min_max(Pos2::new(mech.min.x - 60.0, y), Pos2::new(mech.max.x + 60.0, y + 3.0 * s));
    for x in [deck.min.x + 10.0, deck.max.x - 10.0 - 2.0 * s, deck.center().x - s] {
        p.rect_filled(Rect::from_min_max(Pos2::new(x, deck.max.y), Pos2::new(x + 2.0 * s, floor)), 0.0, Color32::from_rgb(52, 48, 40));
    }
    let mut cy = deck.max.y + 4.0 * s;
    while cy < floor {
        p.line_segment([Pos2::new(deck.min.x + 10.0, cy), Pos2::new(deck.center().x, cy + 6.0 * s)], Stroke::new(s * 0.6, Color32::from_rgb(44, 40, 34)));
        p.line_segment([Pos2::new(deck.max.x - 10.0, cy), Pos2::new(deck.center().x, cy + 6.0 * s)], Stroke::new(s * 0.6, Color32::from_rgb(44, 40, 34)));
        cy += 12.0 * s;
    }
    p.rect_filled(deck, 0.0, Color32::from_rgb(70, 66, 60));
    let mut x = deck.min.x;
    let mut i = 0;
    while x < deck.max.x {
        let c = if i % 2 == 0 { Color32::from_rgb(200, 160, 40) } else { Color32::from_rgb(24, 22, 20) };
        p.rect_filled(Rect::from_min_max(Pos2::new(x, deck.max.y - s), Pos2::new((x + 2.0 * s).min(deck.max.x), deck.max.y)), 0.0, c);
        x += 2.0 * s;
        i += 1;
    }
    p.rect_filled(Rect::from_min_size(deck.min, Vec2::new(deck.width(), s * 0.6)), 0.0, Color32::from_rgb(120, 116, 104));
}

fn paint_mech(p: &egui::Painter, rect: Rect, s: f32, map: &[u8], detail: &[u8], campaign: &Campaign, t: f64) {
    let at = |x: i32, y: i32| -> u8 {
        if x < 0 || y < 0 || x >= art::HULL_W as i32 || y >= art::HULL_H as i32 { 254 } else { map[y as usize * art::HULL_W + x as usize] }
    };
    // Cradle shadow
    p.rect_filled(Rect::from_min_size(Pos2::new(rect.min.x - 10.0, rect.max.y - s), Vec2::new(rect.width() + 20.0, s * 2.0)), 0.0, Color32::from_black_alpha(160));
    for y in 0..art::HULL_H as i32 {
        for x in 0..art::HULL_W as i32 {
            let v = at(x, y);
            if v == 254 {
                continue;
            }
            let base = if v == 255 {
                Color32::from_rgb(70, 74, 80)
            } else {
                let sec = Section::ALL[v as usize];
                let maker = campaign.makers[sec.idx()];
                let metal = [96u8, 100, 106];
                let a = maker.accent();
                let mix = if maker.original() { 0.25 } else { 0.7 };
                let c = [
                    (metal[0] as f32 * (1.0 - mix) + a[0] as f32 * mix) as u8,
                    (metal[1] as f32 * (1.0 - mix) + a[1] as f32 * mix) as u8,
                    (metal[2] as f32 * (1.0 - mix) + a[2] as f32 * mix) as u8,
                ];
                let mut col = rgb(c);
                // Scars: rust and cracks proportional to lost max integrity.
                let lost = (100.0 - campaign.max_hp[sec.idx()]).max(0.0) / 60.0;
                if (hash2(x, y, 12) % 100) as f32 / 100.0 < lost {
                    col = Color32::from_rgb(110, 60, 36);
                }
                if sec == Section::Uplink && y < 3 && (t * 2.0).fract() < 0.5 {
                    col = CYAN;
                }
                col
            };
            let edge = at(x - 1, y) != v || at(x + 1, y) != v || at(x, y - 1) != v || at(x, y + 1) != v;
            let hi = at(x, y - 1) != v || at(x - 1, y) != v;
            let d = detail[y as usize * art::HULL_W + x as usize];
            let col = if d == art::HD_HAZARD {
                if art::hull_hazard_dark(x, y) { Color32::from_rgb(24, 22, 20) } else { art::hull_shade(base, d, t) }
            } else if d != art::HD_NONE {
                art::hull_shade(base, d, t)
            } else if edge {
                if hi { base.gamma_multiply(1.2) } else { base.gamma_multiply(0.4) }
            } else {
                base.gamma_multiply(0.8)
            };
            p.rect_filled(Rect::from_min_size(rect.min + Vec2::new(x as f32 * s, y as f32 * s), Vec2::splat(s)), 0.0, col);
        }
    }
}
