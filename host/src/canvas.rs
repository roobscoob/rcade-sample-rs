use wasm_bindgen::{JsCast, JsValue};
use web_sys::HtmlCanvasElement;

/// Helper function to create the canvas and set up its styles.
pub fn create_and_setup_canvas() -> Result<HtmlCanvasElement, JsValue> {
    // 1. Get the global window and document objects
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("Window not found"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("Document not found"))?;
    let body = document
        .body()
        .ok_or_else(|| JsValue::from_str("Body not found"))?;

    // 2. Create the canvas element
    let canvas = document
        .create_element("canvas")?
        .dyn_into::<HtmlCanvasElement>()?;

    // 3. Set the ID and initial dimensions (matching the CSS resolution concept)
    canvas.set_id("gameCanvas");

    // We'll set a fixed internal resolution for the pixel art effect.
    // The CSS will scale this up to the viewport size.
    const RESOLUTION_WIDTH: u32 = 320;
    const RESOLUTION_HEIGHT: u32 = 180;
    canvas.set_width(RESOLUTION_WIDTH);
    canvas.set_height(RESOLUTION_HEIGHT);

    // 4. Apply the necessary nearest-neighbor styling via CSS property
    let style = canvas.style();

    // Make it fill the viewport (CSS handles this, but it's good practice to set base styles)
    style.set_property("width", "100vw")?;
    style.set_property("height", "100vh")?;
    style.set_property("display", "block")?;

    // Crucial for nearest-neighbor scaling (pixel art effect)
    style.set_property("image-rendering", "pixelated")?;
    // Add vendor prefixes for wider compatibility
    style.set_property("image-rendering", "-moz-crisp-edges")?;
    style.set_property("image-rendering", "crisp-edges")?;

    // 5. Append the canvas to the document body
    body.append_child(&canvas)?;

    web_sys::console::debug_1(&"Canvas created and appended successfully!".into());

    Ok(canvas)
}
