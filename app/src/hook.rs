// Here be dragons!
//
// This module contains glue code to interface Bevy's rendering with an OffscreenCanvas
// in a web worker environment. It sets up the necessary window and display handles
// for wgpu to render to the OffscreenCanvas using WebGL2.
//
// This is required for the project architecture and should not be modified lightly.

use std::{ptr::NonNull, sync::Arc, thread::ThreadId};

use bevy::{
    app::PluginGroupBuilder,
    prelude::*,
    render::{
        RenderDebugFlags, RenderPlugin,
        renderer::{RenderAdapter, RenderAdapterInfo, RenderInstance, RenderQueue, WgpuWrapper},
        settings::RenderCreation,
    },
    window::{RawHandleWrapper, WindowResolution, WindowWrapper},
};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use wasm_bindgen::{JsCast, JsValue, prelude::wasm_bindgen};
use web_sys::{OffscreenCanvas, console};

#[wasm_bindgen]
extern "C" {
    // This tells wasm-bindgen to look for a global symbol named RUST_OFFSCREEN_CANVAS
    // on the 'self' (worker's global) scope.
    #[wasm_bindgen(js_name = RUST_OFFSCREEN_CANVAS, thread_local_v2)]
    pub static RUST_OFFSCREEN_CANVAS_RAW: JsValue;
}

// You would then cast it to the correct type when you need it:
pub fn get_offscreen_canvas() -> Result<OffscreenCanvas, JsValue> {
    RUST_OFFSCREEN_CANVAS_RAW
        .with(|v| v.clone())
        .dyn_into::<OffscreenCanvas>()
        .map_err(|_| JsValue::from_str("Global RUST_OFFSCREEN_CANVAS not found or wrong type."))
}

pub(crate) struct OffscreenWindowHandle {
    window_handle: raw_window_handle::RawWindowHandle,
    display_handle: raw_window_handle::DisplayHandle<'static>,
    thread_id: ThreadId,
}

impl OffscreenWindowHandle {
    pub(crate) fn new(canvas: &OffscreenCanvas) -> Self {
        // Equivalent to WebOffscreenCanvasWindowHandle::from_wasm_bindgen_0_2
        let ptr = NonNull::from(canvas).cast();
        let handle = raw_window_handle::WebOffscreenCanvasWindowHandle::new(ptr);
        let window_handle = raw_window_handle::RawWindowHandle::WebOffscreenCanvas(handle);
        let display_handle = raw_window_handle::DisplayHandle::web();

        Self {
            window_handle,
            display_handle,
            thread_id: std::thread::current().id(),
        }
    }
}

/// # Safety
///
/// At runtime we ensure that OffscreenWrapper is only accessed from the thread it was created on
unsafe impl Send for OffscreenWindowHandle {}

unsafe impl Sync for OffscreenWindowHandle {}

impl HasWindowHandle for OffscreenWindowHandle {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        if self.thread_id != std::thread::current().id() {
            // OffscreenWrapper can only be accessed from the thread it was
            // created on and considering web workers are only single threaded,
            // this error should never happen.
            return Err(raw_window_handle::HandleError::NotSupported);
        }

        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(self.window_handle) })
    }
}

impl HasDisplayHandle for OffscreenWindowHandle {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        Ok(self.display_handle)
    }
}

pub fn setup_added_window(
    mut commands: Commands,
    canvas: NonSendMut<OffscreenCanvas>,
    mut new_windows: Query<Entity, Added<Window>>,
) {
    // This system should only be called once at startup and there should only
    // be one window that's been added.
    let Some(entity) = new_windows.iter_mut().next() else {
        panic!("Multiple windows added")
    };

    let handle = OffscreenWindowHandle::new(&canvas);

    let handle = RawHandleWrapper::new(&WindowWrapper::new(handle))
        .expect("to create offscreen raw handle wrapper. If this fails, multiple threads are trying to access the same canvas!");

    commands.entity(entity).insert(handle);
}

async fn initialize_webgl2(canvas: &web_sys::OffscreenCanvas) -> Result<RenderResources, String> {
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

struct RenderResources {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

pub trait RcadePluginExt {
    fn with_rcade(self, canvas: OffscreenCanvas) -> impl Future<Output = PluginGroupBuilder>
    where
        Self: Sized;
}

impl RcadePluginExt for DefaultPlugins {
    async fn with_rcade(self, canvas: OffscreenCanvas) -> PluginGroupBuilder {
        // Manually initialize WebGL2 rendering resources
        let render_resources = initialize_webgl2(&canvas)
            .await
            .expect("Failed to initialize WebGL2 renderer");

        self.set(bevy::window::WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(336, 262),
                ..Default::default()
            }),
            exit_condition: bevy::window::ExitCondition::DontExit,
            ..Default::default()
        })
        .set(RenderPlugin {
            debug_flags: RenderDebugFlags::default(),
            render_creation: RenderCreation::Manual(bevy::render::settings::RenderResources(
                render_resources.device.into(),
                RenderQueue(Arc::new(WgpuWrapper::new(render_resources.queue))),
                RenderAdapterInfo(WgpuWrapper::new(render_resources.adapter.get_info())),
                RenderAdapter(Arc::new(WgpuWrapper::new(render_resources.adapter))),
                RenderInstance(Arc::new(WgpuWrapper::new(render_resources.instance))),
            )),
            synchronous_pipeline_compilation: false,
        })
    }
}
