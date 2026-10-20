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
    Single,
    Quad,
    Interior,
}

#[derive(Default)]
pub struct CockpitUi {
    pub cam_mode: CamMode,
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
    let top = Rect::from_min_size(r.min + Vec2::new(8.0, 6.0), Vec2::new(w - 16.0, 44.0));
    let main_y0 = r.min.y + 56.0;
    let main_y1 = r.max.y - 6.0;
    let left_w = 330.0;
    let right_w = 236.0;
    let left = Rect::from_min_max(Pos2::new(r.min.x + 8.0, main_y0), Pos2::new(r.min.x + 8.0 + left_w, main_y1));
    let right = Rect::from_min_max(Pos2::new(r.max.x - 8.0 - right_w, main_y0), Pos2::new(r.max.x - 8.0, main_y1));
    let center = Rect::from_min_max(Pos2::new(left.max.x + 8.0, main_y0), Pos2::new(right.min.x - 8.0, main_y1));
    // The camera wall is the hero: as tall as the full-width feed needs, the hotbar takes the rest.
    let feed_h = ((center.width() - 20.0) / FEED_W * FEED_H + 38.0).min(center.height() - 136.0);
    let feed = Rect::from_min_size(center.min, Vec2::new(center.width(), feed_h));
    let hotbar = Rect::from_min_max(Pos2::new(center.min.x, feed.max.y + 8.0), center.max);
    let radar_h = left.width() + 30.0;
    let radar = Rect::from_min_size(left.min, Vec2::new(left.width(), radar_h));
    let log = Rect::from_min_max(Pos2::new(left.min.x, radar.max.y + 8.0), left.max);

    top_bar(&mut root, top, &art, &mech, &mut mission, &scene);
    radar_panel(&mut root, radar, &mut mech, &campaign, &scene);
    log_panel(&mut root, log, &art, &radio, mission.running());
    feed_panel(&mut root, feed, &art, &mut mech, &fx, &mut cmds, &campaign, &scene, &mut st);
    hotbar_panel(&mut root, hotbar, &art, &mut mech, &mut cmds, &campaign, &scene);
    hull_panel(&mut root, right, &art, &mut mech, &mut cmds, &campaign, &scene);
    overload_banner(&mut root, feed, &mech, &scene);

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
        let left = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
        mech.active_cam = (mech.active_cam + if left { CAMERAS - 1 } else { 1 }) % CAMERAS;
    }
    if keys.just_pressed(KeyCode::KeyC) {
        mech.quad_view = !mech.quad_view;
    }
    if keys.just_pressed(KeyCode::KeyT) {
        let mut vis: Vec<&(Entity, Enemy)> = scene.enemies.iter().filter(|(_, e)| e.dist <= mech.stats.radar_range).collect();
        vis.sort_by(|a, b| a.1.dist.total_cmp(&b.1.dist));
        if !vis.is_empty() {
            let i = mech.target.and_then(|t| vis.iter().position(|(e, _)| *e == t)).map(|i| i + 1).unwrap_or(0);
            mech.target = Some(vis[i % vis.len()].0);
        }
    }
    // Hotbar
    let seated = mech.pilot.in_seat();
    if keys.just_pressed(KeyCode::Digit1) {
        cmds.fire_rail = true;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        mech.weapons_free = !mech.weapons_free;
    }
    if keys.just_pressed(KeyCode::Digit3) {
        cmds.ecm = true;
    }
    if keys.just_pressed(KeyCode::Digit4) {
        cmds.vent = true;
    }
    if keys.just_pressed(KeyCode::Digit5) {
        cmds.scram = true;
    }
    if keys.just_pressed(KeyCode::Digit6) && mech.overload.is_some() {
        cmds.eject = true;
    }
    if keys.just_pressed(KeyCode::Digit7) {
        if seated {
            cmds.crawl_to = Some(crew_destination(mech, scene));
        } else if matches!(mech.pilot, Pilot::Working { .. }) {
            cmds.return_to_seat = true;
        }
    }
    let _ = st;
}

// --------------------------------------------------------------------------------------
// Top bar
// --------------------------------------------------------------------------------------

fn top_bar(ui: &mut Ui, rect: Rect, art: &Art, mech: &Mech, mission: &mut Mission, scene: &Scene) {
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
    art.sprites.crest.paint(&p, logo.left_top() + Vec2::new(6.0, 3.0), 2.0, 0.0, Some(Color32::from_rgb(40, 32, 24)), false);
    text(&p, logo.left_center() + Vec2::new(34.0, 0.0), Align2::LEFT_CENTER, "HANGAR ZERO", pix(16.0), Color32::from_rgb(20, 16, 10));
    text(&p, logo.right_center() + Vec2::new(10.0, 0.0), Align2::LEFT_CENTER, format!("WF-7 BASTION // OP-{:02}", mission.number), pix(10.0), AMBER);

    // Mission clock
    let clock_x = rect.min.x + 470.0;
    text(&p, Pos2::new(clock_x, rect.center().y), Align2::LEFT_CENTER, format!("T+{}", fmt_clock(mission.time)), mono(34.0), GREEN);

    // Objective bar
    let ob = Rect::from_min_size(Pos2::new(rect.min.x + 590.0, rect.min.y + 8.0), Vec2::new(420.0, 28.0));
    text(&p, ob.left_top() + Vec2::new(0.0, -1.0), Align2::LEFT_TOP, "UPLINK", pix(9.0), CYAN);
    let bar = Rect::from_min_max(ob.min + Vec2::new(0.0, 11.0), ob.max);
    seg_bar(&p, bar, mech.uplink() / 100.0, 40, CYAN);
    text(&p, Pos2::new(ob.max.x + 8.0, ob.center().y + 4.0), Align2::LEFT_CENTER, format!("{:3.0}%", mech.uplink()), mono(28.0), CYAN);
    if mech.repair_target != Section::Uplink && mech.uplink() < 100.0 {
        text(&p, Pos2::new(ob.max.x, ob.min.y - 1.0), Align2::RIGHT_TOP, "REPAIR DRONES ELSEWHERE", pix(8.0), AMBER);
    }

    // Alarm lamps
    let lamps: [(&str, bool, Color32); 6] = [
        ("CAM", mech.cameras_up() < mech.cams_at_start, AMBER),
        ("OVLD", mech.overload.is_some(), RED),
        ("HEAT", mech.heat > 80.0, AMBER),
        ("BREACH", mech.breach.is_some() || mech.interlocks_failed, RED),
        ("BOARD", !scene.boarders.is_empty(), PURPLE),
        ("INCOMING", !scene.shells.is_empty(), YELLOW),
    ];
    let mut x = rect.max.x - 6.0 * 74.0 - 60.0;
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
    let size = inner.width().min(inner.height());
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
    // Ground returns: the city itself, as faint static clutter that the sweep lights up.
    for i in 0..90 {
        let h = hash2(i, 0, 61);
        let a = (h % 6283) as f32 / 1000.0;
        let d = 120.0 + ((h >> 13) % 700) as f32;
        if d > st.radar_range {
            continue;
        }
        let since = (mech.sweep - a).rem_euclid(TAU);
        let glow = 0.12 + 0.25 * (1.0 - since / TAU);
        let pt = polar(c, k, a, d);
        let w = 2.0 + ((h >> 24) % 3) as f32;
        p.rect_filled(Rect::from_min_size(Pos2::new(pt.x.round(), pt.y.round()), Vec2::new(w, 2.0)), 0.0, GREEN.gamma_multiply(glow));
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

    // Quadrants with no working camera: radar-only.
    for cam in 0..CAMERAS {
        if mech.camera_ok(cam) {
            continue;
        }
        let a0 = camera_bearing(cam) - CAMERA_FOV / 2.0;
        let mut pts = vec![c];
        for i in 0..=12 {
            pts.push(polar(c, 1.0, a0 + CAMERA_FOV * i as f32 / 12.0, rad));
        }
        p.add(egui::Shape::convex_polygon(pts, Color32::from_rgba_unmultiplied(120, 30, 30, 50), Stroke::NONE));
        let lp = polar(c, 1.0, camera_bearing(cam), rad * 0.62);
        if (scene.t * 1.2).fract() < 0.7 {
            text(&p, lp, Align2::CENTER_CENTER, "NO CAM", pix(8.0), RED.gamma_multiply(0.8));
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
        // Blips sit where the radar thinks the contact is.
        let pos = polar(c, k, e.est_angle, e.est_dist.min(RADAR_SCALE_M));
        let since = (mech.sweep - e.est_angle).rem_euclid(TAU);
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
        let identified = e.dist <= st.id_range || e.covered;
        let dimmer = if e.covered { 1.0 } else { 0.7 };
        if identified {
            let col = kind_color(e.kind).gamma_multiply(glow * dimmer);
            paint_icon(&p, pos, kind_icon(e.kind), 2.0, if e.hit_flash > 0.0 { Color32::WHITE } else { col });
            text(&p, pos + Vec2::new(10.0, -10.0), Align2::LEFT_BOTTOM, format!("{}-{:02}", e.kind.callsign(), e.serial), mono(16.0), col);
        } else {
            paint_icon(&p, pos, &ICON_UNKNOWN, 2.0, AMBER.gamma_multiply(glow * dimmer));
        }
        if !e.covered {
            // Uncertainty: the blip could be anywhere on this arc.
            let er = (e.err * e.est_dist * k).max(5.0);
            pixel_ring(&p, pos, er, AMBER.gamma_multiply(0.45), true);
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
    // Bastion from above: torso, shoulders, the rail arm and the mast.
    const SELF_ICON: [&str; 9] = ["....x....", "....x....", ".xx.x.xx.", "xxxxxxxxx", "xxxxxxxxx", "xxxxxxxxx", ".xx...xx.", "....xxxx.", "......xx."];
    for (y, row) in SELF_ICON.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if ch == 'x' {
                let min = c + Vec2::new((x as f32 - 4.5) * 2.0, (y as f32 - 4.5) * 2.0);
                p.rect_filled(Rect::from_min_size(Pos2::new(min.x.round(), min.y.round()), Vec2::splat(2.0)), 0.0, AMBER);
            }
        }
    }

    text(&p, rr.left_bottom() + Vec2::new(4.0, -4.0), Align2::LEFT_BOTTOM, format!("DET {:.0}m", st.radar_range), mono(18.0), GREEN);
    if square {
        text(&p, rr.right_top() + Vec2::new(-4.0, 2.0), Align2::RIGHT_TOP, "JACKAL FIRMWARE", mono(16.0), rgb(Maker::Jackal.accent()));
    }

    let _ = ui;
}

// --------------------------------------------------------------------------------------
// External camera / interior cutaway
// --------------------------------------------------------------------------------------

const FEED_W: f32 = 320.0;
const FEED_H: f32 = 208.0;

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
    if mech.quad_view && st.cam_mode == CamMode::Single {
        st.cam_mode = CamMode::Quad;
    } else if !mech.quad_view && st.cam_mode == CamMode::Quad {
        st.cam_mode = CamMode::Single;
    }
    let interior = st.cam_mode == CamMode::Interior || !mech.pilot.in_seat();
    let title = if interior {
        "INTERIOR CUTAWAY".to_string()
    } else if st.cam_mode == CamMode::Quad {
        format!("CAMERA WALL — {} OF {} FEEDS LIVE", mech.cameras_up(), CAMERAS)
    } else {
        format!("CAMERA {} · {} {:03.0}", mech.active_cam + 1, CAMERA_NAMES[mech.active_cam], camera_bearing(mech.active_cam).to_degrees())
    };
    let inner = panel_frame(ui, rect, &title, None);

    // Mode tabs in the header
    let tabs = [("SINGLE", CamMode::Single), ("QUAD [C]", CamMode::Quad), ("INT", CamMode::Interior)];
    let mut x = rect.max.x - 12.0 - 3.0 * 64.0;
    for (lbl, mode) in tabs {
        if pbutton(ui, Rect::from_min_size(Pos2::new(x, rect.min.y + 3.0), Vec2::new(60.0, 18.0)), lbl, GREEN, true, st.cam_mode == mode).clicked() {
            st.cam_mode = mode;
            mech.quad_view = mode == CamMode::Quad;
        }
        x += 64.0;
    }
    // Camera selectors
    let target_cam = mech.target.and_then(|t| scene.enemies.iter().find(|(e, _)| *e == t)).map(|(_, e)| camera_for(e.est_angle));
    if !interior {
        let mut x = rect.min.x + 14.0 + title.len() as f32 * 8.6 + 30.0;
        for cam in 0..CAMERAS {
            let ok = mech.camera_ok(cam);
            let col = if ok { GREEN } else { RED };
            let lbl = format!("{} {:03.0}", CAMERA_NAMES[cam], camera_bearing(cam).to_degrees());
            let br = Rect::from_min_size(Pos2::new(x, rect.min.y + 3.0), Vec2::new(62.0, 18.0));
            if pbutton(ui, br, &lbl, col, true, mech.active_cam == cam).clicked() {
                mech.active_cam = cam;
            }
            // The selected target sits in this quadrant.
            if target_cam == Some(cam) && (scene.t * 3.0).fract() < 0.7 {
                let c = Pos2::new(br.max.x - 7.0, br.min.y - 1.0);
                ui.painter().add(egui::Shape::convex_polygon(
                    vec![c + Vec2::new(0.0, -4.0), c + Vec2::new(4.0, 0.0), c + Vec2::new(0.0, 4.0), c + Vec2::new(-4.0, 0.0)],
                    Color32::from_rgb(255, 80, 60),
                    Stroke::NONE,
                ));
            }
            x += 66.0;
        }
    }

    let painter_bg = ui.painter_at(inner);
    painter_bg.rect_filled(inner, 0.0, Color32::BLACK);

    if interior {
        let scale = (inner.width() / FEED_W).min(inner.height() / FEED_H).max(1.0);
        let fr = Rect::from_center_size(inner.center(), Vec2::new(FEED_W * scale, FEED_H * scale));
        let p = ui.painter_at(fr);
        interior_view(ui, &p, fr, scale, art, mech, cmds, scene);
        scanlines(&p, fr, 24);
        vignette(&p, fr);
    } else if st.cam_mode == CamMode::Quad {
        let gap = 4.0;
        let cw = (inner.width() - gap) / 2.0;
        let ch = (inner.height() - gap) / 2.0;
        let scale = (cw / FEED_W).min(ch / FEED_H);
        for cam in 0..CAMERAS {
            let (cx, cy) = (cam % 2, cam / 2);
            let cell = Rect::from_min_size(inner.min + Vec2::new(cx as f32 * (cw + gap), cy as f32 * (ch + gap)), Vec2::new(cw, ch));
            let fr = Rect::from_center_size(cell.center(), Vec2::new(FEED_W * scale, FEED_H * scale));
            let p = ui.painter_at(fr);
            cam_view(&p, fr, scale, cam, art, mech, fx, campaign, scene, false);
            scanlines(&p, fr, 24);
            vignette(&p, fr);
            if mech.active_cam == cam {
                p.rect_stroke(fr, CornerRadius::ZERO, Stroke::new(2.0, AMBER), StrokeKind::Inside);
            }
            if ui.interact(fr, ui.id().with(("camcell", cam)), Sense::click()).clicked() {
                mech.active_cam = cam;
            }
        }
    } else {
        let scale = (inner.width() / FEED_W).min(inner.height() / FEED_H).max(1.0);
        let fr = Rect::from_center_size(inner.center(), Vec2::new(FEED_W * scale, FEED_H * scale));
        let p = ui.painter_at(fr);
        cam_view(&p, fr, scale, mech.active_cam, art, mech, fx, campaign, scene, true);
        scanlines(&p, fr, 24);
        vignette(&p, fr);
    }
}

/// Soft edge darkening plus the pixel bezel brackets of the monitor.
fn vignette(p: &egui::Painter, fr: Rect) {
    for i in 0..4 {
        let a = 20 - i * 5;
        p.rect_stroke(fr.shrink(i as f32 * 3.0), CornerRadius::ZERO, Stroke::new(3.0, Color32::from_black_alpha(a as u8 * 4)), StrokeKind::Inside);
    }
    let l = (fr.width() * 0.06).clamp(14.0, 34.0);
    let w = 4.0;
    let bez = Color32::from_rgb(70, 74, 82);
    let hi = Color32::from_rgb(120, 126, 134);
    for (corner, dx, dy) in [(fr.left_top(), 1.0, 1.0), (fr.right_top(), -1.0, 1.0), (fr.left_bottom(), 1.0, -1.0), (fr.right_bottom(), -1.0, -1.0)] {
        let h = Rect::from_two_pos(corner, corner + Vec2::new(l * dx, w * dy));
        let v = Rect::from_two_pos(corner, corner + Vec2::new(w * dx, l * dy));
        p.rect_filled(h, 0.0, bez);
        p.rect_filled(v, 0.0, bez);
        p.rect_filled(Rect::from_two_pos(corner, corner + Vec2::new(l * dx, 1.0 * dy)), 0.0, hi);
        p.rect_filled(Rect::from_two_pos(corner, corner + Vec2::new(1.0 * dx, l * dy)), 0.0, hi);
        // A screw in each bracket
        p.rect_filled(Rect::from_center_size(corner + Vec2::new(8.0 * dx, 8.0 * dy), Vec2::splat(3.0)), 0.0, Color32::from_rgb(30, 32, 36));
    }
}

/// One fixed hull camera, looking down its quadrant.
#[allow(clippy::too_many_arguments)]
fn cam_view(p: &egui::Painter, fr: Rect, s: f32, cam: usize, art: &Art, mech: &Mech, fx: &Effects, campaign: &Campaign, scene: &Scene, single: bool) {
    let bearing = camera_bearing(cam);
    let fov = CAMERA_FOV;
    let px = |x: f32, y: f32| fr.min + Vec2::new(x * s, y * s);
    let frame_i = (scene.t * 20.0) as i32;
    let label = format!("CAM {} · {} {:03.0}", cam + 1, CAMERA_NAMES[cam], bearing.to_degrees());

    // Dead camera: nothing but noise.
    if !mech.camera_ok(cam) {
        p.rect_filled(fr, 0.0, Color32::from_rgb(8, 8, 10));
        for i in 0..1600 {
            let h = hash2(i, frame_i, 17 + cam as u32);
            let x = (h % FEED_W as u32) as f32;
            let y = ((h >> 9) % FEED_H as u32) as f32;
            let v = ((h >> 20) % 70) as u8 + 10;
            // A little chroma bleed in the snow.
            let c = match (h >> 27) % 9 {
                0 => Color32::from_rgb(v + 30, v, v),
                1 => Color32::from_rgb(v, v, v + 40),
                _ => Color32::from_rgb(v, v, v),
            };
            p.rect_filled(Rect::from_min_size(px(x, y), Vec2::new(s * 2.0, s)), 0.0, c);
        }
        // Rolling bars and a tear line.
        for b in 0..3 {
            let band = ((frame_i / 2 + b * 70) % FEED_H as i32) as f32;
            p.rect_filled(Rect::from_min_size(px(0.0, band), Vec2::new(fr.width(), 10.0 * s)), 0.0, Color32::from_white_alpha(10 + b as u8 * 4));
        }
        let tear = ((frame_i * 3) % FEED_H as i32) as f32;
        p.rect_filled(Rect::from_min_size(px(0.0, tear), Vec2::new(fr.width(), s)), 0.0, Color32::from_white_alpha(60));
        if (scene.t * 1.5).fract() < 0.7 {
            text(p, fr.center(), Align2::CENTER_CENTER, "NO SIGNAL", pix(16.0 * s.min(2.0) / 1.3), RED);
        }
        let why = if mech.hp[Section::Sensors.idx()] <= 0.0 { "SENSOR HEAD DESTROYED" } else { "OPTICS DESTROYED — HAND REPAIR AT SENSOR HEAD" };
        text(p, fr.center() + Vec2::new(0.0, 16.0 * s), Align2::CENTER_CENTER, why, mono(14.0 * s.min(2.0)), DIM);
        text(p, fr.min + Vec2::new(10.0, 12.0), Align2::LEFT_CENTER, label, mono(18.0), RED);
        return;
    }

    // Panorama: far layer scrolls at 60% of the bearing for parallax, near layer on top.
    if let Some(tex) = &art.pano_far {
        let u0 = (bearing * 0.6 - fov / 2.0) / TAU;
        p.image(tex.id(), fr, Rect::from_min_max(Pos2::new(u0, 0.0), Pos2::new(u0 + fov / TAU, 1.0)), Color32::WHITE);
    }
    if let Some(tex) = &art.pano_near {
        let u0 = (bearing - fov / 2.0) / TAU;
        p.image(tex.id(), fr, Rect::from_min_max(Pos2::new(u0, 0.0), Pos2::new(u0 + fov / TAU, 1.0)), Color32::WHITE);
    }
    // Drifting ash and embers
    for i in 0..40 {
        let h = hash2(i, cam as i32, 23);
        let speed = 4.0 + (h % 7) as f32;
        let y = ((h >> 8) as f32 * 0.37 + scene.t as f32 * speed) % FEED_H;
        let x = ((h >> 16) as f32 * 0.53 + (scene.t as f32 * 0.7 + i as f32).sin() * 6.0).rem_euclid(FEED_W);
        let ember = h % 9 == 0;
        let c = if ember { Color32::from_rgb(255, 150, 60) } else { Color32::from_rgb(110, 96, 100) };
        p.rect_filled(Rect::from_min_size(px(x.floor(), y.floor()), Vec2::splat(s)), 0.0, c);
    }
    let to_x = |angle: f32| -> Option<f32> {
        let rel = angle_diff(angle, bearing);
        (rel.abs() < fov * 0.6).then(|| FEED_W / 2.0 + rel / (fov / 2.0) * (FEED_W / 2.0))
    };
    // Searchlights sweeping the smoke from two districts, and aviation lights on the towers.
    for (i, (brg, phase)) in [(0.7f32, 0.0f64), (3.6, 2.1), (5.2, 4.4)].iter().enumerate() {
        if let Some(x) = to_x(*brg) {
            let sweep = ((scene.t * 0.25 + phase).sin() as f32) * 0.9;
            let base = px(x, art::HORIZON as f32 - 2.0);
            let tip = base + Vec2::new(sweep * 90.0, -110.0) * s;
            let a = (18.0 + 10.0 * ((scene.t * 0.9 + i as f64).sin() as f32)) as u8;
            p.add(egui::Shape::convex_polygon(
                vec![base + Vec2::new(-2.0 * s, 0.0), base + Vec2::new(2.0 * s, 0.0), tip + Vec2::new(18.0 * s, 0.0), tip - Vec2::new(18.0 * s, 0.0)],
                Color32::from_rgba_unmultiplied(230, 220, 190, a),
                Stroke::NONE,
            ));
        }
    }
    for (i, (brg, h)) in [(0.3f32, 54.0f32), (1.9, 40.0), (2.8, 62.0), (4.1, 36.0), (4.9, 48.0), (5.9, 58.0)].iter().enumerate() {
        if let Some(x) = to_x(*brg)
            && (scene.t * 0.8 + i as f64 * 0.37).fract() < 0.12
        {
            p.rect_filled(Rect::from_min_size(px(x.floor(), art::HORIZON as f32 - h), Vec2::splat(2.0 * s)), 0.0, Color32::from_rgb(255, 70, 50));
        }
    }
    // Low fog drifting across the street.
    for i in 0..70 {
        let h = hash2(i, 5, 41);
        let w = 6.0 + (h % 20) as f32;
        let x = (((h >> 8) % 320) as f32 + scene.t as f32 * (2.0 + (h % 3) as f32) - bearing.to_degrees() * 3.55).rem_euclid(FEED_W + 40.0) - 20.0;
        let y = art::HORIZON as f32 + 4.0 + ((h >> 16) % 30) as f32;
        let a = 14 + ((h >> 20) % 18) as u8;
        p.rect_filled(Rect::from_min_size(px(x, y), Vec2::new(w * s, 2.0 * s)), 0.0, Color32::from_rgba_unmultiplied(150, 120, 130, a));
    }
    let ground_y = |dist: f32| art::HORIZON as f32 + (1.0 - (dist / 1100.0).clamp(0.0, 1.0)).powf(1.6) * 72.0;

    // Enemies, far to near
    let mut list: Vec<&(Entity, Enemy)> = scene.enemies.iter().collect();
    list.sort_by(|a, b| b.1.dist.total_cmp(&a.1.dist));
    for (ent, e) in list {
        let Some(x) = to_x(e.angle) else { continue };
        let moving = e.state == EnemyState::Approach || e.kind == EnemyKind::Scout;
        let walk_frame = ((scene.t * 6.0 + e.serial as f64) as i32) % 2 == 1;
        let sprite: &Sprite = match e.kind {
            EnemyKind::Scout => if moving && walk_frame { &art.sprites.scout_walk } else { &art.sprites.scout },
            EnemyKind::Artillery => &art.sprites.artillery,
            EnemyKind::Carrier => &art.sprites.carrier,
        };
        let k = if e.dist < 230.0 { 3.0 } else if e.dist < 520.0 { 2.0 } else if e.dist < 800.0 { 1.5 } else { 1.0 };
        let gy = ground_y(e.dist);
        // Walk bob, carrier hover, artillery recoil right after it fires
        let bob = match e.kind {
            EnemyKind::Carrier => ((scene.t * 1.6 + e.serial as f64).sin() as f32 * 2.0).round(),
            _ if moving => ((scene.t * 8.0 + e.serial as f64).sin() as f32 * 0.6).round(),
            _ => 0.0,
        };
        let recoil = if e.kind == EnemyKind::Artillery && e.reveal > 2.6 { ((e.reveal - 2.6) * 8.0).round() } else { 0.0 };
        let facing_left = e.orbit < 0.0 && e.kind == EnemyKind::Scout;
        let min = px(x - sprite.w as f32 * k / 2.0 + if facing_left { -recoil } else { recoil }, gy - sprite.h as f32 * k + bob);
        // Ground shadow
        let sw = sprite.w as f32 * k * s * 0.9;
        let shadow = Rect::from_center_size(Pos2::new(px(x, gy).x, px(x, gy).y - 1.0 * s), Vec2::new(sw, 3.0 * s * k.max(1.0) * 0.6));
        p.rect_filled(shadow, 0.0, Color32::from_black_alpha(if e.kind == EnemyKind::Carrier { 60 } else { 90 }));
        // Distance fog: blend toward the sky as contacts get far
        let fog = ((e.dist - 300.0) / 700.0).clamp(0.0, 0.7);
        let haze = (fog > 0.0).then(|| {
            let f = 255.0 - fog * (255.0 - 150.0);
            Color32::from_rgb(f as u8, (255.0 - fog * (255.0 - 120.0)) as u8, (255.0 - fog * (255.0 - 135.0)) as u8)
        });
        sprite.paint(p, Pos2::new(min.x.round(), min.y.round()), k * s, (e.hit_flash * 2.0).min(0.9), haze, facing_left);
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
            let y = 10.0 + f * (FEED_H - 30.0);
            let xx = x + (FEED_W / 2.0 - x) * f;
            p.rect_filled(Rect::from_min_size(px(xx, y), Vec2::splat(2.0 * s)), 0.0, YELLOW);
            p.rect_filled(Rect::from_min_size(px(xx, y - 3.0), Vec2::new(2.0 * s, 3.0 * s)), 0.0, Color32::from_rgba_unmultiplied(255, 150, 60, 120));
        }
    }

    // Where the arms sit in this camera: both forward, one each to the sides, none aft.
    let rail_at = match cam {
        0 => Some((FEED_W - art.sprites.arm_rail.w as f32 + 6.0, false)),
        1 => Some((-6.0, true)),
        _ => None,
    };
    let ac_at = match cam {
        0 => Some((-6.0, true)),
        3 => Some((FEED_W - art.sprites.arm_ac.w as f32 + 6.0, false)),
        _ => None,
    };
    let rail_origin = match rail_at {
        Some((_, false)) => px(FEED_W - 52.0, FEED_H),
        Some((_, true)) => px(52.0, FEED_H),
        None => px(FEED_W / 2.0, FEED_H + 20.0),
    };
    let ac_origin = match ac_at {
        Some((_, true)) => px(52.0, FEED_H),
        Some((_, false)) => px(FEED_W - 52.0, FEED_H),
        None => px(FEED_W / 2.0, FEED_H + 20.0),
    };

    // Effects
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
                if rail_at.is_some() {
                    // Muzzle flash, then a drift of ionised smoke off the rails.
                    if life < 0.3 {
                        let fl = 1.0 - life / 0.3;
                        let r = (6.0 + 10.0 * fl) * s;
                        p.add(egui::Shape::convex_polygon(
                            vec![rail_origin + Vec2::new(-r, 0.0), rail_origin + Vec2::new(0.0, -r * 1.6), rail_origin + Vec2::new(r, 0.0), rail_origin + Vec2::new(0.0, r * 0.4)],
                            Color32::from_rgba_unmultiplied(200, 240, 255, (180.0 * fl) as u8),
                            Stroke::NONE,
                        ));
                        p.circle_filled(rail_origin, r * 0.45, Color32::from_rgba_unmultiplied(255, 255, 255, (220.0 * fl) as u8));
                    }
                    for i in 0..8 {
                        let h = hash2(i, 0, 57);
                        let dx = ((h % 100) as f32 / 50.0 - 1.0) * 6.0 * s * (0.3 + life);
                        let dy = -(4.0 + life * 30.0 + ((h >> 8) % 100) as f32 / 10.0) * s;
                        let a = ((1.0 - life) * 120.0) as u8;
                        p.rect_filled(Rect::from_center_size(rail_origin + Vec2::new(dx, dy), Vec2::splat(s * (1.0 + life * 2.5))), 0.0, Color32::from_rgba_unmultiplied(170, 200, 210, a));
                    }
                }
            }
            FxKind::Tracer => {
                if let Some(t) = target {
                    let a = ac_origin + (t - ac_origin) * (0.2 + life * 0.8);
                    let b = ac_origin + (t - ac_origin) * (life * 0.8);
                    p.line_segment([b, a], Stroke::new(1.5 * s, YELLOW));
                    p.rect_filled(Rect::from_center_size(ac_origin, Vec2::splat(6.0 * s * (1.0 - life))), 0.0, Color32::from_rgb(255, 230, 140));
                }
            }
            FxKind::Explosion | FxKind::Muzzle => {
                if let Some(t) = target {
                    let big = if f.kind == FxKind::Explosion { 1.0 } else { 0.4 };
                    boom(p, t, s * big * (if f.dist < 300.0 { 3.0 } else { 1.6 }), life, hash2(f.angle.to_bits() as i32, 0, 3));
                }
            }
            FxKind::ShellImpact => {
                if let Some(x) = to_x(f.angle) {
                    let base = px(x, FEED_H - 12.0);
                    boom(p, base, s * 4.0, life, 77);
                    // A column of dust and dirt thrown up, widening as it rises.
                    for i in 0..18 {
                        let h = hash2(i, 77, 13);
                        let rise = life * (30.0 + ((h >> 8) % 40) as f32);
                        let dx = ((h % 100) as f32 / 50.0 - 1.0) * (4.0 + rise * 0.5) * s;
                        let a = ((1.0 - life) * 150.0) as u8;
                        let g = 90 + ((h >> 16) % 50) as u8;
                        p.rect_filled(Rect::from_center_size(base + Vec2::new(dx, -rise * s), Vec2::splat(s * (2.0 + life * 3.0))), 0.0, Color32::from_rgba_unmultiplied(g, g - 20, g - 35, a));
                    }
                }
            }
            FxKind::Ecm => {
                let r = life * fr.width();
                p.circle_stroke(fr.center_bottom(), r, Stroke::new(3.0 * s, CYAN.gamma_multiply(1.0 - life)));
            }
        }
    }

    // Cockpit framing: the mech's own arms, faceplates in maker colours.
    let rail_col = rgb(campaign.makers[Section::Railgun.idx()].accent());
    let ac_col = rgb(campaign.makers[Section::Autocannon.idx()].accent());
    let ah = art.sprites.arm_rail.h as f32;
    if let Some((x, flip)) = rail_at {
        let recoil = if mech.shake > 0.3 { (mech.shake * 3.0).round() } else { 0.0 };
        let dead = mech.hp[Section::Railgun.idx()] <= 0.0;
        let tint = dead.then_some(Color32::from_rgb(90, 70, 70));
        art.sprites.arm_rail.paint_accent(p, px(x + recoil, FEED_H - ah + 4.0 + recoil), s, 0.0, tint, flip, Some(if dead { DIM } else { rail_col }));
    }
    if let Some((x, flip)) = ac_at {
        let jitter = if mech.ac_firing { ((scene.t * 60.0) as i32 % 2) as f32 } else { 0.0 };
        let dead = mech.hp[Section::Autocannon.idx()] <= 0.0;
        let tint = dead.then_some(Color32::from_rgb(90, 70, 70));
        art.sprites.arm_ac.paint_accent(p, px(x, FEED_H - ah + 4.0 + jitter), s, 0.0, tint, flip, Some(if dead { DIM } else { ac_col }));
    }

    // HUD: bearing tape, crosshair, label
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
    // Off-camera target (single view only): an arrow on the nearer edge, with how far to turn.
    if let Some((_, e)) = mech.target.and_then(|t| scene.enemies.iter().find(|(en, _)| *en == t)).filter(|_| single) {
        let rel = angle_diff(e.est_angle, bearing);
        if rel.abs() >= fov * 0.6 {
            let right = rel > 0.0;
            let c = Color32::from_rgb(255, 80, 60);
            let blink = (scene.t * 3.0).fract() < 0.7;
            if blink {
                let ex = if right { fr.max.x - 10.0 * s } else { fr.min.x + 10.0 * s };
                let ey = fr.center().y;
                let dir = if right { 1.0 } else { -1.0 };
                p.add(egui::Shape::convex_polygon(
                    vec![Pos2::new(ex + 6.0 * s * dir, ey), Pos2::new(ex - 4.0 * s * dir, ey - 7.0 * s), Pos2::new(ex - 4.0 * s * dir, ey + 7.0 * s)],
                    c,
                    Stroke::NONE,
                ));
                let turns = ((rel.abs() / CAMERA_FOV).round() as i32).max(1);
                let hint = format!("{}-{:02}  {}", e.kind.callsign(), e.serial, if right { format!("TAB ×{turns}") } else { format!("SHIFT+TAB ×{turns}") });
                let tx = if right { ex - 8.0 * s } else { ex + 8.0 * s };
                text(p, Pos2::new(tx, ey), if right { Align2::RIGHT_CENTER } else { Align2::LEFT_CENTER }, hint, mono(18.0), c);
            }
        }
    }
    let cc = px(FEED_W / 2.0, FEED_H * 0.5);
    for (a, b) in [(Vec2::new(-14.0, 0.0), Vec2::new(-5.0, 0.0)), (Vec2::new(5.0, 0.0), Vec2::new(14.0, 0.0)), (Vec2::new(0.0, -14.0), Vec2::new(0.0, -5.0)), (Vec2::new(0.0, 5.0), Vec2::new(0.0, 14.0))] {
        p.line_segment([cc + a, cc + b], Stroke::new(1.0, GREEN.gamma_multiply(0.7)));
    }
    if (scene.t * 1.5).fract() < 0.6 {
        p.circle_filled(fr.min + Vec2::new(16.0, fr.height() - 14.0), 4.0, RED);
    }
    text(p, fr.min + Vec2::new(26.0, fr.height() - 14.0), Align2::LEFT_CENTER, label, mono(18.0), GREEN);
    let cam_hp = mech.cameras[cam] / CAMERA_HP;
    if cam_hp < 0.6 {
        text(p, fr.right_top() + Vec2::new(-10.0, fr.height() - 14.0), Align2::RIGHT_CENTER, format!("OPTICS {:.0}%", cam_hp * 100.0), mono(18.0), AMBER);
    }

    // Damaged sensors / weak optics / heat
    let sf = mech.frac(Section::Sensors);
    let weak = (1.0 - cam_hp).max(1.0 - sf);
    let static_n = (weak * 500.0) as i32 + if mech.shake > 0.5 { 300 } else { 0 };
    for i in 0..static_n {
        let h = hash2(i, frame_i, 17);
        let x = (h % FEED_W as u32) as f32;
        let y = ((h >> 9) % FEED_H as u32) as f32;
        let v = ((h >> 20) % 200) as u8 + 40;
        p.rect_filled(Rect::from_min_size(px(x, y), Vec2::new(s * 2.0, s)), 0.0, Color32::from_rgb(v, v, v));
    }
    if weak > 0.5 && (frame_i / 5) % 7 == 0 {
        let y = ((hash2(frame_i, 1, 2) % FEED_H as u32) as f32).round();
        p.rect_filled(Rect::from_min_size(px(0.0, y), Vec2::new(fr.width(), 6.0 * s)), 0.0, Color32::from_white_alpha(40));
    }
    // Dirt on this lens, fixed per camera; cracks spread as the optics fail.
    for i in 0..14 {
        let h = hash2(i, cam as i32, 83);
        let x = (h % FEED_W as u32) as f32;
        let y = ((h >> 9) % FEED_H as u32) as f32;
        let a = 18 + ((h >> 20) % 30) as u8;
        p.rect_filled(Rect::from_min_size(px(x, y), Vec2::splat(s * (1.0 + (h % 3) as f32))), 0.0, Color32::from_rgba_unmultiplied(40, 30, 30, a));
    }
    if cam_hp < 0.5 {
        let n = ((0.5 - cam_hp) * 16.0) as i32 + 2;
        let origin = px(((hash2(cam as i32, 1, 91) % FEED_W as u32) as f32), ((hash2(cam as i32, 2, 91) % FEED_H as u32) as f32));
        for i in 0..n {
            let h = hash2(i, cam as i32, 97);
            let a = (h % 628) as f32 / 100.0;
            let len = (20.0 + ((h >> 10) % 60) as f32) * s;
            let mid = origin + Vec2::new(a.cos(), a.sin()) * len * 0.5;
            let end = origin + Vec2::new(a.cos(), a.sin()) * len;
            let kink = Vec2::new(-a.sin(), a.cos()) * 4.0 * s;
            p.line_segment([origin, mid + kink], Stroke::new(1.0, Color32::from_rgba_unmultiplied(220, 230, 240, 120)));
            p.line_segment([mid + kink, end], Stroke::new(1.0, Color32::from_rgba_unmultiplied(220, 230, 240, 80)));
        }
    }
    if mech.heat > 85.0 {
        let a = ((mech.heat - 85.0) / 45.0 * 90.0) as u8;
        p.rect_filled(fr, 0.0, Color32::from_rgba_unmultiplied(255, 60, 20, a));
    }
}

/// Chunky pixel explosion.
fn boom(p: &egui::Painter, c: Pos2, s: f32, life: f32, seed: u32) {
    // Smoke rises and spreads as the fireball dies.
    if life > 0.25 {
        let sl = (life - 0.25) / 0.75;
        for i in 0..10 {
            let h = hash2(i, seed as i32, 21);
            let dx = ((h % 100) as f32 / 50.0 - 1.0) * (3.0 + sl * 8.0) * s;
            let dy = -(((h >> 8) % 100) as f32 / 100.0 + sl * 2.2) * 6.0 * s;
            let g = 70 + ((h >> 16) % 40) as u8;
            let a = ((1.0 - sl) * 170.0) as u8;
            p.rect_filled(Rect::from_center_size(c + Vec2::new(dx, dy), Vec2::splat(s * (1.5 + sl * 2.0))), 0.0, Color32::from_rgba_unmultiplied(g, g - 10, g - 14, a));
        }
    }
    // Debris chunks on ballistic arcs.
    for i in 0..8 {
        let h = hash2(i, seed as i32, 33);
        let vx = ((h % 100) as f32 / 50.0 - 1.0) * 14.0;
        let vy = -(6.0 + ((h >> 8) % 100) as f32 / 100.0 * 12.0);
        let t = life * 1.6;
        let pt = c + Vec2::new(vx * t, vy * t + 24.0 * t * t) * s;
        let col = if (h >> 16) % 3 == 0 { Color32::from_rgb(255, 170, 60) } else { Color32::from_rgb(70, 62, 60) };
        p.rect_filled(Rect::from_center_size(pt, Vec2::splat(s * 1.2)), 0.0, col.gamma_multiply(1.0 - life * 0.6));
    }
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
        // Ceiling conduit with brackets and a flickering lamp
        let cy = r.min.y + 2.0 * s;
        p.rect_filled(Rect::from_min_size(Pos2::new(r.min.x + 2.0 * s, cy), Vec2::new(r.width() - 4.0 * s, s)), 0.0, Color32::from_rgb(80, 66, 50));
        let mut bx = r.min.x + 6.0 * s;
        while bx < r.max.x - 4.0 * s {
            p.rect_filled(Rect::from_min_size(Pos2::new(bx, cy - s), Vec2::new(s, 3.0 * s)), 0.0, Color32::from_rgb(52, 48, 50));
            bx += 14.0 * s;
        }
        let flicker = f > 0.35 || hash2((t * 17.0) as i32, sec.idx() as i32, 8) % 5 != 0;
        let lamp = Pos2::new(r.center().x, r.min.y + 4.0 * s);
        if flicker {
            p.rect_filled(Rect::from_center_size(lamp, Vec2::new(4.0 * s, s)), 0.0, Color32::from_rgb(255, 236, 190));
            p.rect_filled(Rect::from_center_size(lamp + Vec2::new(0.0, 4.0 * s), Vec2::new(16.0 * s, 7.0 * s)), 0.0, Color32::from_rgba_unmultiplied(255, 230, 170, 16));
        } else {
            p.rect_filled(Rect::from_center_size(lamp, Vec2::new(4.0 * s, s)), 0.0, Color32::from_rgb(70, 64, 54));
        }
        // Steam leaking from damaged plumbing
        if f < 0.7 && *sec != Section::Uplink {
            for i in 0..6 {
                let h = hash2(i, sec.idx() as i32, 19);
                let phase = ((t * 1.3 + (h % 100) as f64 / 25.0).fract()) as f32;
                let x = r.min.x + (6 + (h >> 8) % 12) as f32 * s + phase * 6.0 * s;
                let y = r.max.y - 6.0 * s - phase * 12.0 * s;
                let a = ((1.0 - phase) * 110.0 * (1.0 - f)) as u8;
                p.rect_filled(Rect::from_min_size(Pos2::new(x, y), Vec2::splat(s * (1.0 + phase * 2.0))), 0.0, Color32::from_rgba_unmultiplied(220, 225, 230, a));
            }
        }
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
        // Machinery
        let prop = &art.sprites.props[sec.idx()];
        prop.paint(p, Pos2::new(r.min.x + 30.0 * s, r.max.y - (prop.h as f32 + 4.0) * s), s, 0.0, None, false);
        if *sec == Section::Reactor {
            let pulse = ((t * 2.5).sin() * 0.5 + 0.5) as f32;
            p.rect_filled(Rect::from_min_size(Pos2::new(r.min.x + 33.0 * s, r.max.y - 11.0 * s), Vec2::new(8.0 * s, 4.0 * s)), 0.0, Color32::from_rgba_unmultiplied(255, 180, 80, (60.0 * pulse) as u8));
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
        Pilot::Seat => (floor(&room_rect[&Section::Cockpit]), "CLICK A COMPARTMENT TO CRAWL THERE".to_string()),
        Pilot::Crawling { to, t, dur } => (path(to, t / dur), format!("CRAWLING TO {} ... {:.0}s", to.name(), (dur - t).ceil())),
        Pilot::Working { at, t } => {
            let n = scene.boarders_on(at);
            let cam_job = (at == Section::Sensors)
                .then(|| (0..CAMERAS).filter(|c| mech.cameras[*c] < CAMERA_HP).min_by(|a, b| mech.cameras[*a].total_cmp(&mech.cameras[*b])))
                .flatten();
            let msg = if n > 0 {
                format!("FIGHTING BOARDERS ({n}) ... {:.0}%", (t / 2.2 * 100.0).min(100.0))
            } else if let Some(cam) = cam_job {
                format!("REWIRING {} CAMERA ... {:.0}%", CAMERA_NAMES[cam], mech.cameras[cam] / CAMERA_HP * 100.0)
            } else if mech.frac(at) < 1.0 || (at == Section::Uplink && mech.uplink() < 100.0) {
                format!("HAND REPAIR: {}", at.name())
            } else {
                format!("{}: NOTHING TO FIX", at.name())
            };
            (floor(&room_rect[&at]), msg)
        }
        Pilot::Returning { from, t, dur } => (path(from, 1.0 - t / dur), format!("RETURNING TO CHAIR ... {:.0}s", (dur - t).ceil())),
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
// Hull & core: the damage diagram is the readout. Colour is health, a flash is a hit,
// the halo is the shield, ticks are boarders. Click a section to send repair drones.
// --------------------------------------------------------------------------------------

fn hull_panel(ui: &mut Ui, rect: Rect, art: &Art, mech: &mut Mech, cmds: &mut PilotCommands, campaign: &Campaign, scene: &Scene) {
    let inner = panel_frame(ui, rect, "HULL & CORE", Some(campaign.makers[Section::Cockpit.idx()]));
    let s = ((inner.width() - 24.0) / art::HULL_W as f32).floor().min(5.0);
    let dr = Rect::from_min_size(
        Pos2::new(inner.center().x - art::HULL_W as f32 * s / 2.0, inner.min.y + 14.0),
        Vec2::new(art::HULL_W as f32 * s, art::HULL_H as f32 * s),
    );
    let bg = dr.expand(12.0);
    let p = ui.painter_at(bg);
    p.rect_filled(bg, 0.0, Color32::from_rgb(8, 14, 18));
    let mut gx = bg.min.x;
    while gx < bg.max.x {
        p.line_segment([Pos2::new(gx, bg.min.y), Pos2::new(gx, bg.max.y)], Stroke::new(1.0, Color32::from_rgb(16, 30, 38)));
        gx += 4.0 * s;
    }
    let mut gy = bg.min.y;
    while gy < bg.max.y {
        p.line_segment([Pos2::new(bg.min.x, gy), Pos2::new(bg.max.x, gy)], Stroke::new(1.0, Color32::from_rgb(16, 30, 38)));
        gy += 4.0 * s;
    }
    let t = scene.t;
    let st = mech.stats;
    let hover_pos = ui.ctx().pointer_hover_pos();
    let mut hovered: Option<Section> = None;
    let map = &art.hull;
    let at = |x: i32, y: i32| -> u8 {
        if x < 0 || y < 0 || x >= art::HULL_W as i32 || y >= art::HULL_H as i32 { 254 } else { map[y as usize * art::HULL_W + x as usize] }
    };
    let cell = |x: i32, y: i32| Rect::from_min_size(dr.min + Vec2::new(x as f32 * s, y as f32 * s), Vec2::splat(s));

    // Shield halo: a two-pixel aura around the silhouette, bright when charged.
    let shield_frac = if st.shield_cap > 0.0 { (mech.shield / st.shield_cap).clamp(0.0, 1.0) } else { 0.0 };
    let down = mech.shields_down_t > 0.0;
    if shield_frac > 0.02 || down {
        let halo = if down { RED } else { Color32::from_rgb(110, 150, 255) };
        for y in -2..(art::HULL_H as i32 + 2) {
            for x in -2..(art::HULL_W as i32 + 2) {
                if at(x, y) != 254 {
                    continue;
                }
                let mut best = 3;
                for dy in -2..=2i32 {
                    for dx in -2..=2i32 {
                        if at(x + dx, y + dy) != 254 {
                            best = best.min(dx.abs().max(dy.abs()));
                        }
                    }
                }
                if best <= 2 {
                    let shimmer = (((x + y) as f64 * 0.7 + t * 6.0).sin() * 0.5 + 0.5) as f32;
                    let base = if best == 1 { 0.7 } else { 0.3 };
                    let a = if down { 0.4 * ((t * 8.0).fract() < 0.5) as i32 as f32 } else { base * (0.4 + 0.6 * shield_frac) * (0.7 + 0.3 * shimmer) };
                    if a > 0.0 {
                        p.rect_filled(cell(x, y), 0.0, halo.gamma_multiply(a * 0.6));
                    }
                }
            }
        }
    }

    // Drop shadow under the silhouette.
    for y in 0..art::HULL_H as i32 {
        for x in 0..art::HULL_W as i32 {
            if at(x, y) != 254 && at(x + 1, y + 1) == 254 {
                p.rect_filled(cell(x + 1, y + 1), 0.0, Color32::from_black_alpha(120));
            }
        }
    }
    let scan_y = ((t * 0.35).fract() as f32 * (art::HULL_H as f32 + 6.0)) as i32 - 3;
    let mut centroid = [(0.0f32, 0.0f32, 0u32); 10];
    for y in 0..art::HULL_H as i32 {
        for x in 0..art::HULL_W as i32 {
            let v = at(x, y);
            if v == 254 {
                continue;
            }
            let edge = at(x - 1, y) != v || at(x + 1, y) != v || at(x, y - 1) != v || at(x, y + 1) != v;
            let hi = at(x, y - 1) != v || at(x - 1, y) != v;
            let r = cell(x, y);
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
                // Crack pattern on damaged parts
                if sec != Section::Uplink && f < 0.6 && hash2(x, y, 4).is_multiple_of(7) {
                    c = c.gamma_multiply(0.4);
                }
                // Boarders gnawing on a section: a slow purple pulse.
                if scene.boarders_on(sec) > 0 {
                    let k = ((t * 3.0).sin() * 0.5 + 0.5) as f32 * 0.6;
                    c = Color32::from_rgb(
                        (c.r() as f32 + (PURPLE.r() as f32 - c.r() as f32) * k) as u8,
                        (c.g() as f32 + (PURPLE.g() as f32 - c.g() as f32) * k) as u8,
                        (c.b() as f32 + (PURPLE.b() as f32 - c.b() as f32) * k) as u8,
                    );
                }
                // A hit lights the section up and fades.
                let flash = mech.hit_t[sec.idx()];
                if flash > 0.0 {
                    let k = flash * flash;
                    c = Color32::from_rgb(
                        (c.r() as f32 + (255.0 - c.r() as f32) * k) as u8,
                        (c.g() as f32 + (240.0 - c.g() as f32) * k * 0.8) as u8,
                        (c.b() as f32 + (200.0 - c.b() as f32) * k * 0.6) as u8,
                    );
                }
                c
            };
            let d = art.hull_detail[y as usize * art::HULL_W + x as usize];
            if d == art::HD_HAZARD {
                col = if art::hull_hazard_dark(x, y) { Color32::from_rgb(24, 22, 20) } else { art::hull_shade(col, d, t) };
            } else if d != art::HD_NONE {
                col = art::hull_shade(col, d, t);
            } else if edge {
                col = if hi { col.gamma_multiply(1.15) } else { col.gamma_multiply(0.45) };
            } else {
                col = col.gamma_multiply(0.78);
            }
            // Diagnostic scan line sweeping down the frame.
            if y == scan_y {
                col = col.gamma_multiply(1.6);
            }
            p.rect_filled(r, 0.0, col);
        }
    }
    if let Some(h) = hovered {
        for y in 0..art::HULL_H as i32 {
            for x in 0..art::HULL_W as i32 {
                let v = at(x, y);
                if v == h as u8 && (at(x - 1, y) != v || at(x + 1, y) != v || at(x, y - 1) != v || at(x, y + 1) != v) {
                    p.rect_stroke(cell(x, y), CornerRadius::ZERO, Stroke::new(1.0, AMBER), StrokeKind::Inside);
                }
            }
        }
    }
    let cen = |sec: Section| {
        let c = centroid[sec.idx()];
        dr.min + Vec2::new((c.0 / c.2.max(1) as f32 + 0.5) * s, (c.1 / c.2.max(1) as f32 + 0.5) * s)
    };
    // Repair drones: a blinking outline around their target.
    let rt = mech.repair_target;
    if (t * 2.0).fract() < 0.6 {
        for y in 0..art::HULL_H as i32 {
            for x in 0..art::HULL_W as i32 {
                let v = at(x, y);
                if v == rt as u8 {
                    let outside = at(x - 1, y) != v || at(x + 1, y) != v || at(x, y - 1) != v || at(x, y + 1) != v;
                    if outside {
                        p.rect_stroke(cell(x, y), CornerRadius::ZERO, Stroke::new(1.0, Color32::from_rgb(240, 130, 60)), StrokeKind::Inside);
                    }
                }
            }
        }
    }
    // Boarders, incoming shells, the pilot
    for sec in Section::ALL {
        let n = scene.boarders_on(sec);
        for i in 0..n {
            let off = Vec2::new((i as f32 - (n as f32 - 1.0) / 2.0) * 12.0, 0.0);
            let wig = ((t * 6.0 + i as f64).sin() as f32).round();
            art.sprites.tick.paint(&p, cen(sec) + off - Vec2::new(6.0, 5.0 + wig), 1.0, 0.0, None, i % 2 == 1);
        }
        if let Some(sh) = scene.shell_on(sec) {
            let c = cen(sec);
            let rr = 6.0 + (1.0 - sh.t / sh.flight) * 14.0;
            if (t * 6.0).fract() < 0.6 {
                p.circle_stroke(c, rr, Stroke::new(2.0, RED));
                p.line_segment([c - Vec2::new(rr + 3.0, 0.0), c + Vec2::new(rr + 3.0, 0.0)], Stroke::new(1.0, RED));
                p.line_segment([c - Vec2::new(0.0, rr + 3.0), c + Vec2::new(0.0, rr + 3.0)], Stroke::new(1.0, RED));
            }
        }
    }
    if let Pilot::Working { at, .. } | Pilot::Crawling { to: at, .. } = mech.pilot {
        art.sprites.pilot.paint(&p, cen(at) - Vec2::new(5.0, 8.0), 1.0, 0.0, None, false);
    }
    let dresp = ui.interact(dr, ui.id().with("hulldiag"), Sense::click());
    if let Some(h) = hovered {
        if dresp.clicked() {
            mech.repair_target = h;
        }
        let f = if h == Section::Uplink { mech.uplink() } else { mech.frac(h) * 100.0 };
        dresp.on_hover_text(format!("{} {:.0}% — click: repair drones here", h.name(), f));
    }
    let lp = ui.painter_at(inner);
    text(&lp, Pos2::new(bg.min.x, bg.max.y + 4.0), Align2::LEFT_TOP, format!("DRONES → {}", rt.short()), mono(16.0), Color32::from_rgb(240, 130, 60));
    if mech.cameras_up() < CAMERAS {
        text(&lp, Pos2::new(bg.max.x, bg.max.y + 4.0), Align2::RIGHT_TOP, format!("CAM {}/{}", mech.cameras_up(), CAMERAS), mono(16.0), AMBER);
    }

    // --- Core gauges --------------------------------------------------------------------------
    let gz = Rect::from_min_max(Pos2::new(inner.min.x, bg.max.y + 26.0), inner.max);
    let pp = ui.painter_at(gz);
    let therm = Rect::from_min_max(Pos2::new(gz.min.x + 14.0, gz.min.y + 18.0), Pos2::new(gz.min.x + 60.0, gz.max.y - 56.0));
    text(&pp, Pos2::new(therm.center().x, gz.min.y), Align2::CENTER_TOP, "HEAT", pix(10.0), RED);
    pp.rect_filled(therm, 0.0, EDGE_LO);
    // The bulb at the foot of the thermometer, glowing with the core.
    let bulb = Pos2::new(therm.center().x, therm.max.y + 2.0);
    pp.circle_filled(bulb, 10.0, EDGE_LO);
    let bulb_col = if mech.heat >= 100.0 { RED } else if mech.heat >= 80.0 { Color32::from_rgb(255, 120, 40) } else if mech.heat >= 50.0 { AMBER } else { GREEN };
    pp.circle_filled(bulb, 7.0, bulb_col.gamma_multiply(0.4 + 0.6 * (mech.heat / HEAT_MAX).min(1.0)));
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
    text(&pp, Pos2::new(therm.center().x, therm.max.y + 14.0), Align2::CENTER_TOP, format!("{:.0}", mech.heat), mono(28.0), if mech.heat >= 80.0 { RED } else { TEXT });
    text(&pp, Pos2::new(therm.center().x + 2.0, therm.max.y + 36.0), Align2::CENTER_TOP, format!("{:+.1}/s", net), mono(16.0), if net > 0.0 { AMBER } else { GREEN });
    // Shield column
    let sh = Rect::from_min_max(Pos2::new(gz.max.x - 56.0, therm.min.y), Pos2::new(gz.max.x - 14.0, therm.max.y));
    text(&pp, Pos2::new(sh.center().x, gz.min.y), Align2::CENTER_TOP, "SHIELD", pix(10.0), Color32::from_rgb(110, 150, 255));
    pp.rect_filled(sh, 0.0, EDGE_LO);
    let fill = Rect::from_min_max(Pos2::new(sh.min.x + 3.0, sh.max.y - shield_frac * (sh.height() - 6.0) - 3.0), sh.max - Vec2::new(3.0, 3.0));
    if st.shield_cap <= 0.0 {
        text(&pp, sh.center(), Align2::CENTER_CENTER, "NOT\nWIRED", mono(16.0), DIM);
    } else {
        pp.rect_filled(fill, 0.0, if down { RED } else { Color32::from_rgb(110, 150, 255) });
    }
    text(&pp, Pos2::new(sh.center().x, sh.max.y + 4.0), Align2::CENTER_TOP, format!("{:.0}", mech.shield), mono(24.0), Color32::from_rgb(110, 150, 255));
    // Reactor summary between the gauges
    let mid = Pos2::new((therm.max.x + sh.min.x) / 2.0 + 10.0, therm.center().y);
    let draw_col = if st.supply < 1.0 { RED } else if st.parasitic > 0.0 { AMBER } else { DIM };
    text(&pp, mid - Vec2::new(0.0, 22.0), Align2::CENTER_CENTER, "CORE", pix(9.0), DIM);
    text(&pp, mid - Vec2::new(0.0, 4.0), Align2::CENTER_CENTER, format!("{:.0}/{:.0}", st.draw, st.available), mono(20.0), draw_col);
    if st.parasitic > 0.0 {
        text(&pp, mid + Vec2::new(0.0, 14.0), Align2::CENTER_CENTER, format!("-{:.0} TICKS", st.parasitic), mono(16.0), PURPLE);
    } else if st.supply < 1.0 {
        text(&pp, mid + Vec2::new(0.0, 14.0), Align2::CENTER_CENTER, "BROWNOUT", mono(16.0), RED);
    }
    let _ = cmds;
}

/// Big red countdown over the feed while the reactor is cooking off.
fn overload_banner(ui: &mut Ui, feed: Rect, mech: &Mech, scene: &Scene) {
    let Some(t) = mech.overload else { return };
    let big = Rect::from_center_size(Pos2::new(feed.center().x, feed.min.y + 110.0), Vec2::new(460.0, 72.0));
    let p = ui.painter();
    p.rect_filled(big, 0.0, Color32::from_black_alpha(220));
    p.rect_stroke(big, CornerRadius::ZERO, Stroke::new(3.0, RED), StrokeKind::Inside);
    text(p, big.center() - Vec2::new(0.0, 15.0), Align2::CENTER_CENTER, "REACTOR OVERLOAD", pix(16.0), RED);
    text(p, big.center() + Vec2::new(0.0, 16.0), Align2::CENTER_CENTER, format!("CONTAINMENT IN {:.1}s", t.max(0.0)), mono(26.0), if (scene.t * 4.0).fract() < 0.5 { Color32::WHITE } else { RED });
}

// --------------------------------------------------------------------------------------
// Hotbar: every action the pilot has, as a miniature with its own charge.
// --------------------------------------------------------------------------------------

/// Where the pilot goes when they leave the chair: boarders first, then a dead camera,
/// then the worst section.
pub fn crew_destination(mech: &Mech, scene: &Scene) -> Section {
    scene
        .boarders
        .first()
        .map(|b| b.section)
        .or_else(|| (mech.cameras_up() < CAMERAS).then_some(Section::Sensors))
        .or_else(|| {
            Section::ALL
                .iter()
                .copied()
                .filter(|s| *s != Section::Cockpit && *s != Section::Uplink)
                .min_by(|a, b| mech.frac(*a).total_cmp(&mech.frac(*b)))
        })
        .unwrap_or(Section::Reactor)
}

struct Slot {
    name: &'static str,
    key: &'static str,
    color: Color32,
    /// 0..1 fill of the charge bar.
    charge: f32,
    status: String,
    status_col: Color32,
    enabled: bool,
    lit: bool,
}

fn hotbar_panel(ui: &mut Ui, rect: Rect, art: &Art, mech: &mut Mech, cmds: &mut PilotCommands, campaign: &Campaign, scene: &Scene) {
    let inner = panel_frame(ui, rect, "STATIONS", None);
    let st = mech.stats;
    let seated = mech.pilot.in_seat();
    let target = mech.target.and_then(|t| scene.enemies.iter().find(|(e, _)| *e == t)).map(|(_, e)| e.clone());
    let rail_dead = mech.hp[Section::Railgun.idx()] <= 0.0;
    let ac_dead = mech.hp[Section::Autocannon.idx()] <= 0.0;
    let rail_ready = mech.rail_charge >= 1.0 && !rail_dead;
    let lock = target.as_ref().is_some_and(|e| e.dist <= st.radar_range);
    let radar_only = target.as_ref().is_some_and(|e| !e.covered);
    let hitp = (0.92 * (0.5 + 0.5 * mech.eff(Section::Sensors)) - if target.as_ref().is_some_and(|e| e.kind == EnemyKind::Scout) { 0.15 } else { 0.0 })
        * target.as_ref().map(|e| e.aim_factor()).unwrap_or(1.0);
    let rail_status = if rail_dead {
        ("DESTROYED".to_string(), RED)
    } else if !campaign.matrix[Sys::Railgun.idx()] {
        ("NOT WIRED".into(), DIM)
    } else if !rail_ready {
        (if st.rail_rate > 0.0 { format!("{:.1}s", (1.0 - mech.rail_charge) / st.rail_rate) } else { "NO POWER".into() }, AMBER)
    } else if !lock {
        ("T: TARGET".into(), AMBER)
    } else if radar_only {
        (format!("{:.0}% RADAR", hitp * 100.0), AMBER)
    } else {
        (format!("{:.0}% HIT", hitp * 100.0), GREEN)
    };
    let ac_status = if ac_dead {
        ("DESTROYED".to_string(), RED)
    } else if !campaign.matrix[Sys::Autocannon.idx()] {
        ("NOT WIRED".into(), DIM)
    } else if mech.ac_firing {
        (format!("{:.0}/s", st.ac_dps), YELLOW)
    } else if mech.weapons_free {
        ("TRACKING".into(), GREEN)
    } else {
        ("SAFE".into(), DIM)
    };
    let ecm_ok = mech.ecm_cd <= 0.0 && st.eff[Sys::Sensors.idx()] >= 5.0;
    let boarders = scene.boarders.len();
    let crew_status = match mech.pilot {
        Pilot::Seat => (if boarders > 0 { format!("{boarders} BOARDERS") } else { "IN CHAIR".into() }, if boarders > 0 { PURPLE } else { DIM }),
        Pilot::Crawling { .. } => ("CRAWLING".into(), AMBER),
        Pilot::Working { at, .. } => (at.short().to_string(), AMBER),
        Pilot::Returning { .. } => ("RETURNING".into(), AMBER),
    };
    let overload = mech.overload.is_some();
    let slots = [
        Slot { name: "RAIL", key: "1 · SPC", color: CYAN, charge: mech.rail_charge, status: rail_status.0, status_col: rail_status.1, enabled: seated && rail_ready && lock && mech.scram_t <= 0.0, lit: rail_ready && lock },
        Slot { name: "A/C", key: "2", color: YELLOW, charge: if mech.ac_firing { 1.0 - ((scene.t * 1.5).fract()) as f32 } else { 1.0 }, status: ac_status.0, status_col: ac_status.1, enabled: !ac_dead && campaign.matrix[Sys::Autocannon.idx()], lit: mech.weapons_free && !ac_dead },
        Slot { name: "ECM", key: "3", color: CYAN, charge: 1.0 - mech.ecm_cd / 28.0, status: if ecm_ok { "READY".into() } else if mech.ecm_cd > 0.0 { format!("{:.0}s", mech.ecm_cd) } else { "NO SENSORS".into() }, status_col: if ecm_ok { GREEN } else { DIM }, enabled: ecm_ok && seated, lit: ecm_ok },
        Slot { name: "VENT", key: "4", color: Color32::from_rgb(150, 230, 255), charge: 1.0 - mech.vent_cd / 25.0, status: if mech.vent_t > 0.0 { "VENTING".into() } else if mech.vent_cd > 0.0 { format!("{:.0}s", mech.vent_cd) } else { "-20 HEAT/s".into() }, status_col: if mech.vent_cd <= 0.0 { GREEN } else { DIM }, enabled: mech.vent_cd <= 0.0 && seated, lit: mech.vent_t > 0.0 },
        Slot { name: "SCRAM", key: "5", color: AMBER, charge: 1.0 - mech.scram_cd / 30.0, status: if mech.scram_t > 0.0 { format!("DEAD {:.0}s", mech.scram_t) } else if mech.scram_cd > 0.0 { format!("{:.0}s", mech.scram_cd) } else { "KILL BUSES".into() }, status_col: if mech.scram_cd <= 0.0 { GREEN } else { DIM }, enabled: mech.scram_cd <= 0.0 && seated, lit: mech.scram_t > 0.0 },
        Slot { name: "EJECT", key: "6", color: RED, charge: if overload { 1.0 } else { 0.0 }, status: if overload { "NOW".into() } else { "LOCKED".into() }, status_col: if overload { RED } else { DIM }, enabled: overload, lit: overload && (scene.t * 3.0).fract() < 0.5 },
        Slot { name: if seated { "CREW" } else { "RETURN" }, key: "7", color: PURPLE, charge: 1.0, status: crew_status.0, status_col: crew_status.1, enabled: seated || matches!(mech.pilot, Pilot::Working { .. }), lit: boarders > 0 && seated },
    ];
    let n = slots.len() as f32;
    let gap = 8.0;
    let sw = ((inner.width() - gap * (n - 1.0)) / n).min(150.0);
    let total = sw * n + gap * (n - 1.0);
    let x0 = inner.center().x - total / 2.0;
    let sh = inner.height();
    let mut clicked: Option<usize> = None;
    for (i, slot) in slots.iter().enumerate() {
        let r = Rect::from_min_size(Pos2::new(x0 + i as f32 * (sw + gap), inner.min.y), Vec2::new(sw, sh));
        let resp = ui.interact(r, ui.id().with(("slot", i)), if slot.enabled { Sense::click() } else { Sense::hover() });
        if resp.clicked() {
            clicked = Some(i);
        }
        let p = ui.painter_at(r);
        let face = if slot.lit { slot.color.gamma_multiply(0.22) } else { PANEL_DARK };
        p.rect_filled(r, 0.0, face);
        let border = if slot.lit { slot.color } else if resp.hovered() && slot.enabled { slot.color.gamma_multiply(0.7) } else { EDGE_HI };
        p.rect_stroke(r, CornerRadius::ZERO, Stroke::new(if slot.lit { 2.0 } else { 1.0 }, border), StrokeKind::Inside);
        // Miniature
        let icon = &art.sprites.hotbar[i];
        let k = ((sh - 30.0) / 19.0).floor().clamp(2.0, 3.0);
        let imin = Pos2::new(r.center().x - 12.0 * k, r.min.y + 4.0);
        let tint = if !slot.enabled && !slot.lit { Some(Color32::from_rgb(110, 110, 120)) } else { None };
        let flash = if i == 0 && mech.shake > 0.5 { 0.6 } else { 0.0 };
        icon.paint(&p, imin, k, flash, tint, false);
        // Charge bar
        let bar = Rect::from_min_size(Pos2::new(r.min.x + 8.0, imin.y + 19.0 * k + 4.0), Vec2::new(sw - 16.0, 8.0));
        seg_bar(&p, bar, slot.charge, 10, slot.color);
        text(&p, Pos2::new(r.min.x + 8.0, r.max.y - 5.0), Align2::LEFT_BOTTOM, slot.name, pix(10.0), if slot.enabled || slot.lit { slot.color } else { DIM });
        text(&p, Pos2::new(r.max.x - 8.0, r.max.y - 5.0), Align2::RIGHT_BOTTOM, &slot.status, mono(16.0), slot.status_col);
        text(&p, r.right_top() + Vec2::new(-5.0, 3.0), Align2::RIGHT_TOP, slot.key, mono(15.0), slot.color.gamma_multiply(0.8));
    }
    match clicked {
        Some(0) => cmds.fire_rail = true,
        Some(1) => mech.weapons_free = !mech.weapons_free,
        Some(2) => cmds.ecm = true,
        Some(3) => cmds.vent = true,
        Some(4) => cmds.scram = true,
        Some(5) => cmds.eject = true,
        Some(6) => {
            if seated {
                cmds.crawl_to = Some(crew_destination(mech, scene));
            } else {
                cmds.return_to_seat = true;
            }
        }
        _ => {}
    }
}

// --------------------------------------------------------------------------------------
// Log: the radio, under the radar. Short lines, newest at the bottom.
// --------------------------------------------------------------------------------------

fn voice_style(v: Voice) -> (&'static str, Color32) {
    match v {
        Voice::Control => ("CONTROL", CYAN),
        Voice::Chief => ("ODA", AMBER),
        Voice::Pilot => ("YOU", GREEN),
        Voice::Enemy => ("INTERCEPT", RED),
        Voice::System => ("OS", Color32::from_rgb(170, 180, 170)),
    }
}

fn log_panel(ui: &mut Ui, rect: Rect, art: &Art, radio: &Radio, animate: bool) {
    let inner = panel_frame(ui, rect, "RADIO", None);
    let p = ui.painter_at(inner);
    p.rect_filled(inner, 0.0, Color32::from_rgb(8, 12, 9));
    // Who spoke last, as a small portrait with a talk indicator.
    let head = Rect::from_min_size(inner.min, Vec2::new(inner.width(), 60.0));
    if let Some(m) = radio.log.last() {
        let sprite = match m.voice {
            Voice::Control => &art.sprites.control_face,
            Voice::Chief => &art.sprites.chief_face,
            Voice::Pilot => &art.sprites.pilot_face,
            Voice::Enemy => &art.sprites.enemy_face,
            Voice::System => &art.sprites.system_face,
        };
        let (name, col) = voice_style(m.voice);
        let fr = Rect::from_min_size(head.min + Vec2::new(4.0, 4.0), Vec2::splat(48.0));
        p.rect_filled(fr, 0.0, col.gamma_multiply(0.15));
        sprite.paint(&p, fr.min, 2.0, 0.0, None, false);
        // Mouth flaps while the line is fresh.
        let talking = radio.time - m.time < 3.0;
        if talking && ((radio.time * 9.0) as i32) % 2 == 0 {
            let mouth = match m.voice {
                Voice::Pilot => Some((Vec2::new(10.0, 19.0), Vec2::new(4.0, 2.0), Color32::from_rgb(120, 40, 40))),
                Voice::Chief => Some((Vec2::new(10.0, 17.0), Vec2::new(4.0, 2.0), Color32::from_rgb(60, 20, 20))),
                Voice::Control => Some((Vec2::new(10.0, 14.0), Vec2::new(3.0, 2.0), Color32::from_rgb(120, 40, 40))),
                Voice::Enemy => Some((Vec2::new(5.0, 12.0), Vec2::new(14.0, 2.0), RED.gamma_multiply(0.7))),
                Voice::System => None,
            };
            if let Some((off, size, c)) = mouth {
                p.rect_filled(Rect::from_min_size(fr.min + off * 2.0, size * 2.0), 0.0, c);
            }
        }
        scanlines(&p, fr, 50);
        text(&p, Pos2::new(fr.max.x + 10.0, fr.min.y + 10.0), Align2::LEFT_CENTER, name, pix(10.0), col);
        if radio.time - m.time < 3.0 {
            for i in 0..8 {
                let hgt = ((radio.time * 13.0 + i as f32 * 1.7).sin().abs() * 14.0) + 2.0;
                p.rect_filled(Rect::from_min_size(Pos2::new(fr.max.x + 10.0 + i as f32 * 8.0, fr.max.y - 4.0 - hgt), Vec2::new(5.0, hgt)), 0.0, col);
            }
        }
    }
    let log_r = Rect::from_min_max(Pos2::new(inner.min.x, head.max.y), inner.max);
    p.line_segment([log_r.left_top(), log_r.right_top()], Stroke::new(1.0, EDGE_HI));
    child(ui, log_r.shrink2(Vec2::new(6.0, 4.0)), |ui| {
        egui::ScrollArea::vertical().stick_to_bottom(true).auto_shrink([false, false]).show(ui, |ui| {
            let n = radio.log.len();
            for (i, m) in radio.log.iter().enumerate() {
                let (name, col) = voice_style(m.voice);
                let mut body = m.text.clone();
                if i + 1 == n && animate {
                    let chars = ((radio.time - m.time) * 70.0).max(0.0) as usize;
                    body = body.chars().take(chars.max(1)).collect();
                }
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.label(egui::RichText::new(name).color(col).size(17.0));
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
        art.sprites.control_face.paint(&p, face.min + Vec2::new(0.0, 0.0), 3.5, 0.0, None, false);
        scanlines(&p, face, 50);
        ui.add_space(36.0);
        ui.set_max_width(r.width() - 100.0);
        let body = [
            ("CITY CONTROL", CYAN, "Bastion, the city's uplink relay on your mast is down. Without it the shield grid stays dark and the last districts freeze tonight."),
            ("CHIEF ODA", AMBER, "Hold position; the drones rebuild the uplink. Your left camera is dead, so anything west is radar-only: wider error, worse shots."),
            ("CHIEF ODA", AMBER, "Every weapon heats the core. Past 100, containment goes in 12 seconds. VENT dumps heat but drops shields; SCRAM kills everything for 6."),
            ("CITY CONTROL", CYAN, "Jackal scouts, Mortar Crabs past your radar, Tick Carriers that board you. Watch the hull: what's lit is what's being hit."),
        ];
        for (who, c, line) in body {
            ui.label(egui::RichText::new(who).color(c).size(20.0));
            ui.add(egui::Label::new(egui::RichText::new(line).size(21.0).color(TEXT)).wrap());
            ui.add_space(6.0);
        }
        ui.add_space(4.0);
        ui.label(egui::RichText::new("TAB camera right, SHIFT+TAB left · C camera wall · T / click target · 1-7 stations · SPACE fire · P pause").size(18.0).color(DIM));
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
        // Vignette: Bastion's silhouette, lit by what happened to her.
        {
            let vs = 2.0;
            let vr = Rect::from_min_size(Pos2::new(r.min.x + 10.0, r.min.y + 110.0), Vec2::new(art::HULL_W as f32 * vs, art::HULL_H as f32 * vs));
            p.rect_filled(vr.expand(6.0), 0.0, Color32::from_rgb(8, 10, 14));
            for i in 0..40 {
                let h = hash2(i, 3, 71);
                p.rect_filled(Rect::from_min_size(vr.min + Vec2::new((h % 80) as f32, ((h >> 8) % 30) as f32), Vec2::splat(1.0)), 0.0, Color32::from_rgb(120, 110, 130));
            }
            for y in 0..art::HULL_H {
                for x in 0..art::HULL_W {
                    let v = art.hull[y * art::HULL_W + x];
                    if v == 254 {
                        continue;
                    }
                    let d = art.hull_detail[y * art::HULL_W + x];
                    let base = match outcome {
                        Outcome::Victory => Color32::from_rgb(70, 92, 84),
                        _ => Color32::from_rgb(60, 40, 40),
                    };
                    let mut c = if d != art::HD_NONE { art::hull_shade(base, d, scene.t) } else { base };
                    if outcome != Outcome::Victory && v == Section::Reactor as u8 {
                        let pulse = ((scene.t * 5.0).sin() * 0.5 + 0.5) as f32;
                        c = Color32::from_rgb(255, (120.0 + 100.0 * pulse) as u8, 40);
                    }
                    if outcome == Outcome::Victory && v == Section::Uplink as u8 && (scene.t * 2.0).fract() < 0.5 {
                        c = CYAN;
                    }
                    p.rect_filled(Rect::from_min_size(vr.min + Vec2::new(x as f32 * vs, y as f32 * vs), Vec2::splat(vs)), 0.0, c);
                }
            }
            if outcome != Outcome::Victory {
                let life = ((scene.t * 0.7).fract()) as f32;
                boom(&p, vr.center() - Vec2::new(0.0, 10.0), 3.0, life, 5);
            }
            if outcome == Outcome::Ejected {
                let f = ((scene.t * 0.5).fract()) as f32;
                art.sprites.hotbar[5].paint(&p, vr.center_top() + Vec2::new(10.0 + f * 40.0, -20.0 - f * 60.0), 1.0, 0.0, None, false);
            }
        }
        let stats = [
            ("MISSION TIME", fmt_clock(mission.time)),
            ("CONTACTS DESTROYED", format!("{}", mech.kills)),
            ("PEAK CORE HEAT", format!("{:.0}", mech.heat_peak)),
            ("UPLINK", format!("{:.0}%", mech.uplink())),
        ];
        let mut y = r.min.y + 120.0;
        for (k, v) in stats {
            text(&p, Pos2::new(r.min.x + 130.0, y), Align2::LEFT_TOP, k, mono(24.0), DIM);
            text(&p, Pos2::new(r.max.x - 60.0, y), Align2::RIGHT_TOP, v, mono(24.0), TEXT);
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
        let mut ui = modal(ctx, "pause", Vec2::new(520.0, 300.0), DIM);
        let r = ui.max_rect();
        let p = ui.painter().clone();
        text(&p, Pos2::new(r.center().x, r.min.y + 4.0), Align2::CENTER_TOP, "PAUSED", pix(24.0), AMBER);
        let mut y = r.min.y + 52.0;
        for (k, v) in [
            ("TAB", "next camera clockwise (SHIFT+TAB back)"),
            ("C", "camera wall: all four feeds"),
            ("T / click radar", "select target (next contact is picked after a kill)"),
            ("1-7", "stations: rail · a/c · ECM · vent · scram · eject · crew"),
            ("SPACE", "fire railgun"),
            ("click hull", "send repair drones there"),
            ("P / ESC", "pause"),
        ] {
            text(&p, Pos2::new(r.min.x + 130.0, y), Align2::RIGHT_TOP, k, mono(21.0), AMBER);
            text(&p, Pos2::new(r.min.x + 146.0, y), Align2::LEFT_TOP, v, mono(21.0), TEXT);
            y += 24.0;
        }
        let br = Rect::from_min_size(Pos2::new(r.center().x - 100.0, r.max.y - 44.0), Vec2::new(200.0, 40.0));
        if pbutton(&mut ui, br, "RESUME [P]", AMBER, true, true).clicked() {
            mission.paused = false;
        }
    }
}
