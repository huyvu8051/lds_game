
use bevy::app::App;
use bevy::asset::AssetApp;

pub fn build_headless_game_ecs_app(physical_width: u32, physical_height: u32, scale_factor: f32) -> App {
    // Before anything spawns: it decides whether mesh and image data survives
    // on the CPU, which metal_renderer needs and bevy_render does not.
    crate::mark_native_renderer_in_use();
    crate::ablation::log_configuration();

    let mut app = App::new();

    app.add_plugins(bevy::app::PanicHandlerPlugin);
    app.add_plugins(bevy::log::LogPlugin::default());
    app.add_plugins(bevy::MinimalPlugins);
    app.add_plugins(bevy::diagnostic::DiagnosticsPlugin);

    app.add_plugins(bevy::state::app::StatesPlugin);

    app.add_plugins(bevy::transform::TransformPlugin);
    app.add_plugins(bevy::asset::AssetPlugin::default());

    app.add_plugins(bevy::mesh::MeshPlugin);
    app.add_plugins(bevy::image::ImagePlugin::default());
    // Without this nothing registers a loader for .wav or .ogg.
    // Seedling rather than bevy_audio, and Steam Audio inside it. The scene
    // comes from Avian's colliders, which this path already builds.
    //
    // NO_AUDIO=1 keeps everything except the cpal backend, which is the part
    // that opens the device and runs the real-time output thread. That thread
    // keeps its own deadline on a performance core and never lands in the
    // frame's own timings, so removing it is the only way to price it. The
    // core plugin stays so the AudioSample asset, its loader and every
    // SamplePlayer still exist -- the game plays into a graph with no output
    // rather than failing to load its sounds.
    let audio_off = std::env::var("NO_AUDIO").is_ok();
    if !crate::ablation::enabled(crate::ablation::Group::Audio) {
        // Nothing at all, not even the silent graph NO_AUDIO leaves behind:
        // the point of this group is to price the graph too. setup_common
        // still asks the asset server for every sound, so the asset type has
        // to exist or the first load aborts the process. Without seedling
        // there is no loader for them either, so the handles stay pending --
        // which is the intent, nothing decodes.
        app.init_asset::<bevy_seedling::sample::AudioSample>();
    } else if audio_off {
        use bevy::app::PluginGroup;
        app.add_plugins(
            bevy_seedling::SeedlingPlugins
                .build()
                .disable::<bevy_seedling::platform::cpal::CpalPlatformPlugin>(),
        );
    } else {
        app.add_plugins(bevy_seedling::SeedlingPlugins);
    }
    // Not on iOS: see the note on the dependency in Cargo.toml.
    //
    // Nor under NO_AUDIO: Steam Audio reads resources the cpal backend
    // creates and panics in its first frame without them. Skipping it leaves
    // the desktop build in the same shape the phone is always in.
    //
    // The ablation switch is the same gate for the same reason: with seedling
    // gone there is no Time<Audio>, and Steam Audio's node observer asks for
    // it the moment a node is inserted.
    #[cfg(not(target_os = "ios"))]
    if !audio_off && crate::ablation::enabled(crate::ablation::Group::Audio) {
        app.add_plugins(bevy_steam_audio::SteamAudioPlugin::default());
        app.add_plugins(avian_steam_audio::AvianSteamAudioScenePlugin);
    }

    // Every texture the game ships is ASTC in a KTX2 container. Without
    // RenderPlugin nothing tells Bevy what the GPU can take, so the loader
    // refuses them and the map draws untextured -- Apple silicon takes ASTC
    // natively, so say so.
    let formats = bevy::image::CompressedImageFormats::ASTC_LDR;
    app.register_asset_loader(bevy::image::ImageLoader::new(formats));
    app.insert_resource(bevy::image::CompressedImageFormatSupport(formats));

    app.add_plugins(bevy::camera::CameraPlugin);
    app.add_plugins(bevy::light::LightPlugin);
    app.add_plugins(bevy::text::TextPlugin);

    app.init_asset::<bevy::image::TextureAtlasLayout>();
    if crate::ablation::enabled(crate::ablation::Group::Ui) {
        app.add_plugins(bevy::ui::UiPlugin::default());
    }
    app.add_plugins(bevy::scene::ScenePlugin);
    app.add_plugins(bevy::world_serialization::WorldSerializationPlugin);
    app.add_plugins(bevy::gltf::GltfPlugin::default());
    metal_renderer::gltf_materials::register_gltf_standard_materials(&mut app);

    app.init_asset::<bevy::pbr::StandardMaterial>();

    if crate::ablation::enabled(crate::ablation::Group::Animation) {
        app.add_plugins(bevy::animation::AnimationPlugin);
    } else {
        // Same reason the materials below are registered by hand: the plugin
        // is what the ablation prices, but setup_common asks for
        // Assets<AnimationGraph> unconditionally and a missing resource fails
        // system-param validation, which on this path aborts the process.
        app.init_asset::<bevy::animation::graph::AnimationGraph>();
        app.init_asset::<bevy::animation::AnimationClip>();
    }

    app.init_asset::<bevy::audio::AudioSource>();

    app.add_plugins(bevy::input::InputPlugin);

    app.add_plugins(bevy::picking::DefaultPickingPlugins);

    app.add_message::<bevy::window::WindowEvent>();

    app.add_plugins(bevy::gizmos::GizmoPlugin);

    app.register_type::<bevy::pbr::MeshMaterial3d<bevy::pbr::StandardMaterial>>();
    app.add_plugins(avian3d::prelude::PhysicsPlugins::default());
    app.insert_resource(avian3d::prelude::SubstepCount(crate::PHYSICS_SUBSTEPS));
    // Both of these live in main() for the windowed build, which this path
    // does not go through. Anything tuned there has to be repeated here or the
    // two builds quietly stop being the same game -- see PHYSICS_SUBSTEPS.
    use bevy::ecs::schedule::IntoScheduleConfigs;

    // Bevy's frustum culling writes ViewVisibility, and nothing on this path
    // reads it -- the extractor takes InheritedVisibility, which is the manual
    // shown/hidden flag, and metal_renderer now does its own frustum test
    // against its own camera. Bevy was culling against a camera this path does
    // not render from, and throwing the answer away: 0.31ms a frame between
    // these two sets.
    //
    // VisibilityPropagate is deliberately left alone. That is what computes
    // InheritedVisibility, which the extractor does read, and turning it off
    // would make everything invisible rather than faster.
    app.configure_sets(
        bevy::app::PostUpdate,
        (
            bevy::camera::visibility::VisibilitySystems::CheckVisibility,
            bevy::camera::visibility::VisibilitySystems::MarkNewlyHiddenEntitiesInvisible,
            bevy::camera::visibility::VisibilitySystems::CalculateBounds,
        )
            .run_if(|| false),
    );
    app.configure_sets(
        bevy::app::PostUpdate,
        bevy::light::SimulationLightSystems::CheckLightVisibility.run_if(|| false),
    );

    app.configure_sets(
        bevy::app::PostUpdate,
        bevy::ui::UiSystems::Layout.run_if(bevy::time::common_conditions::on_timer(
            std::time::Duration::from_millis(1000 / crate::UI_LAYOUT_HZ),
        )),
    );
    // Without these the native path has physics and a player but no zombies,
    // no AI and no world_ready -- it has never actually run the game. They sit
    // beside PhysicsPlugins in main()'s plugin list for the windowed build and
    // were simply never carried across.
    app.add_plugins(crate::sim::SimPlugin);
    if crate::ablation::enabled(crate::ablation::Group::ZombieAi) {
        app.add_plugins(crate::zombie_nav::ZombieNavmeshPlugin);
    }
    // No MaterialPlugin here: iOS draws through its own Metal renderer. The
    // asset collections still have to exist or the systems asking for them
    // panic, which across FFI aborts.
    app.init_asset::<crate::DamageOverlayMaterial>();
    app.init_asset::<crate::MuzzleSmokeMaterial>();
    app.init_asset::<crate::MuzzleFlashMaterial>();

    // Before GamePlugin: the mesh extractor looks this resource up on every
    // pass, and a map that finishes loading before the bake does would be
    // cached without it and never re-extracted.
    let scenario = crate::current_scenario();
    let lightmap = shared::asset_path(scenario.map_asset_dir())
        .map(|dir| crate::lightmap::load(&dir))
        .unwrap_or_default();
    app.insert_resource(lightmap);

    // EXEC=single runs Update on the single-threaded executor. The block in
    // lib.rs is one .chain() of ~190 systems, so it is already sequential --
    // what this drops is the executor's per-completion bookkeeping, not any
    // real parallelism.
    match std::env::var("EXEC").as_deref() {
        Ok("single") => {
            app.edit_schedule(bevy::app::Update, |s| {
                s.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
            });
        }
        Ok("all") => {
            use bevy::app::{First, FixedPostUpdate, FixedPreUpdate, FixedUpdate, Last, PostUpdate, PreUpdate, Update};
            app.edit_schedule(First, |s| { s.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new()); });
            app.edit_schedule(PreUpdate, |s| { s.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new()); });
            app.edit_schedule(FixedPreUpdate, |s| { s.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new()); });
            app.edit_schedule(FixedUpdate, |s| { s.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new()); });
            app.edit_schedule(FixedPostUpdate, |s| { s.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new()); });
            app.edit_schedule(Update, |s| { s.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new()); });
            app.edit_schedule(PostUpdate, |s| { s.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new()); });
            app.edit_schedule(Last, |s| { s.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new()); });
        }
        _ => {}
    }
    app.add_plugins(crate::GamePlugin);

    app.world_mut().spawn((
        bevy::window::Window {
            resolution: bevy::window::WindowResolution::new(physical_width, physical_height)
                .with_scale_factor_override(scale_factor),
            ..Default::default()
        },
        bevy::window::PrimaryWindow,
        bevy::window::CursorOptions {
            grab_mode: bevy::window::CursorGrabMode::Locked,
            ..Default::default()
        },
    ));

    if app.plugins_state() != bevy::app::PluginsState::Ready {
        while app.plugins_state() == bevy::app::PluginsState::Adding {
            bevy::tasks::tick_global_task_pools_on_main_thread();
        }
    }
    app.finish();
    app.cleanup();

    app
}
