use std::{ptr::NonNull, thread::ThreadId};

use bevy::{
    prelude::*,
    window::{RawHandleWrapper, WindowWrapper},
};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use wasm_bindgen::{JsCast, JsValue, prelude::wasm_bindgen};
use web_sys::OffscreenCanvas;

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
