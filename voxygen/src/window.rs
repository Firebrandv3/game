use gfx_window_glutin;
use glutin;

use glutin::{EventsLoop, WindowBuilder, ContextBuilder, GlContext, GlRequest, GlWindow, WindowEvent, CursorState};
use glutin::Api::OpenGl;

use renderer::{Renderer, ColorFormat, DepthFormat};

use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::sync::atomic::{AtomicBool, Ordering};

pub enum Event {
    CloseRequest,
    CursorMoved { dx: f64, dy: f64 },
    MouseWheel { dx: f64, dy: f64, modifiers: glutin::ModifiersState },
    KeyboardInput { i: glutin::KeyboardInput, device: glutin::DeviceId },
    Resized { w: u32, h: u32 },
}

pub struct RenderWindow {
    events_loop: Mutex<EventsLoop>,
    gl_window: Mutex<GlWindow>,
    renderer: RwLock<Renderer>,
    cursor_trapped: AtomicBool,
}

impl RenderWindow {
    pub fn new() -> RenderWindow {
        let events_loop = Mutex::new(EventsLoop::new());
        let win_builder = WindowBuilder::new()
            .with_title("Veloren (Voxygen)")
            .with_dimensions(800, 500)
            .with_maximized(false);

        let ctx_builder = ContextBuilder::new()
            .with_gl(GlRequest::Specific(OpenGl, (3, 2)))
            .with_vsync(true);

        let (gl_window, device, factory, color_view, depth_view) =
            gfx_window_glutin::init::<ColorFormat, DepthFormat>(win_builder, ctx_builder, &events_loop.lock().unwrap());

        RenderWindow {
            events_loop,
            gl_window: Mutex::new(gl_window),
            renderer: RwLock::new(Renderer::new(device, factory, color_view, depth_view)),
            cursor_trapped: AtomicBool::new(true),
        }
    }

    pub fn renderer<'a>(&'a self) -> RwLockReadGuard<'a, Renderer> { self.renderer.read().unwrap() }
    pub fn renderer_mut<'a>(&'a self) -> RwLockWriteGuard<'a, Renderer> { self.renderer.write().unwrap() }

    pub fn cursor_trapped(&self) -> &AtomicBool {
        &self.cursor_trapped
    }

    pub fn handle_events<'a, F: FnMut(Event)>(&self, mut func: F) {
        // We need to mutate these inside the closure, so we take a mutable reference
        let gl_window = &mut self.gl_window.lock().unwrap();
        let events_loop = &mut self.events_loop.lock().unwrap();

        events_loop.poll_events(|event| {
            if let glutin::Event::WindowEvent { event, .. } = event {
                match event {
                    WindowEvent::CursorMoved { position, .. } => {
                        match gl_window.get_inner_size() {
                            Some((x, y)) => {
                                func(Event::CursorMoved {
                                    dx: position.0 - x as f64 * 0.5,
                                    dy: position.1 - y as f64 * 0.5,
                                });
                                // TODO: Should we handle this result?
                                if self.cursor_trapped.load(Ordering::Relaxed) {
                                    // ratio calculated every event because window might be moved to other monitor
                                    // kept as float because of fractional hidpi factors
                                    let center_ratio = 2. * gl_window.hidpi_factor();
                                    let _ = gl_window.set_cursor_position((x as f32 / center_ratio) as i32, (y as f32 / center_ratio) as i32);
                                    let _ = gl_window.set_cursor_state(CursorState::Hide);
                                } else {
                                    let _ = gl_window.set_cursor_state(CursorState::Normal);
                                }
                            },
                            None => {},
                        }
                    },
                    WindowEvent::Resized { 0: w, 1: h } => {
                        let mut color_view = self.renderer.read().unwrap().color_view().clone();
                        let mut depth_view = self.renderer.read().unwrap().depth_view().clone();
                        gfx_window_glutin::update_views(
                            &gl_window,
                            &mut color_view,
                            &mut depth_view,
                        );
                        self.renderer.write().unwrap().set_views(color_view, depth_view);
                        func(Event::Resized {
                            w,
                            h,
                        });
                    },
                    WindowEvent::MouseWheel { delta, modifiers, .. } => {
                        let dx: f64;
                        let dy: f64;
                        match delta {
                            glutin::MouseScrollDelta::LineDelta(x,y) => {
                                dx = f64::from(x) * 8.0;
                                dy = f64::from(y) * 8.0;
                            },
                            glutin::MouseScrollDelta::PixelDelta(x,y) => {
                                dx = x.into();
                                dy = y.into();
                            },
                        }
                        func(Event::MouseWheel {
                            dx,
                            dy,
                            modifiers,
                        });
                    },
                    WindowEvent::KeyboardInput { device_id, input } => {
                        // keeping the device_id here to allow players using multiple keyboards
                        func(Event::KeyboardInput {
                            device: device_id,
                            i: input,
                        });
                    },
                    WindowEvent::MouseInput { device_id, state, button, modifiers } => {
                        if button == glutin::MouseButton::Left {
                            self.cursor_trapped.store(true, Ordering::Relaxed);
                            let _ = gl_window.set_cursor_state(CursorState::Hide);
                        }
                    },
                    WindowEvent::CloseRequested => func(Event::CloseRequest),
                    _ => {},
                }
            }
        });
    }

    pub fn swap_buffers(&self) {
        self.gl_window.lock().unwrap().swap_buffers().expect("Failed to swap window buffers");
    }
}
