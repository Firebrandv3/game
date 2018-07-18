// Library
use coord::prelude::*;

// Project
use region::{VolState, Chunk};

// Local
use {Client, Payloads, CHUNK_SIZE};

pub(crate) fn gen_chunk(pos: Vec2<i64>) -> Chunk {
    Chunk::test(vec3!(pos.x * CHUNK_SIZE, pos.y * CHUNK_SIZE, 0), vec3!(CHUNK_SIZE, CHUNK_SIZE, 256))
}

impl<P: Payloads> Client<P> {
    pub(crate) fn update_chunks(&self) {
        if let Some(uid) = self.player().entity_uid {
            if let Some(player_entity) = self.entities_mut().get_mut(&uid) {
                let player_chunk = player_entity
                    .pos()
                    .map(|e| e as i64)
                    .div_euc(vec3!([CHUNK_SIZE; 3]));

                // Generate chunks around the player
                for i in player_chunk.x - self.view_distance .. player_chunk.x + self.view_distance + 1 {
                    for j in player_chunk.y - self.view_distance .. player_chunk.y + self.view_distance + 1 {
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
                    if (pos - vec2!(player_chunk.x, player_chunk.y)).snake_length() > self.view_distance * 2 {
                        self.jobs.do_once(move |c| c.chunk_mgr().remove(pos));
                    }
                }
            }
        }
    }
}
