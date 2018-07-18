#![feature(nll, euclidean_division)]

// Crates
#[macro_use]
extern crate log;
#[macro_use]
extern crate coord;
extern crate common;
extern crate region;

// Modules
mod error;
mod player;
mod callbacks;
mod tick;
mod world;
mod net;

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

    view_distance: i64,
}

impl<P: Payloads> Callback<ServerMessage> for Client<P> {
    fn recv(&self, msg: Result<ServerMessage, common::net::Error>) {
        self.handle_packet(msg.unwrap());
    }
}

impl<P: Payloads> Client<P> {
    pub fn new<U: ToSocketAddrs, GF: FnPayloadFunc<Chunk, P::Chunk, Output=P::Chunk>>(
        mode: ClientMode,
        alias: String,
        remote_addr: U,
        gen_payload: GF,
        view_distance: i64,
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

            chunk_mgr: VolMgr::new(CHUNK_SIZE, VolGen::new(world::gen_chunk, gen_payload)),

            callbacks: RwLock::new(Callbacks::new()),

            view_distance: view_distance.max(1).min(10),
        });

        *client.conn.callbackobj() = Some(client.clone());

        let client_ref = client.clone();
        client.jobs.set_root(client_ref);

        Ok(client)
    }

    fn set_status(&self, status: ClientStatus) {
        *self.status.write().unwrap() = status;
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
