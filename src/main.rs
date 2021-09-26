use bevy::prelude::*;

use bevy::diagnostic::LogDiagnosticsPlugin;
use bevy_tank_war::TankWarGamePlugin;

fn main() {
    env_logger::init();

    App::build()
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(WindowDescriptor {
            title: "Tank War - Rust edition".to_string(),
            width: 1024.,
            height: 768.,
            vsync: true,
            resizable: false,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
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
