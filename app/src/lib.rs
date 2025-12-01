pub mod example;
pub mod hook;

use bevy::{
    app::PluginsState,
    light::DirectionalLightShadowMap,
    log::{Level, LogPlugin},
    prelude::*,
    window::{RawHandleWrapper, WindowResolution, WindowWrapper},
};
use gloo_timers::callback::Interval;
use rcade_plugin_input_classic::ClassicController;
use wasm_bindgen::prelude::*;
use web_sys::{console, js_sys};

use crate::{
    example::{camera_control_system, rotate, setup},
    hook::{OffscreenWindowHandle, get_offscreen_canvas, setup_added_window},
};

#[wasm_bindgen]
pub struct BevyApp {
    app: App,
}

#[wasm_bindgen(start)]
pub async fn start() {
    console_error_panic_hook::set_once();

    let mut app = BevyApp::new().await;

    let mut frame_count = 0;

    loop {
        let perf = js_sys::global()
            .dyn_into::<web_sys::WorkerGlobalScope>()
            .unwrap()
            .performance()
            .unwrap();

        let a = perf.now();
        app.update();
        frame_count += 1;
        let b = perf.now();
        gloo_timers::future::sleep(std::time::Duration::from_nanos(0)).await;
        let c = perf.now();
    }
}

impl BevyApp {
    pub async fn new() -> Self {
        let mut app = App::new();
        let canvas = get_offscreen_canvas().unwrap();
        let controller = ClassicController::acquire().await.unwrap();

        app.add_plugins(
            DefaultPlugins
                .set(bevy::window::WindowPlugin {
                    primary_window: Some(Window {
                        resolution: WindowResolution::new(336, 262),
                        ..Default::default()
                    }),
                    exit_condition: bevy::window::ExitCondition::DontExit,
                    ..Default::default()
                })
                .set(ImagePlugin::default_nearest())
                .set(LogPlugin {
                    level: Level::WARN,
                    ..Default::default()
                }),
        )
        .insert_resource(DirectionalLightShadowMap { size: 512 })
        .insert_non_send_resource(controller)
        .add_systems(PreStartup, setup_added_window)
        .add_systems(Startup, setup)
        .add_systems(Update, rotate)
        .add_systems(Update, camera_control_system);

        app.insert_non_send_resource(canvas);

        BevyApp { app }
    }

    pub fn update(&mut self) {
        if self.app.plugins_state() != PluginsState::Cleaned {
            if self.app.plugins_state() == PluginsState::Ready {
                self.app.finish();
                self.app.cleanup();
            }
        } else {
            self.app.update();
        }
    }
}
