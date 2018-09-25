// Standard
use std::sync::Arc;

// Library
use specs::{Builder, Component, Entity, EntityBuilder, VecStorage};
use vek::*;

// Project
use common::{
    manager::Manager,
    msg::{CompStore, PlayMode, ServerMsg, ServerPostOffice},
};
use region::ecs::{
    phys::{Dir, Pos, Vel},
    CreateUtil, NetComp,
};

// Local
use net::Client;
use Payloads;
use Server;

// Player

#[derive(Clone, Debug)]
pub struct Player {
    pub alias: String,
    pub mode: PlayMode,
}

impl Component for Player {
    type Storage = VecStorage<Self>;
}

impl NetComp for Player {
    fn to_store(&self) -> Option<CompStore> {
        Some(CompStore::Player {
            alias: self.alias.clone(),
            mode: self.mode,
        })
    }
}

// Server

impl<P: Payloads> Server<P> {
    pub(crate) fn create_player(
        &mut self,
        alias: String,
        mode: PlayMode,
        po: Manager<ServerPostOffice>,
    ) -> EntityBuilder {
        match mode {
            PlayMode::Headless => self.world.create_entity(),
            PlayMode::Character => self.world.create_character(alias.clone()),
        }.with(Player { alias, mode })
        .with(Client {
            postoffice: Arc::new(po),
        })
    }

    // pub(crate) fn update_player_entity(&mut self, player: Entity, pos: Vec3<f32>, vel: Vec3<f32>, dir: Vec2<f32>) {
    //     self.world.write_storage::<Pos>().get_mut(player).map(|p| {
    //         if Vec2::<f32>::from(p.0).distance(pos.into()) < 3.0 {
    //             p.0 = pos
    //         }
    //     }); // Basic sanity check
    //     self.world.write_storage::<Vel>().get_mut(player).map(|v| v.0 = vel);
    //     self.world.write_storage::<Dir>().get_mut(player).map(|c| c.0 = dir);
    // }
}
