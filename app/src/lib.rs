pub mod hook;

use rand::Rng;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::js_sys;
use web_sys::{DedicatedWorkerGlobalScope, OffscreenCanvas, OffscreenCanvasRenderingContext2d};

use crate::hook::get_offscreen_canvas;

// --- Config ---
const PARTICLE_COUNT: usize = 120;
const CONNECT_DISTANCE: f64 = 110.0;

struct Particle {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    color: String,
}

impl Particle {
    fn new(w: f64, h: f64) -> Self {
        let mut rng = rand::rng();
        Self {
            x: rng.random_range(0.0..w),
            y: rng.random_range(0.0..h),
            // Slightly faster velocity since there is no interaction to "push" them
            vx: rng.random_range(-1.0..1.0),
            vy: rng.random_range(-1.0..1.0),
            // Neon Cyan/Blue/Purple Palette
            color: format!("hsl({}, 80%, 30%)", rng.random_range(180..280)),
        }
    }

    fn update(&mut self, w: f64, h: f64) {
        self.x += self.vx;
        self.y += self.vy;

        // Bounce off walls
        if self.x < 0.0 || self.x > w {
            self.vx *= -1.0;
        }
        if self.y < 0.0 || self.y > h {
            self.vy *= -1.0;
        }
    }
}

struct SimulationState {
    ctx: OffscreenCanvasRenderingContext2d,
    particles: Vec<Particle>,
    width: f64,
    height: f64,
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    // Shared State
    let state: Rc<RefCell<Option<SimulationState>>> = Rc::new(RefCell::new(None));

    let canvas = get_offscreen_canvas().unwrap();
    let width = canvas.width() as f64;
    let height = canvas.height() as f64;

    let ctx = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<OffscreenCanvasRenderingContext2d>()
        .unwrap();

    let mut particles = Vec::with_capacity(PARTICLE_COUNT);
    for _ in 0..PARTICLE_COUNT {
        particles.push(Particle::new(width, height));
    }

    *state.borrow_mut() = Some(SimulationState {
        ctx,
        particles,
        width,
        height,
    });

    // Start the infinite loop
    request_animation_loop(state.clone());

    Ok(())
}

fn request_animation_loop(state: Rc<RefCell<Option<SimulationState>>>) {
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        if let Some(sim) = state.borrow_mut().as_mut() {
            render(sim);
        }
        request_animation_frame(f.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut()>));

    request_animation_frame(g.borrow().as_ref().unwrap());
}

fn render(sim: &mut SimulationState) {
    let ctx = &sim.ctx;

    // 1. Clear / Fade
    ctx.set_global_composite_operation("source-over").unwrap();
    ctx.set_fill_style_str("rgba(5, 5, 10, 0.2)"); // Very dark fade for trails
    ctx.fill_rect(0.0, 0.0, sim.width, sim.height);

    // 2. Glow Mode
    ctx.set_global_composite_operation("lighter").unwrap();

    // 3. Logic & Draw
    for i in 0..sim.particles.len() {
        sim.particles[i].update(sim.width, sim.height);

        // Draw Dot
        let p = &sim.particles[i];
        ctx.begin_path();
        ctx.set_fill_style_str(&p.color);
        ctx.arc(p.x, p.y, 2.0, 0.0, std::f64::consts::PI * 2.0)
            .unwrap();
        ctx.fill();

        // Draw Lines (The Constellation)
        for j in (i + 1)..sim.particles.len() {
            let p2 = &sim.particles[j];
            let dx = p.x - p2.x;
            let dy = p.y - p2.y;
            let dist_sq = dx * dx + dy * dy;

            // Avoid square root for performance unless necessary
            if dist_sq < (CONNECT_DISTANCE * CONNECT_DISTANCE) {
                let dist = dist_sq.sqrt();
                let alpha = 1.0 - (dist / CONNECT_DISTANCE);

                ctx.begin_path();
                ctx.set_stroke_style_str(&format!("rgba(100, 200, 255, {})", alpha / 10.0));
                ctx.set_line_width(0.5);
                ctx.move_to(p.x, p.y);
                ctx.line_to(p2.x, p2.y);
                ctx.stroke();
            }
        }
    }
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    let global = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();
    global
        .request_animation_frame(f.as_ref().unchecked_ref())
        .ok();
}
