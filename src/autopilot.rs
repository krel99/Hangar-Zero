//! A crude scripted pilot used by the simulation tests and the `play` screenshot
//! scene: keeps repair on the uplink, snipes what it can see, vents when hot and
//! leaves the chair when the hull is crawling with boarders.

use bevy::prelude::*;

use crate::sim::*;

pub fn autopilot(
    mut mech: ResMut<Mech>,
    mut cmds: ResMut<PilotCommands>,
    enemies: Query<(Entity, &Enemy)>,
    boarders: Query<&Boarder>,
) {
    let range = mech.stats.radar_range;
    let pick = enemies
        .iter()
        .filter(|(_, e)| e.dist <= range)
        .max_by_key(|(_, e)| match e.kind {
            EnemyKind::Artillery => 3,
            EnemyKind::Carrier => 2,
            EnemyKind::Scout => 1,
        })
        .map(|(e, _)| e);
    mech.target = pick;
    // Look where the target is, if that camera still works.
    if let Some((_, e)) = pick.and_then(|t| enemies.get(t).ok()) {
        let cam = camera_for(e.angle);
        if mech.camera_ok(cam) {
            mech.active_cam = cam;
        }
    }
    mech.alloc = if mech.heat > 60.0 { [5.0, 20.0, 10.0, 10.0, 10.0, 15.0, 30.0] } else { [5.0, 20.0, 20.0, 10.0, 10.0, 20.0, 15.0] };
    if mech.heat < 70.0 && mech.rail_charge >= 1.0 {
        cmds.fire_rail = true;
    }
    if mech.heat > 88.0 {
        cmds.vent = true;
    }
    if mech.overload.is_some() {
        cmds.scram = true;
    }
    let worst = Section::ALL[..9].iter().copied().min_by(|a, b| mech.frac(*a).total_cmp(&mech.frac(*b))).unwrap();
    mech.repair_target = if mech.frac(worst) < 0.3 { worst } else { Section::Uplink };
    if let Some(b) = boarders.iter().next()
        && mech.pilot.in_seat()
        && boarders.iter().count() >= 3
    {
        cmds.crawl_to = Some(b.section);
    }
    if let Pilot::Working { at, .. } = mech.pilot
        && boarders.iter().all(|b| b.section != at)
    {
        cmds.return_to_seat = true;
    }
}

