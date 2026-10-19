//! The cockpit: radar, external camera / interior cutaway, weapon stations,
//! power routing, heat, hull diagram and radio.

use std::f32::consts::{PI, TAU};

use bevy::prelude::*;
use bevy_egui::{
    EguiContexts,
    egui::{self, Align2, Color32, CornerRadius, LayerId, Order, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2},
};

use super::*;
use crate::art::{self, Sprite};
use crate::campaign::{Campaign, Maker, PartId};
use crate::rng::hash2;
use crate::sim::*;
use crate::{Flow, FlowRequest};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum CamMode {
    #[default]
    Auto,
    External,
    Interior,
}

#[derive(Default)]
pub struct CockpitUi {
    pub cam_bearing: f32,
    pub cam_mode: CamMode,
    pub selected_sys: usize,
}

/// Snapshot of the world the UI needs, gathered once per frame.
pub struct Scene {
    pub enemies: Vec<(Entity, Enemy)>,
    pub drones: Vec<Drone>,
    pub boarders: Vec<Boarder>,
    pub shells: Vec<Shell>,
    pub t: f64,
}

impl Scene {
    fn boarders_on(&self, s: Section) -> usize {
        self.boarders.iter().filter(|b| b.section == s).count()
    }
    fn shell_on(&self, s: Section) -> Option<&Shell> {
        self.shells.iter().filter(|sh| sh.target == s).min_by(|a, b| (a.flight - a.t).total_cmp(&(b.flight - b.t)))
    }
}

pub fn sys_color(s: Sys) -> Color32 {
    match s {
        Sys::Locomotion => Color32::from_rgb(214, 160, 92),
        Sys::Sensors => GREEN,
        Sys::Railgun => CYAN,
        Sys::Autocannon => YELLOW,
        Sys::Shields => Color32::from_rgb(110, 150, 255),
        Sys::Repair => Color32::from_rgb(240, 130, 60),
        Sys::Cooling => Color32::from_rgb(150, 230, 255),
    }
}

/// Root ui covering the whole viewport, zoomed so the cockpit is laid out at 1600x900.
pub fn root_ui(ctx: &egui::Context, name: &str) -> Ui {
    Ui::new(
        ctx.clone(),
        egui::Id::new(name),
        UiBuilder::new().layer_id(LayerId::background()).max_rect(ctx.content_rect()),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn cockpit_ui(
    mut contexts: EguiContexts,
    mut art: ResMut<Art>,
    mut mech: ResMut<Mech>,
    mut mission: ResMut<Mission>,
    radio: Res<Radio>,
    fx: Res<Effects>,
    mut cmds: ResMut<PilotCommands>,
    campaign: Res<Campaign>,
    mut flow: ResMut<Flow>,
    q: (Query<(Entity, &Enemy)>, Query<&Drone>, Query<&Boarder>, Query<&Shell>),
    keys: Res<ButtonInput<KeyCode>>,
    mut st: Local<CockpitUi>,
) -> Result {
    let ctx = contexts.ctx_mut()?.clone();
    art.ensure_textures(&ctx);
    let scene = Scene {
        enemies: q.0.iter().map(|(e, en)| (e, en.clone())).collect(),
        drones: q.1.iter().cloned().collect(),
        boarders: q.2.iter().cloned().collect(),
        shells: q.3.iter().cloned().collect(),
        t: ctx.input(|i| i.time),
    };

    handle_keys(&keys, &mut mech, &mut mission, &mut cmds, &mut st, &scene);

    let mut root = root_ui(&ctx, "cockpit");
    let r = root.max_rect();
    root.painter().rect_filled(r, 0.0, BG);
    // Hit shake: nudge the whole console a couple of pixels.
    let shake = if mission.running() { mech.shake } else { 0.0 };
    let jolt = Vec2::new(
        ((scene.t * 53.0).sin() as f32) * shake * 3.0,
        ((scene.t * 41.0).cos() as f32) * shake * 3.0,
    );
    let r = r.translate(jolt);

    let w = r.width();
    let h = r.height();
    let top = Rect::from_min_size(r.min + Vec2::new(8.0, 6.0), Vec2::new(w - 16.0, 44.0));
    let main_y0 = r.min.y + 56.0;
    let bottom_h = 236.0;
    let main_y1 = r.max.y - bottom_h - 14.0;
    let left_w = 396.0;
    let right_w = 516.0;
    let left = Rect::from_min_max(Pos2::new(r.min.x + 8.0, main_y0), Pos2::new(r.min.x + 8.0 + left_w, main_y1));
    let right = Rect::from_min_max(Pos2::new(r.max.x - 8.0 - right_w, main_y0), Pos2::new(r.max.x - 8.0, main_y1));
    let center = Rect::from_min_max(Pos2::new(left.max.x + 8.0, main_y0), Pos2::new(right.min.x - 8.0, main_y1));
    let feed_h = (center.height() * 0.58).max(200.0);
    let feed = Rect::from_min_size(center.min, Vec2::new(center.width(), feed_h));
    let weapons = Rect::from_min_max(Pos2::new(center.min.x, feed.max.y + 8.0), center.max);
    let hull = Rect::from_min_max(Pos2::new(r.min.x + 8.0, main_y1 + 8.0), Pos2::new(left.max.x, r.max.y - 6.0));
    let radio_r = Rect::from_min_max(Pos2::new(hull.max.x + 8.0, hull.min.y), Pos2::new(r.max.x - 8.0, hull.max.y));
    let _ = h;

    top_bar(&mut root, top, &mech, &mut mission, &scene);
    radar_panel(&mut root, left, &mut mech, &campaign, &scene);
    feed_panel(&mut root, feed, &art, &mut mech, &fx, &mut cmds, &campaign, &scene, &mut st);
    weapons_panel(&mut root, weapons, &mut mech, &mut cmds, &campaign, &scene);
    power_panel(&mut root, right, &mut mech, &mut cmds, &campaign, &scene, &mut st);
    hull_panel(&mut root, hull, &art, &mut mech, &mut cmds, &campaign, &scene);
    radio_panel(&mut root, radio_r, &art, &radio, mission.running());

    // Global alarm tint
    let p = root.painter();
    if mech.overload.is_some() && mission.outcome.is_none() {
        let a = (((scene.t * 4.0).sin() * 0.5 + 0.5) * 50.0) as u8;
        p.rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(255, 20, 10, a));
    } else if mech.hit_flash > 0.0 {
        p.rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(255, 60, 30, (mech.hit_flash * 40.0) as u8));
    }
    if mech.scram_t > 0.0 {
        p.rect_filled(r, 0.0, Color32::from_black_alpha(110));
        text(p, r.center() - Vec2::new(0.0, 120.0), Align2::CENTER_CENTER, format!("SCRAM — BUSES DEAD {:.1}s", mech.scram_t), pix(22.0), CYAN);
    }

    overlays(&ctx, &art, &mech, &mut mission, &campaign, &mut flow, &scene);
    Ok(())
}

fn handle_keys(keys: &ButtonInput<KeyCode>, mech: &mut Mech, mission: &mut Mission, cmds: &mut PilotCommands, st: &mut CockpitUi, scene: &Scene) {
    if (keys.just_pressed(KeyCode::KeyP) || keys.just_pressed(KeyCode::Escape))
        && mission.outcome.is_none() && !mission.briefing {
            mission.paused = !mission.paused;
        }
    if !mission.running() {
        return;
    }
    if keys.just_pressed(KeyCode::Space) {
        cmds.fire_rail = true;
    }
    if keys.just_pressed(KeyCode::KeyV) {
        cmds.vent = true;
    }
    if keys.just_pressed(KeyCode::KeyX) {
        cmds.scram = true;
    }
    if keys.just_pressed(KeyCode::KeyE) {
        cmds.ecm = true;
    }
    if keys.just_pressed(KeyCode::Tab) {
        let mut vis: Vec<&(Entity, Enemy)> = scene.enemies.iter().filter(|(_, e)| e.dist <= mech.stats.radar_range).collect();
        vis.sort_by(|a, b| a.1.dist.total_cmp(&b.1.dist));
        if !vis.is_empty() {
            let i = mech.target.and_then(|t| vis.iter().position(|(e, _)| *e == t)).map(|i| i + 1).unwrap_or(0);
            mech.target = Some(vis[i % vis.len()].0);
        }
    }
    let digits = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6, KeyCode::Digit7];
    for (i, k) in digits.iter().enumerate() {
        if keys.just_pressed(*k) {
            st.selected_sys = i;
        }
    }
    if mech.pilot.in_seat() {
        let step = if keys.pressed(KeyCode::ShiftLeft) { 1.0 } else { 5.0 };
        let s = Sys::ALL[st.selected_sys];
        if keys.just_pressed(KeyCode::ArrowUp) {
            mech.alloc[s.idx()] = (mech.alloc[s.idx()] + step).min(s.cap());
        }
        if keys.just_pressed(KeyCode::ArrowDown) {
            mech.alloc[s.idx()] = (mech.alloc[s.idx()] - step).max(0.0);
        }
    }
}

// --------------------------------------------------------------------------------------
// Top bar
// --------------------------------------------------------------------------------------

fn top_bar(ui: &mut Ui, rect: Rect, mech: &Mech, mission: &mut Mission, scene: &Scene) {
    let p = ui.painter().clone();
    p.rect_filled(rect, 0.0, PANEL);
    p.line_segment([rect.left_bottom(), rect.right_bottom()], Stroke::new(2.0, EDGE_LO));
    p.line_segment([rect.left_top(), rect.right_top()], Stroke::new(2.0, EDGE_HI));
    // Logo plate
    let logo = Rect::from_min_size(rect.min + Vec2::new(8.0, 7.0), Vec2::new(214.0, 30.0));
    p.rect_filled(logo, 0.0, AMBER);
    for i in 0..6 {
        let x = logo.max.x - 40.0 + i as f32 * 7.0;
        p.line_segment([Pos2::new(x, logo.max.y), Pos2::new(x + 12.0, logo.min.y)], Stroke::new(3.0, Color32::from_rgb(30, 24, 16)));
    }
    text(&p, logo.left_center() + Vec2::new(10.0, 0.0), Align2::LEFT_CENTER, "HANGAR ZERO", pix(16.0), Color32::from_rgb(20, 16, 10));
    text(&p, logo.right_center() + Vec2::new(10.0, -8.0), Align2::LEFT_CENTER, format!("WF-7 BASTION // OP-{:02}", mission.number), pix(10.0), AMBER);
    text(&p, logo.right_center() + Vec2::new(10.0, 8.0), Align2::LEFT_CENTER, "DEFEND & RESTORE UPLINK", mono(18.0), DIM);

    // Mission clock
    let clock_x = rect.min.x + 470.0;
    text(&p, Pos2::new(clock_x, rect.center().y), Align2::LEFT_CENTER, format!("T+{}", fmt_clock(mission.time)), mono(34.0), GREEN);

    // Objective bar
    let ob = Rect::from_min_size(Pos2::new(rect.min.x + 590.0, rect.min.y + 8.0), Vec2::new(420.0, 28.0));
    text(&p, ob.left_top() + Vec2::new(0.0, -1.0), Align2::LEFT_TOP, "CITY UPLINK RESTORE", pix(9.0), CYAN);
    let bar = Rect::from_min_max(ob.min + Vec2::new(0.0, 11.0), ob.max);
    seg_bar(&p, bar, mech.uplink() / 100.0, 40, CYAN);
    text(&p, Pos2::new(ob.max.x + 8.0, ob.center().y + 4.0), Align2::LEFT_CENTER, format!("{:3.0}%", mech.uplink()), mono(28.0), CYAN);
    let hint = if mech.repair_target == Section::Uplink { "DRONES ON UPLINK" } else { "DRONES ELSEWHERE" };
    text(&p, Pos2::new(ob.max.x - 0.0, ob.min.y - 1.0), Align2::RIGHT_TOP, hint, pix(8.0), if mech.repair_target == Section::Uplink { CYAN } else { AMBER });

    // Alarm lamps
    let lamps: [(&str, bool, Color32); 5] = [
        ("OVLD", mech.overload.is_some(), RED),
        ("HEAT", mech.heat > 80.0, AMBER),
        ("BREACH", mech.breach.is_some() || mech.interlocks_failed, RED),
        ("BOARD", !scene.boarders.is_empty(), PURPLE),
        ("INCOMING", !scene.shells.is_empty(), YELLOW),
    ];
    let mut x = rect.max.x - 5.0 * 74.0 - 60.0;
    for (label, on, c) in lamps {
        lamp(&p, Rect::from_min_size(Pos2::new(x, rect.min.y + 9.0), Vec2::new(70.0, 26.0)), label, on, c, scene.t);
        x += 74.0;
    }
    let pr = Rect::from_min_size(Pos2::new(rect.max.x - 52.0, rect.min.y + 9.0), Vec2::new(44.0, 26.0));
    if pbutton(ui, pr, if mission.paused { ">" } else { "||" }, DIM, mission.outcome.is_none() && !mission.briefing, mission.paused).clicked() {
        mission.paused = !mission.paused;
    }
}

// --------------------------------------------------------------------------------------
// Radar
// --------------------------------------------------------------------------------------

const RADAR_SCALE_M: f32 = 820.0;

const ICON_SCOUT: [&str; 7] = ["...x...", "..xxx..", ".xx.xx.", "xx...xx", ".xx.xx.", "..xxx..", "...x..."];
const ICON_ART: [&str; 7] = ["xxxxxxx", "x.....x", "x.xxx.x", "x.xxx.x", "x.xxx.x", "x.....x", "xxxxxxx"];
const ICON_CARRIER: [&str; 7] = ["..xxx..", ".x...x.", "x.x.x.x", "x..x..x", "x.x.x.x", ".x...x.", "..xxx.."];
const ICON_UNKNOWN: [&str; 7] = ["...x...", "...x...", ".......", "xx.x.xx", ".......", "...x...", "...x..."];

fn paint_icon(p: &egui::Painter, c: Pos2, icon: &[&str; 7], px: f32, col: Color32) {
    for (y, row) in icon.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if ch == 'x' {
                let min = c + Vec2::new((x as f32 - 3.5) * px, (y as f32 - 3.5) * px);
                p.rect_filled(Rect::from_min_size(min, Vec2::splat(px)), 0.0, col);
            }
        }
    }
}

fn kind_icon(k: EnemyKind) -> &'static [&'static str; 7] {
    match k {
        EnemyKind::Scout => &ICON_SCOUT,
        EnemyKind::Artillery => &ICON_ART,
        EnemyKind::Carrier => &ICON_CARRIER,
    }
}

fn kind_color(k: EnemyKind) -> Color32 {
    match k {
        EnemyKind::Scout => RED,
        EnemyKind::Artillery => Color32::from_rgb(255, 140, 60),
        EnemyKind::Carrier => PURPLE,
    }
}

fn pixel_ring(p: &egui::Painter, c: Pos2, r: f32, col: Color32, dash: bool) {
    let n = (r * TAU / 3.0) as usize;
    for i in 0..n {
        if dash && (i / 4) % 2 == 1 {
            continue;
        }
        let a = i as f32 / n as f32 * TAU;
        let pt = c + Vec2::new(a.cos() * r, a.sin() * r);
        p.rect_filled(Rect::from_min_size(Pos2::new(pt.x.round() - 1.0, pt.y.round() - 1.0), Vec2::splat(2.0)), 0.0, col);
    }
}

fn polar(c: Pos2, k: f32, angle: f32, dist: f32) -> Pos2 {
    c + Vec2::new(angle.sin(), -angle.cos()) * dist * k
}

fn radar_panel(ui: &mut Ui, rect: Rect, mech: &mut Mech, campaign: &Campaign, scene: &Scene) {
    let inner = panel_frame(ui, rect, "TACTICAL RADAR", Some(campaign.makers[Section::Sensors.idx()]));
    let square = campaign.installed.contains(&PartId::JackalDish);
    let size = inner.width().min(inner.height() - 170.0);
    let rr = Rect::from_center_size(Pos2::new(inner.center().x, inner.min.y + size / 2.0), Vec2::splat(size));
    let p = ui.painter_at(rr.expand(4.0));
    let c = rr.center();
    let rad = size / 2.0 - 14.0;
    let k = rad / RADAR_SCALE_M;
    let st = mech.stats;
    let scope_bg = Color32::from_rgb(6, 16, 10);
    let lit_bg = Color32::from_rgb(10, 30, 16);

    // Bezel
    p.rect_filled(rr, 0.0, PANEL_DARK);
    if square {
        p.rect_filled(Rect::from_center_size(c, Vec2::splat(rad * 2.0)), 0.0, scope_bg);
        p.rect_filled(Rect::from_center_size(c, Vec2::splat(st.radar_range * k * 2.0)), 0.0, lit_bg);
        let mut g = -rad;
        while g <= rad {
            p.line_segment([c + Vec2::new(g, -rad), c + Vec2::new(g, rad)], Stroke::new(1.0, GREEN_DIM.gamma_multiply(0.6)));
            p.line_segment([c + Vec2::new(-rad, g), c + Vec2::new(rad, g)], Stroke::new(1.0, GREEN_DIM.gamma_multiply(0.6)));
            g += rad / 4.0;
        }
        p.rect_stroke(Rect::from_center_size(c, Vec2::splat(rad * 2.0)), CornerRadius::ZERO, Stroke::new(2.0, rgb(Maker::Jackal.accent())), StrokeKind::Outside);
    } else {
        p.circle_filled(c, rad, scope_bg);
        p.circle_filled(c, (st.radar_range * k).min(rad), lit_bg);
        for i in 1..=4 {
            pixel_ring(&p, c, rad * i as f32 / 4.0, GREEN_DIM, false);
        }
        p.line_segment([c - Vec2::new(rad, 0.0), c + Vec2::new(rad, 0.0)], Stroke::new(1.0, GREEN_DIM));
        p.line_segment([c - Vec2::new(0.0, rad), c + Vec2::new(0.0, rad)], Stroke::new(1.0, GREEN_DIM));
    }
    // Detection & ID rings
    pixel_ring(&p, c, (st.radar_range * k).min(rad), GREEN.gamma_multiply(0.7), true);
    pixel_ring(&p, c, (st.id_range * k).min(rad), GREEN.gamma_multiply(0.3), true);
    pixel_ring(&p, c, AC_RANGE * k, YELLOW.gamma_multiply(0.25), true);
    for (lbl, a) in [("N", 0.0f32), ("E", PI / 2.0), ("S", PI), ("W", PI * 1.5)] {
        text(&p, polar(c, 1.0, a, rad + 8.0), Align2::CENTER_CENTER, lbl, pix(9.0), GREEN.gamma_multiply(0.7));
    }
    // Sweep with phosphor trail
    for i in 0..24 {
        let a = mech.sweep - i as f32 * 0.02;
        let alpha = (1.0 - i as f32 / 24.0) * 0.55;
        p.line_segment([c, polar(c, k, a, st.radar_range.min(RADAR_SCALE_M))], Stroke::new(2.0, GREEN.gamma_multiply(alpha)));
    }
    // Sensor damage: static and ghosts
    let sf = mech.frac(Section::Sensors);
    let frame = (scene.t * 12.0) as i32;
    let noise = ((1.0 - sf) * 160.0) as i32;
    for i in 0..noise {
        let h = hash2(i, frame, 5);
        let a = (h % 6283) as f32 / 1000.0;
        let d = ((h >> 13) % 1000) as f32 / 1000.0 * st.radar_range;
        let pt = polar(c, k, a, d);
        p.rect_filled(Rect::from_min_size(pt, Vec2::splat(2.0)), 0.0, GREEN.gamma_multiply(0.35));
    }
    if sf < 0.5 || square {
        let epoch = (scene.t / 3.0) as i32;
        for g in 0..2 {
            let h = hash2(g, epoch, 99);
            let a = (h % 6283) as f32 / 1000.0;
            let d = 150.0 + ((h >> 12) % 500) as f32;
            if d < st.radar_range {
                paint_icon(&p, polar(c, k, a, d), &ICON_UNKNOWN, 2.0, AMBER.gamma_multiply(0.5));
            }
        }
    }

    // Incoming shells
    for s in &scene.shells {
        let frac = (s.t / s.flight).clamp(0.0, 1.0);
        let from = polar(c, 1.0, s.angle, rad);
        let at = from + (c - from) * frac;
        p.line_segment([from, at], Stroke::new(1.0, RED.gamma_multiply(0.6)));
        p.rect_filled(Rect::from_center_size(at, Vec2::splat(5.0)), 0.0, RED);
    }

    // Contacts
    let resp = ui.interact(rr, ui.id().with("radar"), Sense::click());
    let click = resp.clicked().then(|| resp.interact_pointer_pos()).flatten();
    let mut best: Option<(Entity, f32)> = None;
    for (ent, e) in &scene.enemies {
        let pos = polar(c, k, e.angle, e.dist.min(RADAR_SCALE_M));
        let since = (mech.sweep - e.angle).rem_euclid(TAU);
        let glow = 1.0 - since / TAU * 0.65;
        let visible = e.dist <= st.radar_range;
        if !visible {
            if e.reveal > 0.0 {
                // Muzzle flash triangulation: bearing only.
                let edge = polar(c, 1.0, e.angle, rad);
                p.line_segment([c + (edge - c) * 0.75, edge], Stroke::new(2.0, Color32::from_rgb(255, 140, 60)));
                text(&p, edge + (c - edge).normalized() * 18.0, Align2::CENTER_CENTER, "FLASH", pix(8.0), Color32::from_rgb(255, 140, 60));
            }
            continue;
        }
        let identified = e.dist <= st.id_range;
        if identified {
            let col = kind_color(e.kind).gamma_multiply(glow);
            paint_icon(&p, pos, kind_icon(e.kind), 2.0, if e.hit_flash > 0.0 { Color32::WHITE } else { col });
            text(&p, pos + Vec2::new(10.0, -10.0), Align2::LEFT_BOTTOM, format!("{}-{:02}", e.kind.callsign(), e.serial), mono(16.0), col);
        } else {
            paint_icon(&p, pos, &ICON_UNKNOWN, 2.0, AMBER.gamma_multiply(glow));
        }
        if Some(*ent) == mech.target {
            let blink = (scene.t * 4.0).fract() < 0.7;
            if blink {
                let b = 11.0;
                for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                    let corner = pos + Vec2::new(sx * b, sy * b);
                    p.line_segment([corner, corner - Vec2::new(sx * 5.0, 0.0)], Stroke::new(2.0, Color32::WHITE));
                    p.line_segment([corner, corner - Vec2::new(0.0, sy * 5.0)], Stroke::new(2.0, Color32::WHITE));
                }
            }
            let dir = (pos - c).normalized();
            let mut d = 12.0;
            while d < (pos - c).length() - 14.0 {
                p.line_segment([c + dir * d, c + dir * (d + 4.0)], Stroke::new(1.0, Color32::WHITE.gamma_multiply(0.5)));
                d += 9.0;
            }
        }
        if let Some(cp) = click {
            let dd = (cp - pos).length();
            if dd < 22.0 && best.is_none_or(|(_, b)| dd < b) {
                best = Some((*ent, dd));
            }
        }
    }
    if let Some((e, _)) = best {
        mech.target = Some(e);
    }
    for d in &scene.drones {
        if d.dist <= st.radar_range {
            let pos = polar(c, k, d.angle, d.dist);
            p.rect_filled(Rect::from_center_size(pos, Vec2::splat(4.0)), 0.0, PURPLE);
        }
    }
    // Self + shield bubble
    let shield_frac = if st.shield_cap > 0.0 { mech.shield / st.shield_cap } else { 0.0 };
    if mech.shield > 1.0 {
        p.circle_stroke(c, 12.0, Stroke::new(2.0, Color32::from_rgb(110, 150, 255).gamma_multiply(0.3 + 0.7 * shield_frac)));
    }
    p.rect_filled(Rect::from_center_size(c, Vec2::new(6.0, 8.0)), 0.0, AMBER);
    p.rect_filled(Rect::from_center_size(c + Vec2::new(0.0, -5.0), Vec2::new(2.0, 3.0)), 0.0, AMBER);

    text(&p, rr.left_bottom() + Vec2::new(4.0, -4.0), Align2::LEFT_BOTTOM, format!("DET {:.0}m", st.radar_range), mono(18.0), GREEN);
    text(&p, rr.right_bottom() + Vec2::new(-4.0, -4.0), Align2::RIGHT_BOTTOM, format!("ID {:.0}m", st.id_range), mono(18.0), GREEN.gamma_multiply(0.6));
    text(&p, rr.left_top() + Vec2::new(4.0, 2.0), Align2::LEFT_TOP, format!("SCALE {:.0}m", RADAR_SCALE_M), mono(16.0), DIM);
    if square {
        text(&p, rr.right_top() + Vec2::new(-4.0, 2.0), Align2::RIGHT_TOP, "JACKAL FIRMWARE", mono(16.0), rgb(Maker::Jackal.accent()));
    }

    // Contact list
    let list = Rect::from_min_max(Pos2::new(inner.min.x, rr.max.y + 6.0), inner.max);
    let lp = ui.painter_at(list);
    lp.rect_filled(list, 0.0, PANEL_DARK);
    text(&lp, list.min + Vec2::new(6.0, 3.0), Align2::LEFT_TOP, "CONTACT        CLASS          RANGE  BRG  HULL", mono(16.0), DIM);
    let mut rows: Vec<&(Entity, Enemy)> = scene.enemies.iter().filter(|(_, e)| e.dist <= st.radar_range).collect();
    rows.sort_by(|a, b| a.1.dist.total_cmp(&b.1.dist));
    let mut y = list.min.y + 22.0;
    for (ent, e) in rows.iter().take(6) {
        let row = Rect::from_min_size(Pos2::new(list.min.x + 2.0, y), Vec2::new(list.width() - 4.0, 22.0));
        let sel = mech.target == Some(*ent);
        let resp = ui.interact(row, ui.id().with(("contact", ent.index_u32())), Sense::click());
        if sel {
            lp.rect_filled(row, 0.0, Color32::from_rgb(40, 34, 20));
        } else if resp.hovered() {
            lp.rect_filled(row, 0.0, Color32::from_rgb(28, 30, 34));
        }
        if resp.clicked() {
            mech.target = Some(*ent);
        }
        let identified = e.dist <= st.id_range;
        let col = if identified { kind_color(e.kind) } else { AMBER };
        paint_icon(&lp, Pos2::new(row.min.x + 12.0, row.center().y), if identified { kind_icon(e.kind) } else { &ICON_UNKNOWN }, 2.0, col);
        let name = if identified { format!("{}-{:02}", e.kind.callsign(), e.serial) } else { "????".into() };
        let class = if identified { e.kind.name() } else { "UNKNOWN" };
        text(&lp, Pos2::new(row.min.x + 26.0, row.center().y), Align2::LEFT_CENTER, name, mono(19.0), col);
        text(&lp, Pos2::new(row.min.x + 96.0, row.center().y), Align2::LEFT_CENTER, class, mono(19.0), TEXT);
        text(&lp, Pos2::new(row.min.x + 250.0, row.center().y), Align2::RIGHT_CENTER, format!("{:.0}m", e.dist), mono(19.0), TEXT);
        text(&lp, Pos2::new(row.min.x + 290.0, row.center().y), Align2::RIGHT_CENTER, format!("{:03.0}", bearing_deg(e.angle)), mono(19.0), DIM);
        let hb = Rect::from_min_size(Pos2::new(row.min.x + 300.0, row.center().y - 5.0), Vec2::new(row.width() - 306.0, 10.0));
        if identified {
            seg_bar(&lp, hb, e.hp / e.max_hp, 8, col);
        } else {
            text(&lp, hb.center(), Align2::CENTER_CENTER, "--", mono(16.0), DIM);
        }
        y += 23.0;
    }
    if rows.is_empty() {
        text(&lp, list.center(), Align2::CENTER_CENTER, "NO CONTACTS IN DETECTION RANGE", mono(19.0), DIM);
    }
}

// --------------------------------------------------------------------------------------
// External camera / interior cutaway
// --------------------------------------------------------------------------------------

const FEED_W: f32 = 320.0;
const FEED_H: f32 = 150.0;

#[allow(clippy::too_many_arguments)]
fn feed_panel(
    ui: &mut Ui,
    rect: Rect,
    art: &Art,
    mech: &mut Mech,
    fx: &Effects,
    cmds: &mut PilotCommands,
    campaign: &Campaign,
    scene: &Scene,
    st: &mut CockpitUi,
) {
    let interior = match st.cam_mode {
        CamMode::Interior => true,
        CamMode::External => false,
        CamMode::Auto => !mech.pilot.in_seat(),
    };
    let title = if interior { "INTERIOR CUTAWAY" } else { "EXTERNAL CAMERA FEED" };
    let inner = panel_frame(ui, rect, title, None);

    // Mode tabs in the header
    let tabs = [("AUTO", CamMode::Auto), ("EXT", CamMode::External), ("INT", CamMode::Interior)];
    let mut x = rect.max.x - 12.0 - 3.0 * 52.0;
    for (lbl, mode) in tabs {
        if pbutton(ui, Rect::from_min_size(Pos2::new(x, rect.min.y + 3.0), Vec2::new(48.0, 18.0)), lbl, GREEN, true, st.cam_mode == mode).clicked() {
            st.cam_mode = mode;
        }
        x += 52.0;
    }

    let scale = (inner.width() / FEED_W).min(inner.height() / FEED_H).max(1.0);
    let fr = Rect::from_center_size(inner.center(), Vec2::new(FEED_W * scale, FEED_H * scale));
    let p = ui.painter_at(fr);
    let painter_bg = ui.painter_at(inner);
    painter_bg.rect_filled(inner, 0.0, Color32::BLACK);

    if interior {
        interior_view(ui, &p, fr, scale, art, mech, cmds, scene);
    } else {
        external_view(&p, fr, scale, art, mech, fx, campaign, scene, st);
    }
    scanlines(&p, fr, 46);
    // Vignette corners
    for i in 0..6 {
        let a = 26 - i * 4;
        p.rect_stroke(fr.shrink(i as f32 * 3.0), CornerRadius::ZERO, Stroke::new(3.0, Color32::from_black_alpha(a as u8 * 4)), StrokeKind::Inside);
    }
}

#[allow(clippy::too_many_arguments)]
fn external_view(p: &egui::Painter, fr: Rect, s: f32, art: &Art, mech: &Mech, fx: &Effects, campaign: &Campaign, scene: &Scene, st: &mut CockpitUi) {
    // Camera follows the selected target; otherwise slowly scans.
    let want = mech
        .target
        .and_then(|t| scene.enemies.iter().find(|(e, _)| *e == t))
        .map(|(_, e)| e.angle)
        .unwrap_or(st.cam_bearing + 0.0015);
    st.cam_bearing += angle_diff(want, st.cam_bearing) * 0.08;
    st.cam_bearing = st.cam_bearing.rem_euclid(TAU);
    let bearing = st.cam_bearing;
    let fov = PI / 2.0;
    let px = |x: f32, y: f32| fr.min + Vec2::new(x * s, y * s);

    // Panorama (repeat-wrapped texture)
    if let Some(tex) = &art.pano {
        let u0 = (bearing - fov / 2.0) / TAU;
        let u1 = u0 + fov / TAU;
        let v1 = FEED_H / art::PANO_H as f32;
        p.image(tex.id(), fr, Rect::from_min_max(Pos2::new(u0, 0.0), Pos2::new(u1, v1)), Color32::WHITE);
    }
    let to_x = |angle: f32| -> Option<f32> {
        let rel = angle_diff(angle, bearing);
        (rel.abs() < fov * 0.6).then(|| FEED_W / 2.0 + rel / (fov / 2.0) * (FEED_W / 2.0))
    };
    let ground_y = |dist: f32| art::HORIZON as f32 + (1.0 - (dist / 1100.0).clamp(0.0, 1.0)).powf(1.6) * 44.0;

    // Enemies, far to near
    let mut list: Vec<&(Entity, Enemy)> = scene.enemies.iter().collect();
    list.sort_by(|a, b| b.1.dist.total_cmp(&a.1.dist));
    for (ent, e) in list {
        let Some(x) = to_x(e.angle) else { continue };
        let sprite: &Sprite = match e.kind {
            EnemyKind::Scout => &art.sprites.scout,
            EnemyKind::Artillery => &art.sprites.artillery,
            EnemyKind::Carrier => &art.sprites.carrier,
        };
        let k = if e.dist < 230.0 { 3.0 } else if e.dist < 520.0 { 2.0 } else if e.dist < 800.0 { 1.5 } else { 1.0 };
        let gy = ground_y(e.dist);
        // Walk bob
        let bob = if e.state == EnemyState::Approach || e.kind == EnemyKind::Scout { ((scene.t * 8.0 + e.serial as f64).sin() as f32 * 0.6).round() } else { 0.0 };
        let min = px(x - sprite.w as f32 * k / 2.0, gy - sprite.h as f32 * k + bob);
        let haze = if e.dist > 600.0 { Some(Color32::from_rgb(170, 130, 140)) } else { None };
        sprite.paint(p, Pos2::new(min.x.round(), min.y.round()), k * s, (e.hit_flash * 2.0).min(0.9), haze, e.orbit < 0.0 && e.kind == EnemyKind::Scout);
        if Some(*ent) == mech.target {
            let w = sprite.w as f32 * k * s;
            let h = sprite.h as f32 * k * s;
            let r = Rect::from_min_size(min, Vec2::new(w, h)).expand(4.0);
            let c = Color32::from_rgb(255, 80, 60);
            for (a, b) in [(r.left_top(), Vec2::new(8.0, 0.0)), (r.left_top(), Vec2::new(0.0, 8.0)), (r.right_top(), Vec2::new(-8.0, 0.0)), (r.right_top(), Vec2::new(0.0, 8.0)), (r.left_bottom(), Vec2::new(8.0, 0.0)), (r.left_bottom(), Vec2::new(0.0, -8.0)), (r.right_bottom(), Vec2::new(-8.0, 0.0)), (r.right_bottom(), Vec2::new(0.0, -8.0))] {
                p.line_segment([a, a + b], Stroke::new(2.0, c));
            }
            text(p, r.center_top() - Vec2::new(0.0, 4.0), Align2::CENTER_BOTTOM, format!("{}-{:02} {:.0}m", e.kind.callsign(), e.serial, e.dist), mono(16.0), c);
        }
    }
    for d in &scene.drones {
        if let Some(x) = to_x(d.angle) {
            let y = ground_y(d.dist) - 18.0 - (d.dist * 0.05).sin() * 4.0;
            let k = if d.dist < 200.0 { 2.0 } else { 1.0 };
            art.sprites.drone.paint(p, px(x, y), k * s, 0.0, None, false);
        }
    }
    // Shells arcing in
    for sh in &scene.shells {
        if let Some(x) = to_x(sh.angle) {
            let f = sh.t / sh.flight;
            let y = 10.0 + f * 120.0;
            let xx = x + (FEED_W / 2.0 - x) * f;
            p.rect_filled(Rect::from_min_size(px(xx, y), Vec2::splat(2.0 * s)), 0.0, YELLOW);
            p.rect_filled(Rect::from_min_size(px(xx, y - 3.0), Vec2::new(2.0 * s, 3.0 * s)), 0.0, Color32::from_rgba_unmultiplied(255, 150, 60, 120));
        }
    }

    // Effects
    let rail_origin = px(268.0, 150.0);
    let ac_origin = px(52.0, 150.0);
    for f in &fx.list {
        let life = f.t / f.ttl;
        let target = to_x(f.angle).map(|x| px(x, ground_y(f.dist) - 10.0));
        match f.kind {
            FxKind::Rail => {
                if let Some(t) = target {
                    let a = 1.0 - life;
                    p.line_segment([rail_origin, t], Stroke::new(6.0 * s * a, CYAN.gamma_multiply(a)));
                    p.line_segment([rail_origin, t], Stroke::new(2.0 * s * a, Color32::WHITE.gamma_multiply(a)));
                }
            }
            FxKind::Tracer => {
                if let Some(t) = target {
                    let a = ac_origin + (t - ac_origin) * (0.2 + life * 0.8);
                    let b = ac_origin + (t - ac_origin) * (life * 0.8);
                    p.line_segment([b, a], Stroke::new(1.5 * s, YELLOW));
                }
                p.rect_filled(Rect::from_center_size(ac_origin, Vec2::splat(6.0 * s * (1.0 - life))), 0.0, Color32::from_rgb(255, 230, 140));
            }
            FxKind::Explosion | FxKind::Muzzle => {
                if let Some(t) = target {
                    let big = if f.kind == FxKind::Explosion { 1.0 } else { 0.4 };
                    boom(p, t, s * big * (if f.dist < 300.0 { 3.0 } else { 1.6 }), life, hash2(f.angle.to_bits() as i32, 0, 3));
                }
            }
            FxKind::ShellImpact => {
                let x = to_x(f.angle).unwrap_or(FEED_W / 2.0);
                boom(p, px(x, 138.0), s * 4.0, life, 77);
            }
            FxKind::Ecm => {
                let r = life * fr.width();
                p.circle_stroke(fr.center_bottom(), r, Stroke::new(3.0 * s, CYAN.gamma_multiply(1.0 - life)));
            }
        }
    }

    // Cockpit framing: arm silhouettes in the lower corners
    let frame = Color32::from_rgb(18, 18, 22);
    let rail_col = rgb(campaign.makers[Section::Railgun.idx()].accent()).gamma_multiply(0.45);
    let ac_col = rgb(campaign.makers[Section::Autocannon.idx()].accent()).gamma_multiply(0.45);
    for i in 0..12 {
        let y = 150.0 - i as f32 * 2.0;
        let w = 22.0 - i as f32;
        p.rect_filled(Rect::from_min_size(px(256.0 + i as f32, y - 2.0), Vec2::new((w + 30.0) * s, 2.0 * s)), 0.0, frame);
        p.rect_filled(Rect::from_min_size(px(34.0 - i as f32 * 0.3, y - 2.0), Vec2::new((w + 10.0) * s, 2.0 * s)), 0.0, frame);
    }
    p.rect_filled(Rect::from_min_size(px(262.0, 140.0), Vec2::new(14.0 * s, 3.0 * s)), 0.0, rail_col);
    p.rect_filled(Rect::from_min_size(px(44.0, 140.0), Vec2::new(14.0 * s, 3.0 * s)), 0.0, ac_col);

    // HUD: bearing tape, crosshair, REC
    let tape_y = 4.0;
    for i in -9..=9 {
        let deg = (bearing.to_degrees() / 5.0).round() * 5.0 + i as f32 * 5.0;
        let rel = (deg.to_radians() - bearing + PI).rem_euclid(TAU) - PI;
        let x = FEED_W / 2.0 + rel / (fov / 2.0) * (FEED_W / 2.0);
        let d = deg.rem_euclid(360.0) as i32;
        let tall = d % 15 == 0;
        p.rect_filled(Rect::from_min_size(px(x, tape_y), Vec2::new(1.0 * s, if tall { 4.0 } else { 2.0 } * s)), 0.0, GREEN.gamma_multiply(0.8));
        if d % 45 == 0 {
            let lbl = match d {
                0 => "N".to_string(),
                90 => "E".into(),
                180 => "S".into(),
                270 => "W".into(),
                _ => format!("{d:03}"),
            };
            text(p, px(x, tape_y + 5.0), Align2::CENTER_TOP, lbl, mono(16.0), GREEN);
        }
    }
    let cc = px(FEED_W / 2.0, 75.0);
    for (a, b) in [(Vec2::new(-14.0, 0.0), Vec2::new(-5.0, 0.0)), (Vec2::new(5.0, 0.0), Vec2::new(14.0, 0.0)), (Vec2::new(0.0, -14.0), Vec2::new(0.0, -5.0)), (Vec2::new(0.0, 5.0), Vec2::new(0.0, 14.0))] {
        p.line_segment([cc + a, cc + b], Stroke::new(1.0, GREEN.gamma_multiply(0.7)));
    }
    if (scene.t * 1.5).fract() < 0.6 {
        p.circle_filled(fr.min + Vec2::new(16.0, 32.0), 5.0, RED);
    }
    text(p, fr.min + Vec2::new(26.0, 32.0), Align2::LEFT_CENTER, format!("CAM-1  BRG {:03.0}", bearing_deg(bearing)), mono(18.0), GREEN);
    text(p, fr.right_top() + Vec2::new(-8.0, 24.0), Align2::RIGHT_TOP, if mech.target.is_some() { "TRACK: TARGET" } else { "TRACK: SCAN" }, mono(18.0), GREEN);

    // Damaged sensors / heat
    let sf = mech.frac(Section::Sensors);
    let frame_i = (scene.t * 20.0) as i32;
    let static_n = ((1.0 - sf) * 500.0) as i32 + if mech.shake > 0.5 { 300 } else { 0 };
    for i in 0..static_n {
        let h = hash2(i, frame_i, 17);
        let x = (h % 320) as f32;
        let y = ((h >> 9) % 150) as f32;
        let v = ((h >> 20) % 200) as u8 + 40;
        p.rect_filled(Rect::from_min_size(px(x, y), Vec2::new(s * 2.0, s)), 0.0, Color32::from_rgb(v, v, v));
    }
    if sf < 0.3 && (frame_i / 5) % 7 == 0 {
        let y = ((hash2(frame_i, 1, 2) % 150) as f32).round();
        p.rect_filled(Rect::from_min_size(px(0.0, y), Vec2::new(fr.width(), 6.0 * s)), 0.0, Color32::from_white_alpha(40));
    }
    if mech.heat > 85.0 {
        let a = ((mech.heat - 85.0) / 45.0 * 90.0) as u8;
        p.rect_filled(fr, 0.0, Color32::from_rgba_unmultiplied(255, 60, 20, a));
    }
}

/// Chunky pixel explosion.
fn boom(p: &egui::Painter, c: Pos2, s: f32, life: f32, seed: u32) {
    let r = (2.0 + life * 6.0) * s;
    let cols = [Color32::WHITE, YELLOW, Color32::from_rgb(255, 140, 50), Color32::from_rgb(180, 60, 40), Color32::from_rgb(60, 50, 50)];
    let stage = ((life * 4.0) as usize).min(4);
    let n = 14;
    for i in 0..n {
        let h = hash2(i, seed as i32, 9);
        let a = (h % 628) as f32 / 100.0;
        let d = ((h >> 10) % 100) as f32 / 100.0 * r;
        let pt = c + Vec2::new(a.cos() * d, a.sin() * d * 0.7);
        let col = cols[(stage + (i as usize % 2)).min(4)];
        let sz = s * (2.0 - life);
        p.rect_filled(Rect::from_center_size(pt, Vec2::splat(sz.max(s * 0.6))), 0.0, col);
    }
}

#[allow(clippy::too_many_arguments)]
fn interior_view(ui: &mut Ui, p: &egui::Painter, fr: Rect, s: f32, art: &Art, mech: &Mech, cmds: &mut PilotCommands, scene: &Scene) {
    let px = |x: f32, y: f32| fr.min + Vec2::new(x * s, y * s);
    p.rect_filled(fr, 0.0, Color32::from_rgb(14, 12, 14));
    // Hull outline
    let decks: [&[Section]; 5] = [
        &[Section::Sensors, Section::Uplink],
        &[Section::Cockpit],
        &[Section::Autocannon, Section::ShieldGen, Section::Railgun],
        &[Section::Coolant, Section::Reactor],
        &[Section::LeftLeg, Section::RightLeg],
    ];
    let widths = [140.0, 110.0, 280.0, 200.0, 180.0];
    let mut room_rect = std::collections::HashMap::new();
    for (d, secs) in decks.iter().enumerate() {
        let y0 = 4.0 + d as f32 * 29.0;
        let w = widths[d];
        let x0 = (FEED_W - w) / 2.0;
        let rw = w / secs.len() as f32;
        for (i, sec) in secs.iter().enumerate() {
            let r = Rect::from_min_size(px(x0 + i as f32 * rw, y0), Vec2::new((rw - 2.0) * s, 26.0 * s));
            room_rect.insert(*sec, r);
        }
    }
    // Outer hull plating around every deck, plus a faint blueprint grid behind.
    let mut gx = 0.0;
    while gx < FEED_W {
        p.rect_filled(Rect::from_min_size(px(gx, 0.0), Vec2::new(1.0, fr.height())), 0.0, Color32::from_rgb(22, 24, 32));
        gx += 10.0;
    }
    for (d, secs) in decks.iter().enumerate() {
        let first = room_rect[&secs[0]];
        let last = room_rect[&secs[secs.len() - 1]];
        let hull = Rect::from_min_max(first.min, last.max).expand(3.0 * s);
        p.rect_filled(hull, 0.0, Color32::from_rgb(58, 54, 62));
        p.rect_filled(Rect::from_min_size(hull.min, Vec2::new(hull.width(), s)), 0.0, Color32::from_rgb(92, 88, 96));
        let mut rx = hull.min.x + 3.0 * s;
        while rx < hull.max.x - 2.0 * s {
            p.rect_filled(Rect::from_min_size(Pos2::new(rx, hull.min.y + s), Vec2::splat(s)), 0.0, Color32::from_rgb(30, 28, 34));
            p.rect_filled(Rect::from_min_size(Pos2::new(rx, hull.max.y - 2.0 * s), Vec2::splat(s)), 0.0, Color32::from_rgb(30, 28, 34));
            rx += 8.0 * s;
        }
        let _ = d;
    }
    // Ladder shaft
    let shaft = Rect::from_min_size(px(156.0, 4.0), Vec2::new(8.0 * s, 145.0 * s));
    p.rect_filled(shaft, 0.0, Color32::from_rgb(30, 28, 30));
    let mut y = 6.0;
    while y < 148.0 {
        p.rect_filled(Rect::from_min_size(px(157.0, y), Vec2::new(6.0 * s, 1.0 * s)), 0.0, Color32::from_rgb(90, 80, 60));
        y += 4.0;
    }
    let t = scene.t;
    for (sec, r) in &room_rect {
        let f = mech.frac(*sec);
        let hc = health_color(if *sec == Section::Uplink { mech.uplink() / 100.0 } else { f });
        p.rect_filled(*r, 0.0, Color32::from_rgb(36, 34, 40));
        p.rect_stroke(*r, CornerRadius::ZERO, Stroke::new(s, Color32::from_rgb(70, 66, 72)), StrokeKind::Inside);
        // Pipes
        p.rect_filled(Rect::from_min_size(r.min + Vec2::new(0.0, 3.0 * s), Vec2::new(r.width(), s)), 0.0, Color32::from_rgb(64, 54, 44));
        p.rect_filled(Rect::from_min_size(r.min + Vec2::new(0.0, r.height() - 4.0 * s), Vec2::new(r.width(), 2.0 * s)), 0.0, Color32::from_rgb(28, 26, 30));
        // Status light
        let blink = f < 0.35 && (t * 4.0).fract() < 0.5;
        p.rect_filled(Rect::from_min_size(r.right_top() + Vec2::new(-5.0 * s, 2.0 * s), Vec2::splat(2.0 * s)), 0.0, if blink { Color32::BLACK } else { hc });
        text(p, r.left_top() + Vec2::new(3.0 * s, 5.0 * s), Align2::LEFT_TOP, sec.short(), pix(9.0), hc);
        // Sparks for damaged rooms
        if f < 0.5 && *sec != Section::Uplink {
            for i in 0..4 {
                let h = hash2(i, (t * 10.0) as i32, sec.idx() as u32);
                if h.is_multiple_of(3) {
                    let x = (h >> 4) % (r.width() as u32).max(1);
                    p.rect_filled(Rect::from_min_size(Pos2::new(r.min.x + x as f32, r.min.y + 4.0 * s), Vec2::splat(s)), 0.0, YELLOW);
                }
            }
        }
        // Boarders in the room
        let n = scene.boarders_on(*sec);
        for i in 0..n {
            let wiggle = ((t * 6.0 + i as f64).sin() as f32).round();
            let bx = r.max.x - (12.0 + i as f32 * 11.0) * s;
            art.sprites.tick.paint(p, Pos2::new(bx, r.max.y - 11.0 * s + wiggle), s, 0.0, None, i % 2 == 1);
        }
        // Click to crawl there
        let resp = ui.interact(*r, ui.id().with(("room", sec.idx())), Sense::click());
        if resp.hovered() {
            p.rect_stroke(*r, CornerRadius::ZERO, Stroke::new(s, AMBER), StrokeKind::Inside);
        }
        if resp.clicked() && mech.pilot.in_seat() && *sec != Section::Cockpit {
            cmds.crawl_to = Some(*sec);
        }
    }
    // Pilot position
    let seat = room_rect[&Section::Cockpit].center();
    let floor = |r: &Rect| Pos2::new(r.center().x, r.max.y - 13.0 * s);
    let ladder_x = shaft.center().x;
    let path = |to: Section, f: f32| -> Pos2 {
        let a = floor(&room_rect[&Section::Cockpit]);
        let b = floor(&room_rect[&to]);
        if f < 0.5 {
            let g = f * 2.0;
            Pos2::new(a.x + (ladder_x - a.x) * g.min(0.3) / 0.3, a.y + (b.y - a.y) * ((g - 0.3).max(0.0) / 0.7))
        } else {
            let g = (f - 0.5) * 2.0;
            Pos2::new(ladder_x + (b.x - ladder_x) * g, b.y)
        }
    };
    let (pos, status) = match mech.pilot {
        Pilot::Seat => (floor(&room_rect[&Section::Cockpit]), "PILOT IN CHAIR. CLICK A COMPARTMENT TO CRAWL THERE.".to_string()),
        Pilot::Crawling { to, t, dur } => (path(to, t / dur), format!("CRAWLING TO {} ... {:.1}s", to.name(), dur - t)),
        Pilot::Working { at, t } => {
            let n = scene.boarders_on(at);
            let msg = if n > 0 { format!("FIGHTING BOARDERS ({n}) ... {:.0}%", (t / 2.2 * 100.0).min(100.0)) } else { format!("HAND REPAIR: {} +5/s", at.name()) };
            (floor(&room_rect[&at]), msg)
        }
        Pilot::Returning { from, t, dur } => (path(from, 1.0 - t / dur), format!("RETURNING TO CHAIR ... {:.1}s", dur - t)),
    };
    let _ = seat;
    art.sprites.pilot.paint(p, pos - Vec2::new(4.0 * s, 0.0), s, 0.0, None, false);
    if let Pilot::Working { .. } = mech.pilot {
        let swing = ((t * 8.0).sin() as f32) * 2.0;
        art.sprites.wrench.paint(p, pos + Vec2::new(4.0 * s, (2.0 + swing) * s), s, 0.0, None, false);
    }
    // Status strip
    let strip = Rect::from_min_size(fr.left_bottom() - Vec2::new(0.0, 20.0), Vec2::new(fr.width(), 20.0));
    p.rect_filled(strip, 0.0, Color32::from_black_alpha(200));
    text(p, strip.left_center() + Vec2::new(8.0, 0.0), Align2::LEFT_CENTER, status, mono(18.0), AMBER);
    if !mech.pilot.in_seat() {
        if (t * 2.0).fract() < 0.7 {
            text(p, strip.right_center() - Vec2::new(8.0, 0.0), Align2::RIGHT_CENTER, "PILOT OUT OF CHAIR — WEAPONS HOLD", pix(9.0), RED);
        }
        if let Pilot::Working { .. } = mech.pilot {
            let br = Rect::from_min_size(fr.right_top() + Vec2::new(-176.0, 8.0), Vec2::new(168.0, 26.0));
            if pbutton(ui, br, "RETURN TO CHAIR", GREEN, true, true).clicked() {
                cmds.return_to_seat = true;
            }
        }
    }
}

// --------------------------------------------------------------------------------------
// Weapons
// --------------------------------------------------------------------------------------

fn weapons_panel(ui: &mut Ui, rect: Rect, mech: &mut Mech, cmds: &mut PilotCommands, campaign: &Campaign, scene: &Scene) {
    let inner = panel_frame(ui, rect, "WEAPON STATIONS", None);
    let gap = 8.0;
    let cw = (inner.width() - gap * 2.0) / 3.0;
    let cards: Vec<Rect> = (0..3).map(|i| Rect::from_min_size(Pos2::new(inner.min.x + i as f32 * (cw + gap), inner.min.y), Vec2::new(cw, inner.height()))).collect();
    let st = mech.stats;
    let seated = mech.pilot.in_seat();
    let target = mech.target.and_then(|t| scene.enemies.iter().find(|(e, _)| *e == t)).map(|(_, e)| e.clone());

    // --- Railgun -------------------------------------------------------------------------
    {
        let r = cards[0];
        let maker = campaign.makers[Section::Railgun.idx()];
        station_card(ui, r, "RAILGUN", maker);
        let p = ui.painter_at(r);
        let x = r.min.x + 8.0;
        let mut y = r.min.y + 26.0;
        let charge = mech.rail_charge;
        let ready = charge >= 1.0;
        let lock = target.as_ref().is_some_and(|e| e.dist <= st.radar_range);
        let status = if !seated {
            ("NO PILOT", RED)
        } else if st.rail_rate <= 0.0 && !ready {
            ("NO POWER", RED)
        } else if !ready {
            ("CHARGING", AMBER)
        } else if !lock {
            ("READY / NO LOCK", AMBER)
        } else {
            ("READY", GREEN)
        };
        text(&p, Pos2::new(x, y), Align2::LEFT_TOP, "CAPACITOR", pix(9.0), DIM);
        text(&p, Pos2::new(r.max.x - 8.0, y), Align2::RIGHT_TOP, status.0, pix(9.0), status.1);
        y += 14.0;
        seg_bar(&p, Rect::from_min_size(Pos2::new(x, y), Vec2::new(r.width() - 16.0, 22.0)), charge, 20, if ready { CYAN } else { CYAN.gamma_multiply(0.6) });
        y += 28.0;
        let eta = if ready { "--".into() } else if st.rail_rate > 0.0 { format!("{:.1}s", (1.0 - charge) / st.rail_rate) } else { "∞".into() };
        let hitp = 0.92 * (0.5 + 0.5 * mech.eff(Section::Sensors)) - if target.as_ref().is_some_and(|e| e.kind == EnemyKind::Scout) { 0.15 } else { 0.0 };
        for (k, v) in [
            ("ETA", eta),
            ("HEAT/SHOT", format!("+{:.0}", 22.0 + campaign.mods.rail_heat)),
            ("HIT CHANCE", if lock { format!("{:.0}%", hitp * 100.0) } else { "--".into() }),
            ("MOUNT", format!("{:.0}%", mech.frac(Section::Railgun) * 100.0)),
        ] {
            text(&p, Pos2::new(x, y), Align2::LEFT_TOP, k, mono(18.0), DIM);
            text(&p, Pos2::new(r.max.x - 8.0, y), Align2::RIGHT_TOP, v, mono(18.0), TEXT);
            y += 17.0;
        }
        let by = r.max.y - 44.0;
        let fire = Rect::from_min_size(Pos2::new(x, by), Vec2::new(r.width() * 0.62, 36.0));
        if pbutton(ui, fire, "FIRE [SPACE]", RED, seated && ready && lock && mech.scram_t <= 0.0, ready && lock).clicked() {
            cmds.fire_rail = true;
        }
        let auto = Rect::from_min_max(Pos2::new(fire.max.x + 6.0, by), Pos2::new(r.max.x - 8.0, by + 36.0));
        if pbutton(ui, auto, "AUTO", AMBER, true, mech.rail_auto).clicked() {
            mech.rail_auto = !mech.rail_auto;
        }
    }

    // --- Autocannon ----------------------------------------------------------------------
    {
        let r = cards[1];
        let maker = campaign.makers[Section::Autocannon.idx()];
        station_card(ui, r, "AUTOCANNON", maker);
        let p = ui.painter_at(r);
        let x = r.min.x + 8.0;
        let mut y = r.min.y + 26.0;
        let status = if !seated {
            ("NO PILOT", RED)
        } else if st.ac_dps <= 0.0 {
            ("NO POWER", RED)
        } else if mech.ac_firing {
            ("FIRING", YELLOW)
        } else if !mech.weapons_free {
            ("SAFE", DIM)
        } else {
            ("TRACKING", GREEN)
        };
        text(&p, Pos2::new(x, y), Align2::LEFT_TOP, "BARREL", pix(9.0), DIM);
        text(&p, Pos2::new(r.max.x - 8.0, y), Align2::RIGHT_TOP, status.0, pix(9.0), status.1);
        y += 14.0;
        // Spinning barrel / ammo belt animation
        let belt = Rect::from_min_size(Pos2::new(x, y), Vec2::new(r.width() - 16.0, 22.0));
        p.rect_filled(belt, 0.0, EDGE_LO);
        let offset = if mech.ac_firing { ((scene.t * 40.0) % 10.0) as f32 } else { 0.0 };
        let mut bx = belt.min.x + 3.0 - offset;
        while bx < belt.max.x - 6.0 {
            if bx > belt.min.x {
                p.rect_filled(Rect::from_min_size(Pos2::new(bx, belt.min.y + 4.0), Vec2::new(5.0, 14.0)), 0.0, Color32::from_rgb(190, 150, 60));
                p.rect_filled(Rect::from_min_size(Pos2::new(bx, belt.min.y + 4.0), Vec2::new(5.0, 4.0)), 0.0, Color32::from_rgb(230, 200, 120));
            }
            bx += 10.0;
        }
        y += 28.0;
        for (k, v) in [
            ("DAMAGE", format!("{:.0}/s", st.ac_dps)),
            ("RANGE", format!("{:.0}m", AC_RANGE)),
            ("HEAT", format!("+{:.1}/s firing", st.eff[Sys::Autocannon.idx()] * 0.14)),
            ("MOUNT", format!("{:.0}%", mech.frac(Section::Autocannon) * 100.0)),
        ] {
            text(&p, Pos2::new(x, y), Align2::LEFT_TOP, k, mono(18.0), DIM);
            text(&p, Pos2::new(r.max.x - 8.0, y), Align2::RIGHT_TOP, v, mono(18.0), TEXT);
            y += 17.0;
        }
        let by = r.max.y - 44.0;
        let half = (r.width() - 22.0) / 2.0;
        if pbutton(ui, Rect::from_min_size(Pos2::new(x, by), Vec2::new(half, 36.0)), "WPN FREE", YELLOW, true, mech.weapons_free).clicked() {
            mech.weapons_free = !mech.weapons_free;
        }
        if pbutton(ui, Rect::from_min_size(Pos2::new(x + half + 6.0, by), Vec2::new(half, 36.0)), "PT DEFENCE", PURPLE, true, mech.point_defence).clicked() {
            mech.point_defence = !mech.point_defence;
        }
    }

    // --- EW & crew -----------------------------------------------------------------------
    {
        let r = cards[2];
        station_card(ui, r, "EW / CREW", campaign.makers[Section::Sensors.idx()]);
        let p = ui.painter_at(r);
        let x = r.min.x + 8.0;
        let mut y = r.min.y + 26.0;
        text(&p, Pos2::new(x, y), Align2::LEFT_TOP, "ECM CAPACITOR", pix(9.0), DIM);
        let ecm_ok = mech.ecm_cd <= 0.0 && st.eff[Sys::Sensors.idx()] >= 5.0;
        text(&p, Pos2::new(r.max.x - 8.0, y), Align2::RIGHT_TOP, if ecm_ok { "READY" } else if mech.ecm_cd > 0.0 { "RECHARGE" } else { "SENS<5" }, pix(9.0), if ecm_ok { GREEN } else { AMBER });
        y += 14.0;
        seg_bar(&p, Rect::from_min_size(Pos2::new(x, y), Vec2::new(r.width() - 16.0, 14.0)), 1.0 - mech.ecm_cd / 28.0, 14, CYAN);
        y += 20.0;
        let in_flight = scene.drones.len();
        let on_hull = scene.boarders.len();
        for (k, v, c) in [
            ("DRONES INBOUND", format!("{in_flight}"), if in_flight > 0 { PURPLE } else { TEXT }),
            ("BOARDERS ON HULL", format!("{on_hull}"), if on_hull > 0 { PURPLE } else { TEXT }),
            ("PARASITIC DRAW", format!("-{:.0}u", st.parasitic), if st.parasitic > 0.0 { PURPLE } else { TEXT }),
        ] {
            text(&p, Pos2::new(x, y), Align2::LEFT_TOP, k, mono(18.0), DIM);
            text(&p, Pos2::new(r.max.x - 8.0, y), Align2::RIGHT_TOP, v, mono(18.0), c);
            y += 17.0;
        }
        let by = r.max.y - 44.0;
        let half = (r.width() - 22.0) / 2.0;
        if pbutton(ui, Rect::from_min_size(Pos2::new(x, by), Vec2::new(half, 36.0)), "ECM [E]", CYAN, ecm_ok && seated, ecm_ok).clicked() {
            cmds.ecm = true;
        }
        // Leave the chair: go to the worst boarded section (or the most damaged one).
        let dest = scene
            .boarders
            .first()
            .map(|b| b.section)
            .or_else(|| {
                Section::ALL
                    .iter()
                    .copied()
                    .filter(|s| *s != Section::Cockpit && *s != Section::Uplink)
                    .min_by(|a, b| mech.frac(*a).total_cmp(&mech.frac(*b)))
            })
            .unwrap_or(Section::Reactor);
        let lbl = if seated { "LEAVE CHAIR" } else { "RETURN" };
        let br = Rect::from_min_size(Pos2::new(x + half + 6.0, by), Vec2::new(half, 36.0));
        let can_return = matches!(mech.pilot, Pilot::Working { .. });
        if pbutton(ui, br, lbl, if seated { PURPLE } else { GREEN }, seated || can_return, on_hull > 0 && seated).clicked() {
            if seated {
                cmds.crawl_to = Some(dest);
            } else {
                cmds.return_to_seat = true;
            }
        }
        text(&p, Pos2::new(x, by - 4.0), Align2::LEFT_BOTTOM, if seated { format!("CRAWL TARGET: {}", dest.short()) } else { "PILOT AWAY".into() }, mono(16.0), DIM);
    }
}

fn station_card(ui: &Ui, r: Rect, name: &str, maker: Maker) {
    let p = ui.painter();
    let accent = rgb(maker.accent());
    p.rect_filled(r, 0.0, PANEL_DARK);
    p.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, EDGE_HI), StrokeKind::Inside);
    p.rect_filled(Rect::from_min_size(r.min, Vec2::new(r.width(), 20.0)), 0.0, accent.gamma_multiply(0.18));
    p.rect_filled(Rect::from_min_size(r.min, Vec2::new(4.0, 20.0)), 0.0, accent);
    text(p, r.min + Vec2::new(10.0, 10.0), Align2::LEFT_CENTER, name, pix(11.0), accent);
    text(p, Pos2::new(r.max.x - 6.0, r.min.y + 10.0), Align2::RIGHT_CENTER, maker.name(), pix(8.0), accent.gamma_multiply(0.7));
}

// --------------------------------------------------------------------------------------
// Power routing
// --------------------------------------------------------------------------------------

fn power_panel(ui: &mut Ui, rect: Rect, mech: &mut Mech, cmds: &mut PilotCommands, campaign: &Campaign, scene: &Scene, ui_st: &mut CockpitUi) {
    let inner = panel_frame(ui, rect, "REACTOR & POWER ROUTING", Some(campaign.makers[Section::Reactor.idx()]));
    let st = mech.stats;
    let seated = mech.pilot.in_seat();
    let p = ui.painter_at(inner);

    // --- Reactor bus bar -----------------------------------------------------------------
    let bus = Rect::from_min_size(inner.min, Vec2::new(inner.width(), 60.0));
    let output = campaign.mods.reactor_output;
    text(&p, bus.min, Align2::LEFT_TOP, format!("CORE {:.0}u", output), pix(10.0), AMBER);
    let over = st.draw > st.available;
    text(
        &p,
        Pos2::new(bus.max.x, bus.min.y - 2.0),
        Align2::RIGHT_TOP,
        format!("DRAW {:.0} / AVAIL {:.0}", st.draw, st.available),
        mono(20.0),
        if st.supply < 1.0 { RED } else if over { AMBER } else { GREEN },
    );
    let bar = Rect::from_min_size(bus.min + Vec2::new(0.0, 20.0), Vec2::new(bus.width(), 20.0));
    let full = output * OVERDRIVE;
    let to_x = |v: f32| bar.min.x + (v / full).clamp(0.0, 1.0) * bar.width();
    p.rect_filled(bar, 0.0, EDGE_LO);
    // Stacked draw by system
    let mut acc = 0.0;
    for s in Sys::ALL {
        let v = mech.alloc[s.idx()];
        let r = Rect::from_min_max(Pos2::new(to_x(acc), bar.min.y + 2.0), Pos2::new(to_x(acc + v), bar.max.y - 2.0));
        p.rect_filled(r, 0.0, sys_color(s).gamma_multiply(0.75));
        acc += v;
    }
    // Overdrive zone hatching + parasitic + damage loss
    let avail_x = to_x(st.available);
    let ceil_x = to_x(st.available * OVERDRIVE);
    let mut hx = avail_x;
    while hx < ceil_x {
        p.line_segment([Pos2::new(hx, bar.max.y), Pos2::new(hx + 6.0, bar.min.y)], Stroke::new(1.0, RED.gamma_multiply(0.6)));
        hx += 6.0;
    }
    p.line_segment([Pos2::new(avail_x, bar.min.y - 3.0), Pos2::new(avail_x, bar.max.y + 3.0)], Stroke::new(2.0, Color32::WHITE));
    p.line_segment([Pos2::new(ceil_x, bar.min.y - 3.0), Pos2::new(ceil_x, bar.max.y + 3.0)], Stroke::new(2.0, RED));
    let mut note = String::new();
    if st.parasitic > 0.0 {
        note.push_str(&format!("PARASITIC -{:.0}  ", st.parasitic));
    }
    if mech.frac(Section::Reactor) < 1.0 {
        note.push_str(&format!("CORE DMG -{:.0}  ", output * 0.5 * (1.0 - mech.frac(Section::Reactor))));
    }
    if st.supply < 1.0 {
        note.push_str(&format!("BROWNOUT {:.0}%", st.supply * 100.0));
    } else if over {
        note.push_str(&format!("OVERDRIVE +{:.0}u → +{:.1} heat/s", st.draw - st.available, (st.draw - st.available) * OVERDRIVE_HEAT));
    }
    text(&p, bar.left_bottom() + Vec2::new(0.0, 3.0), Align2::LEFT_TOP, if note.is_empty() { "BUS NOMINAL".to_string() } else { note }, mono(17.0), if over || st.parasitic > 0.0 { AMBER } else { DIM });

    // --- Faders ---------------------------------------------------------------------------
    let fz = Rect::from_min_max(Pos2::new(inner.min.x, bus.max.y + 6.0), Pos2::new(inner.max.x - 118.0, inner.max.y - 84.0));
    let n = Sys::ALL.len() as f32;
    let fw = fz.width() / n;
    let col_top = fz.min.y + 22.0;
    let col_bot = fz.max.y - 62.0;
    let unit_h = (col_bot - col_top) / 40.0;
    for (i, s) in Sys::ALL.iter().copied().enumerate() {
        let cx = fz.min.x + fw * (i as f32 + 0.5);
        let colw = (fw - 18.0).min(40.0);
        let cap = s.cap();
        let slot = Rect::from_min_max(Pos2::new(cx - colw / 2.0, col_bot - cap * unit_h), Pos2::new(cx + colw / 2.0, col_bot));
        let resp = ui.interact(slot.expand2(Vec2::new(4.0, 8.0)), ui.id().with(("fader", i)), Sense::click_and_drag());
        if (resp.dragged() || resp.clicked()) && seated
            && let Some(pos) = resp.interact_pointer_pos() {
                let v = ((col_bot - pos.y) / unit_h).round().clamp(0.0, cap);
                mech.alloc[s.idx()] = v;
                ui_st.selected_sys = i;
            }
        let col = sys_color(s);
        let pp = ui.painter();
        pp.rect_filled(slot.expand(2.0), 0.0, EDGE_LO);
        let seg_u = 2.5;
        let segs = (cap / seg_u) as usize;
        let val = mech.alloc[s.idx()];
        let effv = st.eff[s.idx()];
        for k in 0..segs {
            let y1 = col_bot - k as f32 * seg_u * unit_h;
            let r = Rect::from_min_max(Pos2::new(slot.min.x + 2.0, y1 - seg_u * unit_h + 1.5), Pos2::new(slot.max.x - 2.0, y1 - 0.5));
            let lo = k as f32 * seg_u;
            let c = if lo < effv - 0.01 {
                col
            } else if lo < val - 0.01 {
                RED.gamma_multiply(0.5)
            } else {
                col.gamma_multiply(0.08)
            };
            pp.rect_filled(r, 0.0, c);
        }
        // Knob
        let ky = col_bot - val * unit_h;
        let knob = Rect::from_center_size(Pos2::new(cx, ky), Vec2::new(colw + 10.0, 7.0));
        pp.rect_filled(knob, 0.0, Color32::from_rgb(200, 200, 190));
        pp.rect_filled(Rect::from_min_size(knob.min + Vec2::new(0.0, 5.0), Vec2::new(knob.width(), 2.0)), 0.0, Color32::from_rgb(90, 90, 86));
        if ui_st.selected_sys == i {
            pp.rect_stroke(slot.expand(5.0), CornerRadius::ZERO, Stroke::new(1.0, AMBER), StrokeKind::Outside);
        }
        text(pp, Pos2::new(cx, fz.min.y), Align2::CENTER_TOP, format!("{:.0}", val), mono(24.0), col);
        text(pp, Pos2::new(cx, col_bot + 6.0), Align2::CENTER_TOP, s.name(), pix(10.0), col);
        let readout = match s {
            Sys::Locomotion => format!("EVA {:.0}%", st.evasion * 100.0),
            Sys::Sensors => format!("{:.0}m", st.radar_range),
            Sys::Railgun => if st.rail_rate > 0.0 { format!("{:.1}s", 1.0 / st.rail_rate) } else { "--".into() },
            Sys::Autocannon => format!("{:.0}dps", st.ac_dps),
            Sys::Shields => format!("{:.0}", st.shield_cap),
            Sys::Repair => if mech.repair_target == Section::Uplink { "UPLNK".to_string() } else { format!("{:.1}/s", st.repair_rate) },
            Sys::Cooling => format!("-{:.1}", st.cooling),
        };
        text(pp, Pos2::new(cx, col_bot + 22.0), Align2::CENTER_TOP, readout, mono(16.0), DIM);
        // +/- nudge buttons
        let bw = (fw - 8.0) / 2.0;
        let by = col_bot + 40.0;
        if pbutton(ui, Rect::from_min_size(Pos2::new(cx - bw - 1.0, by), Vec2::new(bw, 18.0)), "-", col, seated, false).clicked() {
            mech.alloc[s.idx()] = (val - 5.0).max(0.0);
            ui_st.selected_sys = i;
        }
        if pbutton(ui, Rect::from_min_size(Pos2::new(cx + 1.0, by), Vec2::new(bw, 18.0)), "+", col, seated, false).clicked() {
            mech.alloc[s.idx()] = (val + 5.0).min(cap);
            ui_st.selected_sys = i;
        }
        if resp.hovered() {
            resp.on_hover_text(s.long_name());
        }
    }

    // --- Heat & shields -------------------------------------------------------------------
    let hz = Rect::from_min_max(Pos2::new(fz.max.x + 10.0, fz.min.y), Pos2::new(inner.max.x, fz.max.y));
    let pp = ui.painter_at(hz.expand(2.0));
    let therm = Rect::from_min_max(Pos2::new(hz.min.x + 4.0, hz.min.y + 22.0), Pos2::new(hz.min.x + 46.0, hz.max.y - 40.0));
    text(&pp, Pos2::new(therm.center().x, hz.min.y), Align2::CENTER_TOP, "HEAT", pix(10.0), RED);
    pp.rect_filled(therm, 0.0, EDGE_LO);
    let hy = |v: f32| therm.max.y - (v / HEAT_MAX) * therm.height();
    pp.rect_filled(Rect::from_min_max(Pos2::new(therm.min.x, hy(HEAT_MAX)), Pos2::new(therm.max.x, hy(HEAT_CRIT))), 0.0, Color32::from_rgb(60, 14, 12));
    let segs = 26;
    for k in 0..segs {
        let v0 = k as f32 / segs as f32 * HEAT_MAX;
        let r = Rect::from_min_max(Pos2::new(therm.min.x + 3.0, hy(v0 + HEAT_MAX / segs as f32) + 1.0), Pos2::new(therm.max.x - 3.0, hy(v0) - 1.0));
        let c = if v0 >= 100.0 { RED } else if v0 >= 80.0 { Color32::from_rgb(255, 120, 40) } else if v0 >= 50.0 { AMBER } else { GREEN };
        pp.rect_filled(r, 0.0, if v0 < mech.heat { c } else { c.gamma_multiply(0.1) });
    }
    for mark in [50.0, 80.0, 100.0] {
        pp.line_segment([Pos2::new(therm.max.x, hy(mark)), Pos2::new(therm.max.x + 6.0, hy(mark))], Stroke::new(2.0, DIM));
        text(&pp, Pos2::new(therm.max.x + 8.0, hy(mark)), Align2::LEFT_CENTER, format!("{mark:.0}"), mono(15.0), DIM);
    }
    let net = st.heat_in - st.heat_out;
    text(&pp, Pos2::new(therm.center().x, therm.max.y + 4.0), Align2::CENTER_TOP, format!("{:.0}", mech.heat), mono(26.0), if mech.heat >= 80.0 { RED } else { TEXT });
    text(&pp, Pos2::new(therm.center().x + 2.0, therm.max.y + 22.0), Align2::CENTER_TOP, format!("{:+.1}/s", net), mono(16.0), if net > 0.0 { AMBER } else { GREEN });
    // Shield column
    let sh = Rect::from_min_max(Pos2::new(hz.max.x - 38.0, therm.min.y), Pos2::new(hz.max.x - 4.0, therm.max.y));
    text(&pp, Pos2::new(sh.center().x, hz.min.y), Align2::CENTER_TOP, "SHLD", pix(10.0), sys_color(Sys::Shields));
    pp.rect_filled(sh, 0.0, EDGE_LO);
    let frac = if st.shield_cap > 0.0 { mech.shield / st.shield_cap } else { 0.0 };
    let fill = Rect::from_min_max(Pos2::new(sh.min.x + 3.0, sh.max.y - frac * sh.height() + 3.0), sh.max - Vec2::new(3.0, 3.0));
    pp.rect_filled(fill, 0.0, if mech.shields_down_t > 0.0 { RED } else { sys_color(Sys::Shields) });
    text(&pp, Pos2::new(sh.center().x, sh.max.y + 4.0), Align2::CENTER_TOP, format!("{:.0}", mech.shield), mono(22.0), sys_color(Sys::Shields));

    // --- Emergency row --------------------------------------------------------------------
    let er = Rect::from_min_max(Pos2::new(inner.min.x, inner.max.y - 76.0), inner.max);
    let bw = (er.width() - 4.0 * 6.0) / 5.0;
    let b = |i: usize, h: f32, y: f32| Rect::from_min_size(Pos2::new(er.min.x + i as f32 * (bw + 6.0), er.min.y + y), Vec2::new(bw, h));
    let vent_lbl = if mech.vent_cd > 0.0 { format!("VENT {:.0}s", mech.vent_cd) } else { "VENT [V]".into() };
    if pbutton(ui, b(0, 36.0, 0.0), &vent_lbl, CYAN, mech.vent_cd <= 0.0 && seated, mech.vent_t > 0.0).clicked() {
        cmds.vent = true;
    }
    let scram_lbl = if mech.scram_cd > 0.0 { format!("SCRAM {:.0}s", mech.scram_cd) } else { "SCRAM [X]".into() };
    if pbutton(ui, b(1, 36.0, 0.0), &scram_lbl, AMBER, mech.scram_cd <= 0.0 && seated, mech.scram_t > 0.0).clicked() {
        cmds.scram = true;
    }
    // Routing presets
    let presets: [(&str, [f32; 7]); 2] = [("ASSAULT", [5.0, 10.0, 40.0, 15.0, 10.0, 0.0, 25.0]), ("RESTORE", [10.0, 10.0, 10.0, 15.0, 20.0, 25.0, 15.0])];
    for (k, (name, values)) in presets.iter().enumerate() {
        if pbutton(ui, b(2 + k, 36.0, 0.0), name, GREEN, seated, false).clicked() {
            mech.alloc = *values;
        }
    }
    let eject_on = mech.overload.is_some();
    if pbutton(ui, b(4, 36.0, 0.0), "EJECT", RED, eject_on, eject_on && (scene.t * 3.0).fract() < 0.5).clicked() {
        cmds.eject = true;
    }
    let hint = "1-7 select bus · ↑/↓ ±5 · drag faders · past the white line = overdrive heat";
    text(&p, Pos2::new(er.min.x, er.max.y - 2.0), Align2::LEFT_BOTTOM, hint, mono(16.0), DIM);
    if let Some(t) = mech.overload {
        let big = Rect::from_center_size(fz.center(), Vec2::new(fz.width() - 20.0, 70.0));
        let pp = ui.painter();
        pp.rect_filled(big, 0.0, Color32::from_black_alpha(220));
        pp.rect_stroke(big, CornerRadius::ZERO, Stroke::new(3.0, RED), StrokeKind::Inside);
        text(pp, big.center() - Vec2::new(0.0, 14.0), Align2::CENTER_CENTER, "REACTOR OVERLOAD", pix(16.0), RED);
        text(pp, big.center() + Vec2::new(0.0, 16.0), Align2::CENTER_CENTER, format!("CONTAINMENT FAILURE IN {:.1}s", t.max(0.0)), mono(26.0), if (scene.t * 4.0).fract() < 0.5 { Color32::WHITE } else { RED });
    }
}

// --------------------------------------------------------------------------------------
// Hull diagram
// --------------------------------------------------------------------------------------

fn hull_panel(ui: &mut Ui, rect: Rect, art: &Art, mech: &mut Mech, cmds: &mut PilotCommands, campaign: &Campaign, scene: &Scene) {
    let inner = panel_frame(ui, rect, "DAMAGE CONTROL", Some(campaign.makers[Section::Cockpit.idx()]));
    let s = ((inner.height() - 4.0) / art::HULL_H as f32).floor().min(4.0);
    let dr = Rect::from_min_size(inner.min + Vec2::new(0.0, 2.0), Vec2::new(art::HULL_W as f32 * s, art::HULL_H as f32 * s));
    let p = ui.painter_at(dr.expand(4.0));
    p.rect_filled(dr.expand(3.0), 0.0, Color32::from_rgb(8, 14, 18));
    // blueprint grid
    let mut gx = dr.min.x;
    while gx < dr.max.x {
        p.line_segment([Pos2::new(gx, dr.min.y), Pos2::new(gx, dr.max.y)], Stroke::new(1.0, Color32::from_rgb(16, 30, 38)));
        gx += 4.0 * s;
    }
    let mut gy = dr.min.y;
    while gy < dr.max.y {
        p.line_segment([Pos2::new(dr.min.x, gy), Pos2::new(dr.max.x, gy)], Stroke::new(1.0, Color32::from_rgb(16, 30, 38)));
        gy += 4.0 * s;
    }
    let t = scene.t;
    let hover_pos = ui.ctx().pointer_hover_pos();
    let mut hovered: Option<Section> = None;
    let map = &art.hull;
    let at = |x: i32, y: i32| -> u8 {
        if x < 0 || y < 0 || x >= art::HULL_W as i32 || y >= art::HULL_H as i32 { 254 } else { map[y as usize * art::HULL_W + x as usize] }
    };
    let mut centroid = [(0.0f32, 0.0f32, 0u32); 10];
    for y in 0..art::HULL_H as i32 {
        for x in 0..art::HULL_W as i32 {
            let v = at(x, y);
            if v == 254 {
                continue;
            }
            let edge = at(x - 1, y) != v || at(x + 1, y) != v || at(x, y - 1) != v || at(x, y + 1) != v;
            let hi = at(x, y - 1) != v || at(x - 1, y) != v;
            let r = Rect::from_min_size(dr.min + Vec2::new(x as f32 * s, y as f32 * s), Vec2::splat(s));
            let mut col = if v == 255 {
                Color32::from_rgb(54, 60, 68)
            } else {
                let sec = Section::ALL[v as usize];
                centroid[v as usize].0 += x as f32;
                centroid[v as usize].1 += y as f32;
                centroid[v as usize].2 += 1;
                if hover_pos.is_some_and(|hp| r.contains(hp)) {
                    hovered = Some(sec);
                }
                let f = if sec == Section::Uplink { mech.uplink() / 100.0 } else { mech.frac(sec) };
                let mut c = if sec == Section::Uplink {
                    if (y as f32) / 7.0 > 1.0 - f { CYAN } else { Color32::from_rgb(30, 60, 70) }
                } else {
                    health_color(f)
                };
                if f <= 0.0 && sec != Section::Uplink && (t * 2.0).fract() < 0.5 {
                    c = Color32::from_rgb(20, 10, 10);
                }
                if sec == mech.repair_target && (t * 2.0).fract() < 0.5 {
                    c = c.gamma_multiply(1.3);
                }
                // Crack pattern on damaged parts
                if sec != Section::Uplink && f < 0.6 && hash2(x, y, 4).is_multiple_of(7) {
                    c = c.gamma_multiply(0.4);
                }
                c
            };
            if edge {
                col = if hi { col.gamma_multiply(1.15) } else { col.gamma_multiply(0.45) };
            } else {
                col = col.gamma_multiply(0.78);
            }
            p.rect_filled(r, 0.0, col);
        }
    }
    let cen = |sec: Section| {
        let c = centroid[sec.idx()];
        dr.min + Vec2::new((c.0 / c.2.max(1) as f32 + 0.5) * s, (c.1 / c.2.max(1) as f32 + 0.5) * s)
    };
    // Boarders + incoming markers
    for sec in Section::ALL {
        let n = scene.boarders_on(sec);
        for i in 0..n {
            let off = Vec2::new((i as f32 - (n as f32 - 1.0) / 2.0) * 10.0, 0.0);
            if (t * 5.0 + i as f64).fract() < 0.75 {
                art.sprites.tick.paint(&p, cen(sec) + off - Vec2::new(5.0, 4.0), 1.0, 0.0, None, false);
            }
        }
        if let Some(sh) = scene.shell_on(sec) {
            let c = cen(sec);
            let rr = 6.0 + (1.0 - sh.t / sh.flight) * 12.0;
            if (t * 6.0).fract() < 0.6 {
                p.circle_stroke(c, rr, Stroke::new(2.0, RED));
                p.line_segment([c - Vec2::new(rr + 3.0, 0.0), c + Vec2::new(rr + 3.0, 0.0)], Stroke::new(1.0, RED));
                p.line_segment([c - Vec2::new(0.0, rr + 3.0), c + Vec2::new(0.0, rr + 3.0)], Stroke::new(1.0, RED));
            }
        }
    }
    if let Pilot::Working { at, .. } | Pilot::Crawling { to: at, .. } = mech.pilot {
        art.sprites.pilot.paint(&p, cen(at) - Vec2::new(4.0, 6.0), 1.0, 0.0, None, false);
    }
    let dresp = ui.interact(dr, ui.id().with("hulldiag"), Sense::click());
    if let Some(h) = hovered {
        if dresp.clicked() {
            mech.repair_target = h;
        }
        dresp.on_hover_text(format!("{} — click to send repair drones", h.name()));
    }

    // --- Section list ---------------------------------------------------------------------
    let lr = Rect::from_min_max(Pos2::new(dr.max.x + 10.0, inner.min.y), inner.max);
    let rows: [Section; 10] = [
        Section::Uplink,
        Section::Reactor,
        Section::Cockpit,
        Section::Sensors,
        Section::Coolant,
        Section::ShieldGen,
        Section::Railgun,
        Section::Autocannon,
        Section::LeftLeg,
        Section::RightLeg,
    ];
    let rh = (lr.height() / rows.len() as f32).min(20.0);
    for (i, sec) in rows.iter().copied().enumerate() {
        let row = Rect::from_min_size(Pos2::new(lr.min.x, lr.min.y + i as f32 * rh), Vec2::new(lr.width(), rh - 1.0));
        let resp = ui.interact(row, ui.id().with(("hrow", i)), Sense::click());
        let pp = ui.painter();
        let is_rt = mech.repair_target == sec;
        if is_rt {
            pp.rect_filled(row, 0.0, Color32::from_rgb(48, 34, 18));
        } else if resp.hovered() {
            pp.rect_filled(row, 0.0, Color32::from_rgb(30, 32, 36));
        }
        if resp.clicked() {
            mech.repair_target = sec;
        }
        let (f, maxf) = if sec == Section::Uplink { (mech.uplink() / 100.0, 1.0) } else { (mech.hp[sec.idx()] / 100.0, mech.max_hp[sec.idx()] / 100.0) };
        let c = if sec == Section::Uplink { CYAN } else { health_color(mech.frac(sec)) };
        text(pp, row.left_center() + Vec2::new(2.0, 0.0), Align2::LEFT_CENTER, if is_rt { "▶" } else { " " }, mono(16.0), AMBER);
        text(pp, row.left_center() + Vec2::new(14.0, 0.0), Align2::LEFT_CENTER, sec.short(), pix(9.0), c);
        let bar = Rect::from_min_size(Pos2::new(row.min.x + 66.0, row.center().y - 4.0), Vec2::new(row.width() - 140.0, 8.0));
        pp.rect_filled(bar, 0.0, EDGE_LO);
        pp.rect_filled(Rect::from_min_size(bar.min, Vec2::new(bar.width() * f.clamp(0.0, 1.2) / 1.2, bar.height())), 0.0, c);
        if maxf < 1.0 {
            // Scarred capacity: permanently lost
            let sx = bar.min.x + bar.width() * maxf / 1.2;
            let ex = bar.min.x + bar.width() / 1.2;
            let mut hx = sx;
            while hx < ex {
                pp.line_segment([Pos2::new(hx, bar.max.y), Pos2::new((hx + 3.0).min(ex), bar.min.y)], Stroke::new(1.0, RED.gamma_multiply(0.7)));
                hx += 3.0;
            }
        }
        let mut tag = format!("{:3.0}%", f * 100.0);
        let nb = scene.boarders_on(sec);
        if nb > 0 {
            tag = format!("{tag} T{nb}");
        }
        text(pp, Pos2::new(row.max.x - 30.0, row.center().y), Align2::RIGHT_CENTER, tag, mono(17.0), if nb > 0 { PURPLE } else { TEXT });
        if sec != Section::Cockpit {
            let go = Rect::from_min_size(Pos2::new(row.max.x - 26.0, row.min.y + 1.0), Vec2::new(26.0, rh - 3.0));
            if pbutton(ui, go, "GO", PURPLE, mech.pilot.in_seat(), false).on_hover_text("Leave the chair and crawl here").clicked() {
                cmds.crawl_to = Some(sec);
            }
        }
    }
    let _ = (PI, campaign);
}

// --------------------------------------------------------------------------------------
// Radio
// --------------------------------------------------------------------------------------

fn voice_style(v: Voice) -> (&'static str, Color32) {
    match v {
        Voice::Control => ("CITY CONTROL", CYAN),
        Voice::Chief => ("CHIEF ODA", AMBER),
        Voice::Pilot => ("YOU", GREEN),
        Voice::Enemy => ("INTERCEPT", RED),
        Voice::System => ("BASTION OS", Color32::from_rgb(170, 180, 170)),
    }
}

fn radio_panel(ui: &mut Ui, rect: Rect, art: &Art, radio: &Radio, animate: bool) {
    let inner = panel_frame(ui, rect, "RADIO / SHIP LOG", None);
    let p = ui.painter_at(inner);
    let last = radio.log.last();
    let face_r = Rect::from_min_size(inner.min, Vec2::new(112.0, inner.height()));
    p.rect_filled(face_r, 0.0, PANEL_DARK);
    if let Some(m) = last {
        let sprite = match m.voice {
            Voice::Control => &art.sprites.control_face,
            Voice::Chief => &art.sprites.chief_face,
            Voice::Pilot => &art.sprites.pilot_face,
            Voice::Enemy => &art.sprites.enemy_face,
            Voice::System => &art.sprites.system_face,
        };
        let (name, col) = voice_style(m.voice);
        let fr = Rect::from_min_size(face_r.min + Vec2::new(8.0, 8.0), Vec2::splat(96.0));
        p.rect_filled(fr, 0.0, col.gamma_multiply(0.15));
        sprite.paint(&p, fr.min, 6.0, 0.0, None, false);
        scanlines(&p, fr, 50);
        // Talking indicator
        let talking = radio.time - m.time < 3.0;
        text(&p, Pos2::new(face_r.center().x, fr.max.y + 6.0), Align2::CENTER_TOP, name, pix(9.0), col);
        if talking {
            for i in 0..6 {
                let hgt = ((radio.time * 13.0 + i as f32 * 1.7).sin().abs() * 12.0) + 2.0;
                p.rect_filled(Rect::from_min_size(Pos2::new(face_r.min.x + 26.0 + i as f32 * 10.0, fr.max.y + 40.0 - hgt), Vec2::new(6.0, hgt)), 0.0, col);
            }
        }
    }
    let log_r = Rect::from_min_max(Pos2::new(face_r.max.x + 8.0, inner.min.y), inner.max);
    p.rect_filled(log_r, 0.0, Color32::from_rgb(8, 12, 9));
    child(ui, log_r.shrink(6.0), |ui| {
        egui::ScrollArea::vertical().stick_to_bottom(true).auto_shrink([false, false]).show(ui, |ui| {
            let n = radio.log.len();
            for (i, m) in radio.log.iter().enumerate() {
                let (name, col) = voice_style(m.voice);
                let mut body = m.text.clone();
                if i + 1 == n && animate {
                    // Typewriter on the newest line
                    let chars = ((radio.time - m.time) * 70.0).max(0.0) as usize;
                    body = body.chars().take(chars.max(1)).collect();
                }
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.label(egui::RichText::new(format!("T+{}", fmt_clock(m.time))).color(DIM).size(18.0));
                    ui.label(egui::RichText::new(format!("[{name}]")).color(col).size(18.0));
                    ui.label(egui::RichText::new(body).color(if i + 1 == n { Color32::WHITE } else { TEXT }).size(19.0));
                });
            }
        });
    });
    scanlines(&p, log_r, 22);
}

// --------------------------------------------------------------------------------------
// Overlays: briefing, pause, outcome
// --------------------------------------------------------------------------------------

fn modal(ctx: &egui::Context, id: &str, size: Vec2, border: Color32) -> Ui {
    let screen = ctx.content_rect();
    let mut ui = Ui::new(
        ctx.clone(),
        egui::Id::new(id),
        UiBuilder::new().layer_id(LayerId::new(Order::Foreground, egui::Id::new(id))).max_rect(screen),
    );
    ui.painter().rect_filled(screen, 0.0, Color32::from_black_alpha(170));
    let r = Rect::from_center_size(screen.center(), size);
    let p = ui.painter();
    p.rect_filled(r.expand(4.0), 0.0, EDGE_LO);
    p.rect_filled(r, 0.0, PANEL);
    p.rect_stroke(r, CornerRadius::ZERO, Stroke::new(2.0, border), StrokeKind::Inside);
    let mut c = ui.new_child(UiBuilder::new().max_rect(r.shrink(24.0)).layout(egui::Layout::top_down(egui::Align::Min)));
    c.set_clip_rect(r);
    // Block clicks to the cockpit underneath.
    let _ = ui.interact(screen, egui::Id::new((id, "blocker")), Sense::click());
    c
}

fn overlays(ctx: &egui::Context, art: &Art, mech: &Mech, mission: &mut Mission, campaign: &Campaign, flow: &mut Flow, scene: &Scene) {
    if mission.briefing {
        let mut ui = modal(ctx, "briefing", Vec2::new(760.0, 520.0), AMBER);
        let r = ui.max_rect();
        let p = ui.painter().clone();
        text(&p, r.min, Align2::LEFT_TOP, format!("OPERATION {:02} // BRIEFING", mission.number), pix(18.0), AMBER);
        let face = Rect::from_min_size(Pos2::new(r.max.x - 84.0, r.min.y + 30.0), Vec2::splat(84.0));
        p.rect_filled(face, 0.0, CYAN.gamma_multiply(0.15));
        art.sprites.control_face.paint(&p, face.min + Vec2::new(2.0, 2.0), 5.0, 0.0, None, false);
        scanlines(&p, face, 50);
        ui.add_space(36.0);
        ui.set_max_width(r.width() - 100.0);
        let body = [
            ("CITY CONTROL", CYAN, "Bastion, the city's uplink relay on your mast is down. Without it the shield grid stays dark and the last districts freeze tonight."),
            ("CHIEF ODA", AMBER, "Route REPAIR power to the UPLINK and hold position. The reactor makes 100 units and you'll want 200. Pick."),
            ("CHIEF ODA", AMBER, "Railgun hits hard and cooks the core. Past 100 heat the containment goes in 12 seconds. VENT dumps heat but drops shields; SCRAM kills everything for 6."),
            ("CITY CONTROL", CYAN, "Expect Jackal scouts, Mortar Crabs sitting out past your radar, and Tick Carriers. Their drones will board you and drink your power."),
        ];
        for (who, c, line) in body {
            ui.label(egui::RichText::new(who).color(c).size(20.0));
            ui.add(egui::Label::new(egui::RichText::new(line).size(21.0).color(TEXT)).wrap());
            ui.add_space(6.0);
        }
        ui.add_space(4.0);
        ui.label(egui::RichText::new("CLICK radar contacts to target · SPACE fire railgun · V vent · X scram · E ECM · TAB cycle targets · P pause").size(18.0).color(DIM));
        let br = Rect::from_min_size(Pos2::new(r.center().x - 140.0, r.max.y - 44.0), Vec2::new(280.0, 44.0));
        if pbutton(&mut ui, br, "BEGIN OPERATION", AMBER, true, true).clicked() {
            mission.briefing = false;
        }
        return;
    }
    if let Some(outcome) = mission.outcome {
        let (title, sub, col) = match outcome {
            Outcome::Victory => ("UPLINK RESTORED", "The shield grid flickers on across the city. Bastion limps home.", CYAN),
            Outcome::Meltdown => ("REACTOR OVERLOAD", "Containment failed. WF-7 BASTION is lost with her pilot.", RED),
            Outcome::Ejected => ("PILOT EJECTED", "The core went critical seconds after you cleared the hatch. Bastion is gone.", RED),
        };
        let mut ui = modal(ctx, "outcome", Vec2::new(700.0, 380.0), col);
        let r = ui.max_rect();
        let p = ui.painter().clone();
        if outcome != Outcome::Victory {
            // Meltdown bloom
            let screen = ctx.content_rect();
            let a = ((scene.t * 2.0).sin() * 0.5 + 0.5) as f32;
            p.rect_filled(Rect::from_min_size(screen.min, Vec2::new(screen.width(), 6.0)), 0.0, RED.gamma_multiply(a));
        }
        text(&p, Pos2::new(r.center().x, r.min.y + 10.0), Align2::CENTER_TOP, title, pix(30.0), col);
        text(&p, Pos2::new(r.center().x, r.min.y + 64.0), Align2::CENTER_TOP, sub, mono(24.0), TEXT);
        let stats = [
            ("MISSION TIME", fmt_clock(mission.time)),
            ("CONTACTS DESTROYED", format!("{}", mech.kills)),
            ("PEAK CORE HEAT", format!("{:.0}", mech.heat_peak)),
            ("UPLINK", format!("{:.0}%", mech.uplink())),
        ];
        let mut y = r.min.y + 120.0;
        for (k, v) in stats {
            text(&p, Pos2::new(r.min.x + 120.0, y), Align2::LEFT_TOP, k, mono(24.0), DIM);
            text(&p, Pos2::new(r.max.x - 120.0, y), Align2::RIGHT_TOP, v, mono(24.0), TEXT);
            y += 28.0;
        }
        let br = Rect::from_min_size(Pos2::new(r.center().x - 160.0, r.max.y - 46.0), Vec2::new(320.0, 46.0));
        let label = if outcome == Outcome::Victory { "RETURN TO HANGAR" } else { "NEW CAMPAIGN" };
        if pbutton(&mut ui, br, label, col, true, true).clicked() {
            flow.request = Some(if outcome == Outcome::Victory { FlowRequest::ToHangar } else { FlowRequest::NewCampaign });
        }
        let _ = campaign;
        return;
    }
    if mission.paused {
        let mut ui = modal(ctx, "pause", Vec2::new(420.0, 160.0), DIM);
        let r = ui.max_rect();
        let p = ui.painter().clone();
        text(&p, Pos2::new(r.center().x, r.min.y + 4.0), Align2::CENTER_TOP, "PAUSED", pix(24.0), AMBER);
        let br = Rect::from_min_size(Pos2::new(r.center().x - 100.0, r.max.y - 44.0), Vec2::new(200.0, 40.0));
        if pbutton(&mut ui, br, "RESUME [P]", AMBER, true, true).clicked() {
            mission.paused = false;
        }
    }
}
