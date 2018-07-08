// Ui
use ui::Ui;

// Standard
use std::net::ToSocketAddrs;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::f32::consts::PI;
use std::collections::HashMap;
//use std::f32::{sin, cos};

// Import contants
use client::CHUNK_SIZE;

// Library
use nalgebra::{Vector2, Vector3, Translation3, Rotation3, convert, dot};
use coord::prelude::*;
use glutin::{ElementState, VirtualKeyCode};
use dot_vox;

// Project
use client;
use client::{Client, ClientMode};

// Local
use camera::Camera;
use window::{RenderWindow, Event};
use model_object::{ModelObject, Constants};
use mesh::{Mesh};
use region::{Chunk, VolState};
use keybinds::Keybinds;
use key_state::KeyState;
use vox::vox_to_model;

struct Payloads {}
impl client::Payloads for Payloads {
    type Chunk = (Mesh, Option<ModelObject>);
}

pub struct Game {
    running: AtomicBool,
    client: Arc<Client<Payloads>>,
    window: RenderWindow,
    data: Mutex<Data>,
    camera: Mutex<Camera>,
    key_state: Mutex<KeyState>,
    ui: Mutex<Ui>,
    keys: Keybinds,
}

// "Data" includes mutable state
struct Data {
    player_model: ModelObject,
    other_player_model: ModelObject,
}

fn gen_payload(chunk: &Chunk) -> <Payloads as client::Payloads>::Chunk {
    (Mesh::from(chunk), None)
}

impl Game {
    pub fn new<R: ToSocketAddrs>(mode: ClientMode, alias: &str, remote_addr: R) -> Game {
        let window = RenderWindow::new();

        let vox = dot_vox::load("data/vox/3.vox").unwrap();
        let voxmodel = vox_to_model(vox);

        let player_mesh = Mesh::from_with_offset(&voxmodel, vec3!(-10.0, -4.0, 0.0));

        let player_model = ModelObject::new(
            &mut window.renderer_mut(),
            &player_mesh,
        );

        let vox = dot_vox::load("data/vox/5.vox").unwrap();
        let voxmodel = vox_to_model(vox);

        let other_player_mesh = Mesh::from(&voxmodel);

        let other_player_model = ModelObject::new(
            &mut window.renderer_mut(),
            &other_player_mesh,
        );

        // Contruct the UI
        let window_dims = window.get_size();

        let mut ui = Ui::new(&mut window.renderer_mut(), window_dims);

        Game {
            data: Mutex::new(Data {
                player_model,
                other_player_model,
            }),
            running: AtomicBool::new(true),
            client: Client::new(mode, alias.to_string(), remote_addr, gen_payload)
				.expect("Could not create new client"),
            window,
            camera: Mutex::new(Camera::new()),
            key_state: Mutex::new(KeyState::new()),
            ui: Mutex::new(ui),
            keys: Keybinds::new(),
        }
    }

    pub fn handle_window_events(&self) -> bool {
        self.window.handle_events(|event| {
            match event {
                Event::CloseRequest => self.running.store(false, Ordering::Relaxed),
                Event::CursorMoved { dx, dy } => {
                    let data = self.data.lock().unwrap();

                    if self.window.cursor_trapped().load(Ordering::Relaxed) {
                        //debug!("dx: {}, dy: {}", dx, dy);
                        self.camera.lock().unwrap().rotate_by(Vector2::<f32>::new(dx as f32 * 0.002, dy as f32 * 0.002));
                    }
                },
                Event::MouseWheel { dy, .. } => {
                    self.camera.lock().unwrap().zoom_by((-dy / 4.0) as f32);
                },
                Event::KeyboardInput { i, .. } => {
                    // Helper function to determine scancode equality
                    fn keypress_eq(key: &Option<u32>, scancode: u32) -> bool {
                        key.map(|sc| sc == scancode).unwrap_or(false)
                    }

                    // Helper variables to clean up code. Add any new input modes here.
                    let general = &self.keys.general;
                    let mount = &self.keys.mount;

                    // General inputs -------------------------------------------------------------
                    if keypress_eq(&general.pause, i.scancode) { // Default: Escape (free cursor)
                        self.window.cursor_trapped().store(false, Ordering::Relaxed)
                    } else if keypress_eq(&general.use_item, i.scancode) { // Default: Ctrl+Q (quit) (temporary)
                        if i.modifiers.ctrl {
                            self.running.store(false, Ordering::Relaxed);
                        }
                    } else if keypress_eq(&general.forward, i.scancode) {
                        self.key_state.lock().unwrap().up = match i.state { // Default: W (up)
                            ElementState::Pressed => true,
                            ElementState::Released => false,
                        }
                    } else if keypress_eq(&general.left, i.scancode) {
                        self.key_state.lock().unwrap().left = match i.state { // Default: A (left)
                            ElementState::Pressed => true,
                            ElementState::Released => false,
                        }
                    } else if keypress_eq(&general.back, i.scancode) {
                        self.key_state.lock().unwrap().down = match i.state { // Default: S (down)
                            ElementState::Pressed => true,
                            ElementState::Released => false,
                        }
                    } else if keypress_eq(&general.right, i.scancode) {
                        self.key_state.lock().unwrap().right = match i.state { // Default: D (right)
                            ElementState::Pressed => true,
                            ElementState::Released => false,
                        }
                    } else if keypress_eq(&general.fly, i.scancode) {
                        self.key_state.lock().unwrap().fly = match i.state { // Default: Space (fly)
                            ElementState::Pressed => true,
                            ElementState::Released => false,
                        }
                    } else if keypress_eq(&general.fall, i.scancode) {
                        self.key_state.lock().unwrap().fall = match i.state { // Default: Shift (fall)
                            ElementState::Pressed => true,
                            ElementState::Released => false,
                        }
                    }
                    // ----------------------------------------------------------------------------

                    // Mount inputs ---------------------------------------------------------------
                    // placeholder
                    // ----------------------------------------------------------------------------

                    // UI Code
                    self.ui.lock().unwrap().ui_event_keyboard_input(i);
                },
                Event::Resized { w, h } => {
                    self.camera.lock().unwrap().set_aspect_ratio(w as f32 / h as f32);
                    self.ui.lock().unwrap().ui_event_window_resize(w, h);
                },
                Event::MouseButton { state, button } => {
                    self.ui.lock().unwrap().ui_event_mouse_button(state, button);
                },
                Event::CursorPosition { x, y} => {
                    self.ui.lock().unwrap().ui_event_mouse_pos(x, y);
                },
                Event::Character { ch } => {
                    self.ui.lock().unwrap().ui_event_character(ch);
                }
                Event::Raw { event } => {
//                    println!("{:?}", event);
                },
            }
        });

        // Calculate movement player movement vector
        let ori = *self.camera.lock().unwrap().ori();
        let unit_vecs = (
            Vector2::new(f32::cos(-ori.x), f32::sin(-ori.x)),
            Vector2::new(f32::sin(ori.x), f32::cos(ori.x))
        );
        let dir_vec = self.key_state.lock().unwrap().dir_vec();
        let mov_vec = unit_vecs.0 * dir_vec.x + unit_vecs.1 * dir_vec.y;
        let fly_vec = self.key_state.lock().unwrap().fly_vec();

        //self.client.player_mut().dir_vec = vec3!(mov_vec.x, mov_vec.y, fly_vec);

        let mut entries = self.client.entities_mut();
        if let Some(eid) = self.client.player().entity_uid {
            if let Some(player_entry) = entries.get_mut(&eid) {
                player_entry.ctrl_vel_mut().x = mov_vec.x;
                player_entry.ctrl_vel_mut().y = mov_vec.y;
                player_entry.ctrl_vel_mut().z = fly_vec * 5.0;
                let ori = *self.camera.lock().unwrap().ori();
                *player_entry.look_dir_mut() = vec2!(ori.x, ori.y);
            }
        }

        self.running.load(Ordering::Relaxed)
    }

    pub fn model_chunks(&self) {
        for (pos, vol) in self.client.chunk_mgr().volumes().iter() {
            if let VolState::Exists(ref chunk, ref mut payload) = *vol.write().unwrap() {
                if let None = payload.1 {
                    payload.1 = Some(ModelObject::new(
                        &mut self.window.renderer_mut(),
                        &payload.0,
                    ));
                }
            }
        }
    }

    pub fn render_frame(&self) {
        let mut renderer = self.window.renderer_mut();
        renderer.begin_frame();

        if let Some(uid) = self.client.player().entity_uid {
            if let Some(e) = self.client.entities().get(&uid) {
                self.camera.lock().unwrap().set_focus(Vector3::<f32>::new(e.pos().x, e.pos().y, e.pos().z + 1.75)); // TODO: Improve this
            }
        }

        let camera_mats = self.camera.lock().unwrap().get_mats();
        let camera_ori = self.camera.lock().unwrap().ori();

        for (pos, vol) in self.client.chunk_mgr().volumes().iter() {
            if let VolState::Exists(ref chunk, ref payload) = *vol.read().unwrap() {
                if let Some(ref model) = payload.1 {
                    let model_mat = &Translation3::<f32>::from_vector(Vector3::<f32>::new(
                        (pos.x * CHUNK_SIZE) as f32,
                        (pos.y * CHUNK_SIZE) as f32,
                        0.0
                    )).to_homogeneous();

                    renderer.update_model_object(
                        &model,
                        Constants::new(
                            &model_mat, // TODO: Improve this
                            &camera_mats.0,
                            &camera_mats.1,
                        )
                    );
                    renderer.render_model_object(&model);
                }
            }
        }

        for (eid, entity) in self.client.entities().iter() {
            let model_mat = &Translation3::<f32>::from_vector(Vector3::<f32>::new(entity.pos().x, entity.pos().y, entity.pos().z)).to_homogeneous();
            let rot = Rotation3::<f32>::new(Vector3::<f32>::new(0.0, 0.0, PI - entity.look_dir().x)).to_homogeneous();
            let model_mat = model_mat * rot;
            let mut data = self.data.lock().unwrap();
            let ref mut model;
            match self.client.player().entity_uid {
                Some(uid) if uid == *eid => model = &mut data.player_model,
                _ => model = &mut data.other_player_model,
            }
            renderer.update_model_object(
                &model,
                Constants::new(
                    &model_mat, // TODO: Improve this
                    &camera_mats.0,
                    &camera_mats.1,
                )
            );
            renderer.render_model_object(&model);
        }

        // Draw ui
        self.ui.lock().unwrap().render(&mut renderer, &self.window.get_size());

        self.window.swap_buffers();
        renderer.end_frame();
    }

    pub fn run(&self) {
        while self.handle_window_events() {
            self.model_chunks();
            self.render_frame();
        }

		self.client.shutdown();
    }
}
