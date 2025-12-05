pub mod example;
pub mod hook;

use std::sync::Arc;

use bevy::{
    app::PluginsState,
    light::DirectionalLightShadowMap,
    log::{Level, LogPlugin},
    prelude::*,
    render::{
        RenderDebugFlags, RenderPlugin,
        renderer::{
            RenderAdapter, RenderAdapterInfo, RenderDevice, RenderInstance, RenderQueue,
            WgpuWrapper,
        },
        settings::{Backends, RenderCreation, WgpuSettings},
    },
    window::WindowResolution,
};
use rcade_plugin_input_classic::ClassicController;
use wasm_bindgen::prelude::*;
use web_sys::console;

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

    loop {
        app.update();
        gloo_timers::future::sleep(std::time::Duration::from_nanos(0)).await;
    }
}

impl BevyApp {
    pub async fn new() -> Self {
        let mut app = App::new();
        let canvas = get_offscreen_canvas().unwrap();
        let controller = ClassicController::acquire().await.unwrap();

        // Manually initialize WebGL2 rendering resources
        let render_resources = Self::initialize_webgl2(&canvas)
            .await
            .expect("Failed to initialize WebGL2 renderer");

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
                .set(RenderPlugin {
                    debug_flags: RenderDebugFlags::default(),
                    render_creation: RenderCreation::Manual(
                        bevy::render::settings::RenderResources(
                            render_resources.device.into(),
                            RenderQueue(Arc::new(WgpuWrapper::new(render_resources.queue))),
                            RenderAdapterInfo(WgpuWrapper::new(
                                render_resources.adapter.get_info(),
                            )),
                            RenderAdapter(Arc::new(WgpuWrapper::new(render_resources.adapter))),
                            RenderInstance(Arc::new(WgpuWrapper::new(render_resources.instance))),
                        ),
                    ),
                    synchronous_pipeline_compilation: false,
                })
                .set(ImagePlugin::default_nearest())
                .set(LogPlugin {
                    level: Level::TRACE,
                    ..Default::default()
                }),
        )
        .insert_resource(DirectionalLightShadowMap { size: 512 })
        .insert_non_send_resource(controller)
        .insert_non_send_resource(canvas)
        .add_systems(PreStartup, setup_added_window)
        .add_systems(Startup, setup)
        .add_systems(Update, rotate)
        .add_systems(Update, camera_control_system);

        BevyApp { app }
    }

    async fn initialize_webgl2(
        canvas: &web_sys::OffscreenCanvas,
    ) -> Result<RenderResources, String> {
        console::log_1(&"Initializing WebGL2 manually...".into());

        // Create wgpu instance with GL backend
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::GL,
            flags: wgpu::InstanceFlags::default(),
            backend_options: wgpu::BackendOptions {
                gl: wgpu::GlBackendOptions {
                    gles_minor_version: wgpu::Gles3MinorVersion::Version0,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        });

        console::log_1(&"Created wgpu instance".into());

        // Create the window handle and surface
        let window_handle = OffscreenWindowHandle::new(canvas);

        let surface_target = unsafe { wgpu::SurfaceTargetUnsafe::from_window(&window_handle) }
            .map_err(|e| format!("Failed to create surface target: {:?}", e))?;

        let surface = unsafe { instance.create_surface_unsafe(surface_target) }
            .map_err(|e| format!("Failed to create surface: {:?}", e))?;

        console::log_1(&"Created surface from OffscreenCanvas".into());

        // Request adapter with the compatible surface
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| format!("Failed to find suitable GPU adapter: {e:?}"))?;

        console::log_1(&format!("Found adapter: {:?}", adapter.get_info()).into());

        // Request device and queue with WebGL2 compatible limits
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("bevy_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                ..Default::default()
            })
            .await
            .map_err(|e| format!("Failed to create device: {:?}", e))?;

        console::log_1(&"Created device and queue".into());

        Ok(RenderResources {
            instance,
            adapter,
            device,
            queue,
        })
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

struct RenderResources {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}
