pub mod canvas;

use js_sys::{Array, Function, Object, Reflect}; // Added Function here
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::console;
use web_sys::{MessageEvent, Worker, WorkerOptions, WorkerType};

use crate::canvas::create_and_setup_canvas;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    web_sys::console::debug_1(&"Main started!".into());

    let canvas = create_and_setup_canvas().unwrap();
    let offscreen_canvas = canvas.transfer_control_to_offscreen().unwrap();
    web_sys::console::debug_1(&"Canvas control transferred to OffscreenCanvas.".into());

    let options = WorkerOptions::new();
    options.set_type(WorkerType::Classic);
    options.set_name("App");

    let worker = Worker::new_with_options("./worker.js", &options).unwrap();

    // --- 1. Forward Window Messages to Worker (With Transferables) ---

    let worker_clone = worker.clone();

    let on_window_msg = Closure::wrap(Box::new(move |event: MessageEvent| {
        let data = event.data();
        let ports = event.ports();

        console::log_3(&"Main -> Worker".into(), &event.data(), &event.ports());

        // Forward to worker
        if let Err(e) = worker_clone.post_message_with_transfer(&data, &ports) {
            web_sys::console::error_2(&"Failed to forward message to worker:".into(), &e);
        }
    }) as Box<dyn FnMut(MessageEvent)>);

    let window = web_sys::window().unwrap();
    window.add_event_listener_with_callback("message", on_window_msg.as_ref().unchecked_ref())?;
    on_window_msg.forget();

    // --- 2. Handle/Forward Worker Messages to Window (FIXED) ---

    // We capture the window object to post messages back to it
    let window_target = window.clone().parent().unwrap().unwrap();

    let on_worker_msg = Closure::wrap(Box::new(move |event: MessageEvent| {
        let data = event.data();
        let ports = event.ports();

        console::log_3(&"Worker -> Main".into(), &event.data(), &event.ports());

        if ports.length() > 0 {
            if let Err(e) = window_target.post_message_with_transfer(&data, "*", &ports) {
                web_sys::console::error_2(&"Failed to forward worker message:".into(), &e);
            }
        } else {
            if let Err(e) = window_target.post_message(&data, "*") {
                web_sys::console::error_2(&"Failed to forward worker message:".into(), &e);
            }
        }
    }) as Box<dyn FnMut(MessageEvent)>);

    worker.set_onmessage(Some(on_worker_msg.as_ref().unchecked_ref()));
    on_worker_msg.forget();

    // --- 3. Initial Setup (Canvas Transfer) ---

    let message_object = Object::new();
    Reflect::set(&message_object, &"type".into(), &"CANVAS".into())?;
    Reflect::set(&message_object, &"canvas".into(), &offscreen_canvas)?;

    let transfer_list = Array::of1(&offscreen_canvas);
    worker.post_message_with_transfer(&message_object, &transfer_list)?;

    web_sys::console::debug_1(&"Web Worker spawned and Canvas transferred.".into());

    Ok(())
}
