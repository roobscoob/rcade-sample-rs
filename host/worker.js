// This script runs in the Web Worker context.

/** @type {OffscreenCanvas | null} */
self.RUST_OFFSCREEN_CANVAS = null;

// The 'self' global variable refers to the WorkerGlobalScope (the worker itself).
self.onmessage = function (event) {
    // We now expect a structured object from the main thread, e.g., { type: "CANVAS", canvas: OffscreenCanvas }
    const data = event.data;

    // Check if the canvas has already been initialized (prevents reprocessing late messages)
    if (self.RUST_OFFSCREEN_CANVAS !== null) {
        console.warn("Worker received a message, but OffscreenCanvas is already initialized.");
        return;
    }

    // Check for the specific 'CANVAS' command type and the OffscreenCanvas object
    if (typeof data === 'object' && data !== null && data.type === "CANVAS" && data.canvas instanceof OffscreenCanvas) {
        self.RUST_OFFSCREEN_CANVAS = data.canvas;
        console.debug("Worker successfully received OffscreenCanvas from 'CANVAS' command.", self.RUST_OFFSCREEN_CANVAS);

        // Stop listening for further messages now that the canvas is received.
        self.onmessage = null;
        console.debug("Message listener removed. Canvas is ready for use.");

        init()
    } else {
        // Log the received data structure to help debug potential mismatches in the Rust code
        console.error("Worker received an invalid message payload. Expected { type: 'CANVAS', canvas: OffscreenCanvas }", data);
    }
};

console.debug("Web Worker script loaded. Waiting for OffscreenCanvas message...");

function init() {
    importScripts("./app/rose-sample-rs.js");
    wasm_bindgen({ module_or_path: "./app/rose-sample-rs_bg.wasm" });
}