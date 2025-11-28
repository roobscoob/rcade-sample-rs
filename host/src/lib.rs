pub mod canvas;

use js_sys::{Array, Object, Reflect};
use wasm_bindgen::prelude::*;
use web_sys::{Worker, WorkerOptions, WorkerType};

use crate::canvas::create_and_setup_canvas;

#[wasm_bindgen(start)]
pub fn start() {
    web_sys::console::debug_1(&"Main started!".into());

    let canvas = create_and_setup_canvas().unwrap();
    let offscreen_canvas = canvas.transfer_control_to_offscreen().unwrap();
    web_sys::console::debug_1(&"Canvas control transferred to OffscreenCanvas.".into());

    let options = WorkerOptions::new();

    options.set_type(WorkerType::Classic);
    options.set_name("App");

    let worker = Worker::new_with_options("./worker.js", &options).unwrap();
    let message_object = Object::new();

    // Set the 'type' property
    Reflect::set(
        &message_object,
        &JsValue::from_str("type"),
        &JsValue::from_str("CANVAS"),
    )
    .unwrap();

    // Set the 'canvas' property
    Reflect::set(
        &message_object,
        &JsValue::from_str("canvas"),
        &offscreen_canvas,
    )
    .unwrap();

    // Create the transfer list (only the OffscreenCanvas needs to be transferred)
    let transfer_list = Array::of1(&offscreen_canvas);

    // Post the message object and transfer the control
    worker
        .post_message_with_transfer(&message_object, &transfer_list)
        .unwrap();

    web_sys::console::debug_1(&"Web Worker spawned and object message {{type: CANVAS, canvas: OffscreenCanvas}} transferred successfully.".into());
}
