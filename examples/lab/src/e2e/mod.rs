use bevy::prelude::*;
use bevy_e2e::{action::Action, actions::assertions, scenario::Scenario};

use crate::{
    LabDiagnostics, trigger_bevel, trigger_crater_steps, trigger_extrude, trigger_subdivide,
};

#[derive(Resource, Default)]
struct BeforeLabSnapshot {
    subdivision_faces: usize,
    extrude_faces: usize,
    crater_min_y: f32,
}

pub struct MeshOpsLabE2EPlugin;

impl Plugin for MeshOpsLabE2EPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy_e2e::E2EPlugin);
        app.init_resource::<BeforeLabSnapshot>();
        let args: Vec<String> = std::env::args().collect();
        let (scenario_name, handoff) = parse_e2e_args(&args);
        if let Some(name) = scenario_name {
            if let Some(mut scenario) = scenario_by_name(&name) {
                if handoff {
                    scenario.actions.push(Action::Handoff);
                }
                bevy_e2e::init_scenario(app, scenario);
            } else {
                error!(
                    "[mesh_ops_lab:e2e] Unknown scenario '{name}'. Available: {:?}",
                    list_scenarios()
                );
            }
        }
    }
}

fn parse_e2e_args(args: &[String]) -> (Option<String>, bool) {
    let mut scenario_name = None;
    let mut handoff = false;
    for arg in args.iter().skip(1) {
        if arg == "--handoff" {
            handoff = true;
        } else if !arg.starts_with('-') && scenario_name.is_none() {
            scenario_name = Some(arg.clone());
        }
    }
    if !handoff {
        handoff = std::env::var("E2E_HANDOFF").is_ok_and(|value| value == "1" || value == "true");
    }
    (scenario_name, handoff)
}

pub fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "mesh_ops_smoke" => Some(mesh_ops_smoke()),
        "mesh_ops_subdivision_compare" => Some(mesh_ops_subdivision_compare()),
        "mesh_ops_extrude_and_bevel" => Some(mesh_ops_extrude_and_bevel()),
        "mesh_ops_runtime_crater" => Some(mesh_ops_runtime_crater()),
        _ => None,
    }
}

pub fn list_scenarios() -> Vec<&'static str> {
    vec![
        "mesh_ops_smoke",
        "mesh_ops_subdivision_compare",
        "mesh_ops_extrude_and_bevel",
        "mesh_ops_runtime_crater",
    ]
}

fn mesh_ops_smoke() -> Scenario {
    Scenario::builder("mesh_ops_smoke")
        .description(
            "Verify that all four lab targets boot with editable meshes and stable diagnostics.",
        )
        .then(Action::WaitFrames(30))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "lab meshes ready",
            |diagnostics| {
                diagnostics.extrude.faces > 0
                    && diagnostics.bevel.faces > 0
                    && diagnostics.subdivision.faces > 0
                    && diagnostics.crater.faces > 0
            },
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "runtime clean at boot",
            |diagnostics| diagnostics.failures == 0,
        ))
        .then(Action::Screenshot("mesh_ops_smoke".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("mesh_ops_smoke"))
        .build()
}

fn mesh_ops_subdivision_compare() -> Scenario {
    Scenario::builder("mesh_ops_subdivision_compare")
        .description("Capture the baseline subdivision target, then run Catmull-Clark once and verify the face count increases.")
        .then(Action::WaitFrames(20))
        .then(Action::Screenshot("subdivision_before".into()))
        .then(Action::WaitFrames(1))
        .then(remember_subdivision_faces())
        .then(Action::Custom(Box::new(trigger_subdivide)))
        .then(wait_until(
            "subdivision faces increase",
            Box::new(|world| {
                let before = world.resource::<BeforeLabSnapshot>().subdivision_faces;
                world.resource::<LabDiagnostics>().subdivision.faces > before
            }),
            180,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "subdivision revision advanced",
            |diagnostics| diagnostics.subdivision.revision > 0,
        ))
        .then(Action::Screenshot("subdivision_after".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("mesh_ops_subdivision_compare"))
        .build()
}

fn mesh_ops_extrude_and_bevel() -> Scenario {
    Scenario::builder("mesh_ops_extrude_and_bevel")
        .description("Trigger one extrusion and one bevel request through the lab control resource and verify both targets change.")
        .then(Action::WaitFrames(20))
        .then(remember_extrude_faces())
        .then(Action::Custom(Box::new(trigger_extrude)))
        .then(wait_until(
            "extrude changed",
            Box::new(|world| {
                world.resource::<LabDiagnostics>().extrude.faces
                    > world.resource::<BeforeLabSnapshot>().extrude_faces
            }),
            120,
        ))
        .then(Action::Screenshot("extrude_after".into()))
        .then(Action::WaitFrames(1))
        .then(Action::Custom(Box::new(trigger_bevel)))
        .then(wait_until(
            "bevel changed",
            Box::new(|world| world.resource::<LabDiagnostics>().bevel.faces == 3),
            120,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "topology changes recorded",
            |diagnostics| diagnostics.topology_changes >= 2 && diagnostics.failures == 0,
        ))
        .then(Action::Screenshot("bevel_after".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("mesh_ops_extrude_and_bevel"))
        .build()
}

fn mesh_ops_runtime_crater() -> Scenario {
    Scenario::builder("mesh_ops_runtime_crater")
        .description("Apply repeated offset requests to the crater patch and verify the local minimum height drops over time.")
        .then(Action::WaitFrames(20))
        .then(Action::Screenshot("crater_before".into()))
        .then(Action::WaitFrames(1))
        .then(remember_crater_height())
        .then(Action::Custom(Box::new(|world| trigger_crater_steps(world, 2))))
        .then(wait_until(
            "crater step 1",
            Box::new(|world| world.resource::<LabDiagnostics>().crater.revision >= 2),
            120,
        ))
        .then(Action::Screenshot("crater_mid".into()))
        .then(Action::WaitFrames(1))
        .then(Action::Custom(Box::new(|world| trigger_crater_steps(world, 2))))
        .then(wait_until(
            "crater depth increased",
            Box::new(|world| {
                world.resource::<LabDiagnostics>().crater.min_y
                    < world.resource::<BeforeLabSnapshot>().crater_min_y - 0.12
            }),
            180,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "crater moved downward",
            |diagnostics| diagnostics.crater.revision >= 4,
        ))
        .then(Action::Screenshot("crater_after".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("mesh_ops_runtime_crater"))
        .build()
}

fn remember_subdivision_faces() -> Action {
    Action::Custom(Box::new(|world| {
        world.resource_mut::<BeforeLabSnapshot>().subdivision_faces =
            world.resource::<LabDiagnostics>().subdivision.faces;
    }))
}

fn remember_extrude_faces() -> Action {
    Action::Custom(Box::new(|world| {
        world.resource_mut::<BeforeLabSnapshot>().extrude_faces =
            world.resource::<LabDiagnostics>().extrude.faces;
    }))
}

fn remember_crater_height() -> Action {
    Action::Custom(Box::new(|world| {
        world.resource_mut::<BeforeLabSnapshot>().crater_min_y =
            world.resource::<LabDiagnostics>().crater.min_y;
    }))
}

fn wait_until(
    label: &str,
    condition: Box<dyn Fn(&World) -> bool + Send + Sync + 'static>,
    max_frames: u32,
) -> Action {
    Action::WaitUntil {
        label: label.to_string(),
        condition,
        max_frames,
    }
}
