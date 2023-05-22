use bevy::prelude::*;
use bevy::window::PresentMode;

//use bevy::diagnostic::LogDiagnosticsPlugin;
use bevy_tank_war::TankWarGamePlugin;

fn main() {
    // env_logger::init();

    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                title: "Tank War - Rust edition".to_string(),
                width: 1024.,
                height: 768.,
                present_mode: PresentMode::AutoNoVsync,
                resizable: false,
                ..default()
            },
            ..default()
        }))
        // // Adds frame time diagnostics
        // .add_plugin(FrameTimeDiagnosticsPlugin::default())
        // Adds a system that prints diagnostics to the console
        // .add_plugin(LogDiagnosticsPlugin {
        //     debug: true,
        //     ..Default::default()
        // })
        .add_plugin(TankWarGamePlugin)
        .run();
}
