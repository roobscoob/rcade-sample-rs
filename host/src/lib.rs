pub mod canvas;

use js_sys::{Array, Object, Reflect};
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, Worker, WorkerOptions, WorkerType};

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

    // Set up worker message listener
    let worker_clone = worker.clone();
    let onmessage_callback = Closure::wrap(Box::new(move |event: MessageEvent| {
        let data = event.data();

        // Check if the message is just the string "request_plugin_channels"
        if let Some(msg) = data.as_string() {
            if msg == "request_plugin_channels" {
                web_sys::console::debug_1(
                    &"Worker requested plugin channels, forwarding to parent window.".into(),
                );

                // Forward the string message to parent window
                if let Some(window) = web_sys::window() {
                    if let Some(parent) = window.parent().ok().flatten() {
                        let _ =
                            parent.post_message(&JsValue::from_str("request_plugin_channels"), "*");
                    }
                }
            }
        }
    }) as Box<dyn FnMut(MessageEvent)>);

    worker.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();

    // Set up window message listener
    let window = web_sys::window().unwrap();
    let window_callback = Closure::wrap(Box::new(move |event: MessageEvent| {
        let data = event.data();

        if let Ok(obj) = data.dyn_into::<Object>() {
            if let Ok(msg_type) = Reflect::get(&obj, &JsValue::from_str("type")) {
                if msg_type.as_string() == Some("plugin_channel_created".to_string()) {
                    // Check if channel property exists
                    if let Ok(channel) = Reflect::get(&obj, &JsValue::from_str("channel")) {
                        if !channel.is_undefined() {
                            web_sys::console::debug_1(
                                &format!("Received plugin_channel_created from parent, forwarding to worker.")
                                    .into(),
                            );

                            if let Ok(ports) = Reflect::get(&event, &JsValue::from_str("ports")) {
                                if let Ok(ports_array) = ports.dyn_into::<Array>() {
                                    // Forward message to worker with ports transferred
                                    let _ =
                                        worker_clone.post_message_with_transfer(&obj, &ports_array);
                                } else {
                                    // If ports is not an array, send without transfer
                                    let _ = worker_clone.post_message(&obj);
                                }
                            } else {
                                // If no ports property, send without transfer
                                let _ = worker_clone.post_message(&obj);
                            }
                        }
                    }
                }
            }
        }
    }) as Box<dyn FnMut(MessageEvent)>);

    window.set_onmessage(Some(window_callback.as_ref().unchecked_ref()));
    window_callback.forget();
}
