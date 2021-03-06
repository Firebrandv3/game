#![feature(integer_atomics, duration_as_u128, label_break_value, specialization)]

// Crates
pub extern crate specs;

// Modules
pub mod api;
mod error;
mod msg;
pub mod net;
pub mod player;
mod tick;

// Reexports
pub use common::util::manager::Manager;
// Crate Reexports
pub use crate::error::Error;

// Standard
use std::{
    net::{TcpListener, ToSocketAddrs},
    sync::atomic::Ordering,
    time::Duration,
};

// Library
use parking_lot::RwLock;
use specs::{Entity, World};

// Project
use common::{
    ecs,
    util::{clock::Clock, manager::Managed, msg::ServerPostOffice},
};

// Local
use crate::{
    api::Api,
    net::{Client, DisconnectReason},
    player::Player,
};

pub trait Payloads: Send + Sync + 'static {
    type Chunk: Send + Sync + 'static;
    type Entity: Send + Sync + 'static;
    type Client: Send + Sync + 'static;

    fn on_player_connect(&self, _api: &dyn Api, _player: Entity) {}
    fn on_player_disconnect(&self, _api: &dyn Api, _player: Entity, _reason: DisconnectReason) {}
    fn on_chat_msg(&self, api: &dyn Api, player: Entity, text: &str) -> Option<String> {
        Some(format!(
            "[{}] {}",
            api.world()
                .read_storage::<Player>()
                .get(player)
                .map(|p| p.alias.as_str())
                .unwrap_or("<none"),
            text
        ))
    }
}

pub struct Server<P: Payloads> {
    listener: TcpListener,
    clock_tick_time: Duration,
    world: World,
    payload: P,
}

// Wrapper

// We use this wrapper to pass `Server` around without locking it
pub struct Wrapper<S>(RwLock<S>);

impl<S> Wrapper<S> {
    pub fn do_for<R, F: FnOnce(&S) -> R>(&self, f: F) -> R { f(&self.0.read()) }
    pub fn do_for_mut<R, F: FnOnce(&mut S) -> R>(&self, f: F) -> R { f(&mut self.0.write()) }
}

impl<P: Payloads> Server<P> {
    pub fn new<S: ToSocketAddrs>(payload: P, bind_addr: S) -> Result<Manager<Wrapper<Self>>, Error> {
        let mut world = ecs::create_world();
        world.register::<Client>();
        world.register::<Player>();

        Ok(Manager::init(Wrapper(RwLock::new(Server {
            listener: TcpListener::bind(bind_addr)?,
            clock_tick_time: Duration::from_millis(0),
            world,
            payload,
        }))))
    }
}

impl<P: Payloads> Managed for Wrapper<Server<P>> {
    fn init_workers(&self, mgr: &mut Manager<Self>) {
        // Incoming clients worker
        Manager::add_worker(mgr, |srv, running, mut mgr| {
            let listener = srv.do_for_mut(|srv| srv.listener.try_clone().expect("Failed to clone server TcpListener"));

            while let (Ok((stream, _addr)), true) = (listener.accept(), running.load(Ordering::Relaxed)) {
                // Convert the incoming stream to a postoffice ready to begin the connection handshake
                if let Ok(po) = ServerPostOffice::to_client(stream) {
                    Manager::add_worker(&mut mgr, move |srv, _, mgr| {
                        if let Ok(client) = net::auth_client(srv, po) {
                            net::handle_player_post(srv, client, mgr);
                        }
                    });
                }
            }
        });

        // Tick workers
        Manager::add_worker(mgr, |srv, running, _| {
            let mut clock = Clock::new(Duration::from_millis(20));
            while running.load(Ordering::Relaxed) {
                srv.do_for_mut(|srv| srv.tick_once(clock.reference_duration()));
                clock.tick();
                srv.do_for_mut(|srv| srv.clock_tick_time += clock.reference_duration());
            }
        });

        // Sync Time worker
        Manager::add_worker(mgr, |srv, running, _| {
            let mut clock = Clock::new(Duration::from_millis(60000));
            while running.load(Ordering::Relaxed) {
                srv.do_for_mut(|srv| srv.tick_time());
                clock.tick();
            }
        });
    }

    fn on_drop(&self, _: &mut Manager<Self>) {
        self.do_for(|srv| srv.listener.set_nonblocking(true))
            .expect("Failed to set nonblocking = true on server TcpListener");
    }
}
