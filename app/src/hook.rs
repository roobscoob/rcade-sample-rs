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
