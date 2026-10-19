//! HANGAR ZERO — you are the last pilot of a decommissioned walking fortress.
//! The whole game is played from the cockpit: radar, power routing, heat,
//! damage control and the radio.

mod art;
mod campaign;
mod rng;
mod shots;
mod sim;
mod ui;

use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass};

use campaign::Campaign;
use rng::Rng;
use sim::*;
use ui::hangar::HangarState;

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    Boot,
    Cockpit,
    Hangar,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowRequest {
    ToHangar,
    Deploy,
    NewCampaign,
}

/// Screen-flow requests raised by the UI.
#[derive(Resource, Default)]
pub struct Flow {
    pub request: Option<FlowRequest>,
}

fn main() {
    let shot = shots::ShotPlan::from_env();
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb_u8(9, 10, 13)))
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "HANGAR ZERO".into(),
                        resolution: WindowResolution::new(1600, 900),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(EguiPlugin::default())
        .add_plugins(SimPlugin)
        .init_state::<AppState>()
        .insert_resource(Rng::new(shot.as_ref().map(|s| s.seed).unwrap_or_else(seed_from_clock)))
        .init_resource::<Campaign>()
        .init_resource::<Flow>()
        .init_resource::<HangarState>()
        .init_resource::<ui::Art>()
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .add_systems(OnEnter(AppState::Boot), boot)
        .add_systems(OnEnter(AppState::Cockpit), start_mission)
        .add_systems(OnExit(AppState::Cockpit), end_mission)
        .add_systems(Update, handle_flow)
        .add_systems(EguiPrimaryContextPass, setup_egui)
        .add_systems(EguiPrimaryContextPass, ui::cockpit::cockpit_ui.run_if(in_state(AppState::Cockpit)).run_if(resource_exists::<FontsReady>).after(setup_egui))
        .add_systems(EguiPrimaryContextPass, ui::hangar::hangar_ui.run_if(in_state(AppState::Hangar)).run_if(resource_exists::<FontsReady>).after(setup_egui));
    if let Some(plan) = shot {
        app.insert_resource(plan).add_systems(Update, shots::run_shot_plan);
    }
    app.run();
}

fn seed_from_clock() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7)
}

/// Fonts once, and a zoom factor so the 1600x900 layout fills any window.
/// Font changes apply on the next egui pass, so the UI waits one frame for them.
#[derive(Resource)]
struct FontsReady;

fn setup_egui(
    mut commands: Commands,
    mut contexts: EguiContexts,
    window: Single<&Window, With<PrimaryWindow>>,
    mut frames: Local<u32>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    if *frames == 0 {
        ui::install_fonts(ctx);
    }
    if *frames == 1 {
        commands.insert_resource(FontsReady);
    }
    *frames = frames.saturating_add(1);
    let zoom = (window.width() / ui::DESIGN_W).min(window.height() / ui::DESIGN_H).max(0.5);
    if (ctx.zoom_factor() - zoom).abs() > 0.001 {
        ctx.set_zoom_factor(zoom);
    }
    Ok(())
}

fn boot(mut next: ResMut<NextState<AppState>>, shot: Option<Res<shots::ShotPlan>>) {
    if shot.is_some_and(|s| s.scene == "hangar") {
        next.set(AppState::Hangar);
    } else {
        next.set(AppState::Cockpit);
    }
}

fn start_mission(mut commands: Commands, campaign: Res<Campaign>, mut radio: ResMut<Radio>, mut fx: ResMut<Effects>, mut cmds: ResMut<PilotCommands>) {
    commands.insert_resource(Mech::new(&campaign));
    commands.insert_resource(Mission::new(campaign.mission));
    *radio = Radio::default();
    fx.list.clear();
    *cmds = PilotCommands::default();
    radio.say(Voice::System, "BASTION OS v7.31 — COLD START COMPLETE. 3 OF 9 SUBSYSTEMS REPORT FAULTS.");
    radio.say(Voice::Control, "Bastion, City Control. Uplink mast is dark. Get it back up and hold the line.");
    if campaign.mission > 1 {
        radio.say(Voice::Chief, format!("Op {:02}. She's not the machine she was, kid. Neither are we.", campaign.mission));
    } else {
        radio.say(Voice::Chief, "Old girl hasn't walked in thirty years. Be gentle with the reactor.");
    }
}

fn end_mission(
    mut commands: Commands,
    q: Query<Entity, Or<(With<Enemy>, With<Drone>, With<Boarder>, With<Shell>)>>,
) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

fn handle_flow(
    mut flow: ResMut<Flow>,
    mut campaign: ResMut<Campaign>,
    mut rng: ResMut<Rng>,
    mut hangar: ResMut<HangarState>,
    mission: Option<Res<Mission>>,
    mech: Option<Res<Mech>>,
    mut next: ResMut<NextState<AppState>>,
) {
    let Some(req) = flow.request.take() else { return };
    match req {
        FlowRequest::ToHangar => {
            if let (Some(mission), Some(mech)) = (mission, mech) {
                campaign.apply_scars(&mission, mech.kills, mech.heat_peak);
            }
            campaign.mission += 1;
            campaign.roll_salvage(&mut rng);
            hangar.chosen = None;
            next.set(AppState::Hangar);
        }
        FlowRequest::Deploy => next.set(AppState::Cockpit),
        FlowRequest::NewCampaign => {
            *campaign = Campaign::default();
            next.set(AppState::Boot);
        }
    }
}
