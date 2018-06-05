use std::thread::JoinHandle;
use std::net::ToSocketAddrs;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use nalgebra::{Vector2, Vector3, Matrix4, Translation3};

use client::{Client, ClientMode};
use camera::Camera;
use window::{RenderWindow, Event};
use model_object::{ModelObject, Constants};
use mesh::{Mesh, Vertex};
use region::Chunk;
use glutin::ElementState;

pub struct Game {
    running: AtomicBool,
    client: Arc<Client>,
    window: Arc<Mutex<RenderWindow>>,
    data: Mutex<Data>,
}

// "Data" includes mutable state
struct Data {
    camera: Camera,
    player_model: ModelObject,
    test_model: ModelObject,
    cursor_trapped: bool,
}

impl Game {
    pub fn new<B: ToSocketAddrs, R: ToSocketAddrs>(mode: ClientMode, alias: &str, bind_addr: B, remote_addr: R) -> Game {
        let mut window = RenderWindow::new();

        let chunk = Chunk::test((100, 100, 100));
        let test_mesh = Mesh::from(&chunk);

        let mut player_mesh = Mesh::new();
        player_mesh.add(&[
            Vertex { pos: [0., 1., 0.], norm: [0., 0., 1.], col: [1., 0., 0., 1.] },
            Vertex { pos: [-1., -1., 0.], norm: [0., 0., 1.], col: [0., 1., 0., 1.] },
            Vertex { pos: [1., -1., 0.], norm: [0., 0., 1.], col: [0., 0., 1., 1.] },

            Vertex { pos: [0., 1., 0.], norm: [0., 0., 1.], col: [1., 0., 0., 1.] },
            Vertex { pos: [1., -1., 0.], norm: [0., 0., 1.], col: [0., 0., 1., 1.] },
            Vertex { pos: [-1., -1., 0.], norm: [0., 0., 1.], col: [0., 1., 0., 1.] },
        ]);

        let client = Client::new(mode, alias.to_string(), bind_addr, remote_addr)
            .expect("Could not create new client");
        Client::start(client.clone());

        Game {
            running: AtomicBool::new(true),
            data: Mutex::new(Data {
                camera: Camera::new(),
                player_model: ModelObject::new(
                    window.renderer_mut(),
                    &player_mesh,
                ),
                test_model: ModelObject::new(
                    window.renderer_mut(),
                    &test_mesh,
                ),
                cursor_trapped: true,
            }),
            client,
            window: Arc::new(Mutex::new(window)),
        }
    }

    pub fn handle_window_events(&self) -> bool {
        self.window.lock().unwrap().handle_events(|event| {
            match event {
                Event::CloseRequest => self.running.store(false, Ordering::Relaxed),
                Event::CursorMoved { dx, dy } => {
                    let mut data = self.data.lock().unwrap();

                    if data.cursor_trapped {
                        data.camera.rotate_by(Vector2::<f32>::new(dx as f32 * 0.002, dy as f32 * 0.002))
                    }
                },
                Event::MouseWheel { dy, .. } => {
                    self.data.lock().unwrap().camera.zoom_by(-dy as f32);
                },
                Event::KeyboardInput { i, .. } => {
                    println!("pressed: {}", i.scancode);
                    match i.scancode {
                        1 => self.data.lock().unwrap().cursor_trapped = false,
                        17 => { //W
                            match i.state {
                                ElementState::Pressed => self.client.set_player_vel(Vector3::new(1.0, 0.0, 0.0)),
                                ElementState::Released => self.client.set_player_vel(Vector3::new(0.0, 0.0, 0.0)),
                            }
                        },
                        30 => { // A
                            match i.state {
                                ElementState::Pressed => self.client.set_player_vel(Vector3::new(0.0, -1.0, 0.0)),
                                ElementState::Released => self.client.set_player_vel(Vector3::new(0.0, 0.0, 0.0)),
                            }
                        },
                        31 => { // S
                            match i.state {
                                ElementState::Pressed => self.client.set_player_vel(Vector3::new(-1.0, 0.0, 0.0)),
                                ElementState::Released => self.client.set_player_vel(Vector3::new(0.0, 0.0, 0.0)),
                            }
                        },
                        32 => { // D
                            match i.state {
                                ElementState::Pressed => self.client.set_player_vel(Vector3::new(0.0, 1.0, 0.0)),
                                ElementState::Released => self.client.set_player_vel(Vector3::new(0.0, 0.0, 0.0)),
                            }
                        },
                        57 => { // Space
                            match i.state {
                                ElementState::Pressed => self.client.set_player_vel(Vector3::new(0.0, 0.0, 1.0)),
                                ElementState::Released => self.client.set_player_vel(Vector3::new(0.0, 0.0, 0.0)),
                            }
                        },
                        42 => { // Shift
                            match i.state {
                                ElementState::Pressed => self.client.set_player_vel(Vector3::new(0.0, 0.0, -1.0)),
                                ElementState::Released => self.client.set_player_vel(Vector3::new(0.0, 0.0, 0.0)),
                            }
                        },
                        _ => (),
                    }
                },
                Event::Resized { w, h } => {
                    self.data.lock().unwrap().camera.set_aspect_ratio(w as f32 / h as f32);
                },
                _ => {},
            }
        });

        self.running.load(Ordering::Relaxed)
    }

    pub fn render_frame(&self) {
        let mut window = self.window.lock().unwrap();

        window.renderer_mut().begin_frame();

        if let Some(uid) = self.client.player_entity_uid() {
            if let Some(e) = self.client.entities().get(&uid) {
                self.data.lock().unwrap().camera.set_focus(*e.pos());
            }
        }

        let camera_mats = self.data.lock().unwrap().camera.get_mats();

        // Render the test model
        window.renderer_mut().update_model_object(
            &self.data.lock().unwrap().test_model,
            Constants::new(&Matrix4::<f32>::identity(), &camera_mats.0, &camera_mats.1)
        );
        window.renderer_mut().render_model_object(&self.data.lock().unwrap().test_model);

        for (uid, entity) in self.client.entities().iter() {
            window.renderer_mut().update_model_object(
                &self.data.lock().unwrap().player_model,
                Constants::new(
                    &Translation3::<f32>::from_vector(*entity.pos()).to_homogeneous(),
                    &camera_mats.0,
                    &camera_mats.1
                )
            );
            window.renderer_mut().render_model_object(&self.data.lock().unwrap().player_model);
        }

        window.swap_buffers();
        window.renderer_mut().end_frame();
    }

    pub fn run(&self) {
        while self.handle_window_events() {
            self.render_frame();
        }
    }
}

impl Drop for Game {
    fn drop(&mut self) {
        Client::stop(self.client.clone());
    }
}