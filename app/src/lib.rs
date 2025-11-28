pub mod hook;

use rand::Rng;
use rcade_plugin_input_classic::ClassicController;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::console;
use web_sys::js_sys;
use web_sys::{DedicatedWorkerGlobalScope, OffscreenCanvasRenderingContext2d};

use crate::hook::get_offscreen_canvas;

#[wasm_bindgen(start)]
pub async fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let canvas = get_offscreen_canvas().unwrap();
    let width = canvas.width() as f64;
    let height = canvas.height() as f64;

    let ctx = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<OffscreenCanvasRenderingContext2d>()
        .unwrap();

    let controller = ClassicController::acquire().await.unwrap();

    console::log_1(&"Got controller".into());

    // Start the infinite loop
    request_animation_loop(ctx, controller);

    Ok(())
}

fn request_animation_loop(
    canvas: OffscreenCanvasRenderingContext2d,
    controller: ClassicController,
) {
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        let state = controller.state();

        if state.player1_up {
            canvas.set_fill_style_str("red");
            canvas.fill_rect(10.0, 10.0, 20.0, 20.0);
        } else {
            canvas.set_fill_style_str("blue");
            canvas.fill_rect(10.0, 10.0, 20.0, 20.0);
        }

        request_animation_frame(f.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut()>));

    request_animation_frame(g.borrow().as_ref().unwrap());
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    let global = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();
    global
        .request_animation_frame(f.as_ref().unchecked_ref())
        .ok();
}
