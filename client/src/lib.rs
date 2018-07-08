#![feature(nll, euclidean_division)]

// Crates
#[macro_use]
extern crate log;
#[macro_use]
extern crate coord;
extern crate common;
extern crate region;

// Modules
mod player;
mod callbacks;
mod error;

// Reexport
pub use common::net::ClientMode;
pub use region::{Volume, Voxel, Chunk, Block, FnPayloadFunc};

// Constants
pub const CHUNK_SIZE: i64 = 32;

// Standard
use std::thread;
use std::time;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard, Mutex};
use std::collections::HashMap;
use std::net::{ToSocketAddrs};

// Library
use coord::prelude::*;

// Project
use region::{Entity, VolMgr, VolGen, VolState};
use common::{get_version, Uid, Jobs, JobHandle};
use common::net::{Connection, ServerMessage, ClientMessage, Callback, UdpMgr};

// Local
use player::Player;
use callbacks::Callbacks;
use error::Error;

const VIEW_DISTANCE: i64 = 3;

#[derive(Copy, Clone, PartialEq)]
pub enum ClientStatus {
    Connecting,
    Connected,
    Timeout,
    Disconnected,
}

pub trait Payloads: 'static {
    type Chunk: Send + Sync + 'static;
}

pub struct Client<P: Payloads> {
    pub jobs: Jobs<Client<P>>,
    run_job: Mutex<Option<JobHandle<()>>>,

    status: RwLock<ClientStatus>,
    conn: Arc<Connection<ServerMessage>>,

    player: RwLock<Player>,
    entities: RwLock<HashMap<Uid, Entity>>,

    chunk_mgr: VolMgr<Chunk, <P as Payloads>::Chunk>,

    callbacks: RwLock<Callbacks>,
}

impl<P: Payloads> Callback<ServerMessage> for Client<P> {
    fn recv(&self, msg: Result<ServerMessage, common::net::Error>) {
        self.handle_packet(msg.unwrap());
    }
}

fn gen_chunk(pos: Vec2<i64>) -> Chunk {
    Chunk::test(vec3!(pos.x * CHUNK_SIZE, pos.y * CHUNK_SIZE, 0), vec3!(CHUNK_SIZE, CHUNK_SIZE, 128))
}

impl<P: Payloads> Client<P> {
    pub fn new<U: ToSocketAddrs, GF: FnPayloadFunc<Chunk, P::Chunk, Output=P::Chunk>>(
        mode: ClientMode,
        alias: String,
        remote_addr: U,
        gen_payload: GF
    ) -> Result<Arc<Client<P>>, Error>
    {
        let conn = Connection::new::<U>(&remote_addr, Box::new(|_m| {}), None, UdpMgr::new())?;
        conn.send(ClientMessage::Connect{ mode, alias: alias.clone(), version: get_version() });
        Connection::start(&conn);

        let client = Arc::new(Client {
            jobs: Jobs::new(),
            run_job: Mutex::new(None),

            status: RwLock::new(ClientStatus::Connecting),
            conn,

            player: RwLock::new(Player::new(alias)),
            entities: RwLock::new(HashMap::new()),

            chunk_mgr: VolMgr::new(CHUNK_SIZE, VolGen::new(gen_chunk, gen_payload)),

            callbacks: RwLock::new(Callbacks::new()),
        });

        *client.conn.callbackobj() = Some(client.clone());

        let client_ref = client.clone();
        client.jobs.set_root(client_ref);

        Ok(client)
    }

    fn set_status(&self, status: ClientStatus) {
        *self.status.write().unwrap() = status;
    }

    fn handle_packet(&self, packet: ServerMessage) {
        match packet {
            ServerMessage::Connected { entity_uid, version } => {
                if version == get_version() {
                    if let Some(uid) = entity_uid {
                        if !self.entities().contains_key(&uid) {
                            self.entities_mut().insert(uid, Entity::new(vec3!(0.0, 0.0, 0.0), vec3!(0.0, 0.0, 0.0), vec3!(0.0, 0.0, 0.0), vec2!(0.0, 0.0)));
                        }
                    }
                    self.player_mut().entity_uid = entity_uid;
                    self.set_status(ClientStatus::Connected);
                    info!("Connected!");
                } else {
                    warn!("Server version mismatch: server is version {}. Disconnected.", version);
                    self.set_status(ClientStatus::Disconnected);
                }
            },
            ServerMessage::Kicked { reason } => {
                warn!("Server kicked client for {}", reason);
                self.set_status(ClientStatus::Disconnected);
            }
            ServerMessage::Shutdown => self.set_status(ClientStatus::Disconnected),
            ServerMessage::RecvChatMsg { alias, msg } => self.callbacks().call_recv_chat_msg(&alias, &msg),
            ServerMessage::EntityUpdate { uid, pos, vel, ctrl_vel, look_dir } => {
                let mut entities = self.entities_mut();
                match entities.get_mut(&uid) {
                    Some(e) => {
                        *e.pos_mut() = pos;
                        *e.vel_mut() = vel;
                        *e.ctrl_vel_mut() = ctrl_vel;
                        *e.look_dir_mut() = look_dir;
                    }
                    None => { entities.insert(uid, Entity::new(pos, vel, ctrl_vel, look_dir)); },
                }
            },
            ServerMessage::Ping => self.conn.send(ClientMessage::Ping),
            _ => {},
        }
    }

    fn update_chunks(&self) {
        if let Some(uid) = self.player().entity_uid {
            if let Some(player_entity) = self.entities_mut().get_mut(&uid) {
                let player_chunk = player_entity
                    .pos()
                    .map(|e| e as i64)
                    .div_euc(vec3!([CHUNK_SIZE; 3]));

                // Generate chunks around the player
                for i in player_chunk.x - VIEW_DISTANCE .. player_chunk.x + VIEW_DISTANCE + 1 {
                    for j in player_chunk.y - VIEW_DISTANCE .. player_chunk.y + VIEW_DISTANCE + 1 {
                        if !self.chunk_mgr().contains(vec2!(i, j)) {
                            self.chunk_mgr().gen(vec2!(i, j));
                        }
                    }
                }

                // Remove chunks that are too far from the player
                // TODO: Could be more efficient (maybe? careful: deadlocks)
                let chunk_pos = self.chunk_mgr()
                    .volumes()
                    .keys()
                    .map(|p| *p)
                    .collect::<Vec<_>>();
                for pos in chunk_pos {
                    // What?! Don't use snake_length
                    if (pos - vec2!(player_chunk.x, player_chunk.y)).snake_length() > VIEW_DISTANCE * 2 {
                        self.chunk_mgr().remove(pos);
                    }
                }
            }
        }
    }

    fn update_physics(&self, dt: f32) {
        // Apply gravity to the play if they are both within a loaded chunk and have ground beneath their feet
        // TODO: We should be able to make this much smaller
        if let Some(uid) = self.player().entity_uid {
            if let Some(player_entity) = self.entities_mut().get_mut(&uid) {
                let player_chunk = player_entity
                    .pos()
                    .map(|e| e as i64)
                    .div_euc(vec3!([CHUNK_SIZE; 3]));

                // Apply gravity to the player
                if let Some(c) = self.chunk_mgr().at(vec2!(player_chunk.x, player_chunk.y)) {
                    if let VolState::Exists(_, _) = *c.read().unwrap() {
                        let _below_feet = *player_entity.pos() - vec3!(0.0, 0.0, -0.1);
                        if player_entity // Get the player's...
                            .get_lower_aabb() // ...bounding box...
                            .shift_by(vec3!(0.0, 0.0, -0.1)) // ...move it a little below the player...
                            .collides_with(self.chunk_mgr()) { // ...and check whether it collides with the ground.
                            player_entity.vel_mut().z = 0.0;
                        } else {
                            player_entity.vel_mut().z -= 0.15; // Apply gravity
                        }
                    }
                } else {
                    player_entity.vel_mut().z = 0.0;
                }
            }
        }

        // Move all entities, avoiding collisions
        for (_uid, entity) in self.entities_mut().iter_mut() {
            // First, calculate the change in position assuming no external influences
            let mut dpos = (*entity.vel() + *entity.ctrl_vel()) * dt;

            // Resolve collisions with the terrain, altering the change in position accordingly
            dpos = entity.get_upper_aabb().resolve_with(self.chunk_mgr(), dpos);

            // Change the entity's position
            *entity.pos_mut() += dpos;

            // Make the player hop up 1-block steps
            if entity.get_lower_aabb().collides_with(self.chunk_mgr()) {
                entity.pos_mut().z += 0.2;
            }
        }
    }

    fn update_server(&self) {
        // Update the server with information about the player
        if let Some(uid) = self.player().entity_uid {
            if let Some(player_entity) = self.entities().get(&uid) {
                self.conn.send(ClientMessage::PlayerEntityUpdate {
                    pos: *player_entity.pos(),
                    vel: *player_entity.vel(),
                    ctrl_vel: *player_entity.ctrl_vel(),
                    look_dir: *player_entity.look_dir(),
                });
            }
        }
    }

    fn tick(&self, dt: f32) -> bool {
        self.update_chunks();
        self.update_physics(dt);
        self.update_server();

        *self.status() != ClientStatus::Disconnected
    }

    pub fn start(&self) {
        if self.run_job.lock().unwrap().is_none() {
            *self.run_job.lock().unwrap() = Some(self.jobs.do_loop(|c| {
                thread::sleep(time::Duration::from_millis(20));
                c.tick(0.2)
            }));
        }
    }

    pub fn shutdown(&self) {
        self.conn.send(ClientMessage::Disconnect);
        self.set_status(ClientStatus::Disconnected);
        if let Some(jh) = self.run_job.lock().unwrap().take() {
            jh.await();
        }
    }

    pub fn send_chat_msg(&self, msg: String) {
        self.conn.send(ClientMessage::ChatMsg { msg })
    }

    pub fn send_cmd(&self, cmd: String) {
        self.conn.send(ClientMessage::SendCmd { cmd })
    }

    pub fn chunk_mgr<'a>(&'a self) -> &'a VolMgr<Chunk, P::Chunk> { &self.chunk_mgr }

    pub fn status<'a>(&'a self) -> RwLockReadGuard<'a, ClientStatus> { self.status.read().unwrap() }

    pub fn callbacks<'a>(&'a self) -> RwLockReadGuard<'a, Callbacks> { self.callbacks.read().unwrap() }

    pub fn player<'a>(&'a self) -> RwLockReadGuard<'a, Player> { self.player.read().unwrap() }
    pub fn player_mut<'a>(&'a self) -> RwLockWriteGuard<'a, Player> { self.player.write().unwrap() }

    pub fn entities<'a>(&'a self) -> RwLockReadGuard<'a, HashMap<Uid, Entity>> { self.entities.read().unwrap() }
    pub fn entities_mut<'a>(&'a self) -> RwLockWriteGuard<'a, HashMap<Uid, Entity>> { self.entities.write().unwrap() }

    pub fn player_entity<'a>(&'a self) -> Option<&'a Entity> {
        unimplemented!();
        //self.player().entity_uid.and_then(|uid| self.entities().map(|e| e.get(&uid)))
    }
}
