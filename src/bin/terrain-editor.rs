// ----------------------------------------------------------------------------
#![forbid(unsafe_code)]
// #![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds

// disable console on windows for release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::prelude::{App, ClearColor, Color, Msaa, WindowDescriptor};
use bevy::DefaultPlugins;

use bevy::render::settings::{WgpuFeatures, WgpuSettings};
use terrain_editor::EditorPlugin;
// ----------------------------------------------------------------------------
fn main() {
    let mut app = App::new();

    app.insert_resource(ClearColor(Color::rgb(0.4, 0.4, 0.4)))
        // terrain rendering uses multiple rendertargets from witch subsequent
        // passes try to sample. therefore msaa must be deactivated.
        .insert_resource(Msaa { samples: 1 })
        .insert_resource(WgpuSettings {
            features: WgpuFeatures::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                | WgpuFeatures::TEXTURE_FORMAT_16BIT_NORM,
            ..Default::default()
        })
        // .insert_resource(bevy::log::LogSettings {
        //     level: bevy::log::Level::INFO,
        //     filter: "wgpu=error,bevy_render=trace".to_string(),
        // })
        .insert_resource(WindowDescriptor {
            width: 800.,
            height: 600.,
            title: "Terrain Texture Editor".to_string(),
            ..Default::default()
        })
        // pipelined default plugins initializes some lights???
        .add_plugins(DefaultPlugins)
        // .add_plugins_with(DefaultPlugins, |plugins| {
        //     plugins.disable::<LogPlugin>()
        // })
        // .add_plugin(bevy::log::LogPlugin::default())
        // .add_plugin(bevy::core::CorePlugin::default())
        // .add_plugin(bevy::transform::TransformPlugin::default())
        // .add_plugin(bevy::diagnostic::DiagnosticsPlugin::default())
        // .add_plugin(bevy::input::InputPlugin::default())
        // .add_plugin(bevy::window::WindowPlugin::default())
        // .add_plugin(bevy::asset::AssetPlugin::default())
        // .add_plugin(bevy::scene::ScenePlugin::default())
        // .add_plugin(bevy::winit::WinitPlugin::default())
        // .add_plugin(bevy::render::RenderPlugin::default())
        // .add_plugin(bevy::core_pipeline::CorePipelinePlugin::default())
        // .add_plugin(bevy::bevy_pbr::PbrPlugin::default())
        .add_plugin(EditorPlugin);

    #[cfg(debug_assertions)]
    {
        app
            // .add_plugin(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
            .add_plugin(bevy::diagnostic::LogDiagnosticsPlugin::default());
    }

    // bevy_mod_debugdump::print_render_graph(&mut app);
    // bevy_mod_debugdump::print_render_schedule_graph(&mut app);
    app.run();
}
// ----------------------------------------------------------------------------
