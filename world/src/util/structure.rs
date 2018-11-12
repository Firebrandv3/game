// Library
use vek::*;

// Local
use cachegen::CacheGen;
use Gen;

#[allow(dead_code)]
pub fn dist_by_euc(p: Vec2<i64>) -> i64 { (p * p).sum() }

#[allow(dead_code)]
pub fn dist_by_axis(p: Vec2<i64>) -> i64 { p.map(|e| e.abs()).reduce_max() }

struct Producer;

impl<'a, F, S, O: Clone> Gen<(&'a StructureGen<O>, &'a S, &'a F)> for Producer
where
    F: Fn(&StructureGen<O>, Vec2<i64>, &S) -> O + Send + Sync + 'static,
{
    type In = Vec2<i64>;
    type Out = O;

    fn sample<'b>(
        &'b self,
        i: Self::In,
        (structure_gen, supplement, f): &'b (&'a StructureGen<O>, &'a S, &'a F),
    ) -> Self::Out {
        (**f)(structure_gen, i, supplement)
    }
}

pub struct StructureGen<O: 'static> {
    freq: u64,
    warp: u64,
    seed: u32,
    dist_func: fn(p: Vec2<i64>) -> i64,
    cache: CacheGen<Producer, Vec2<i64>, O>,
}

impl<O> StructureGen<O> {
    pub fn new(freq: u64, warp: u64, seed: u32, dist_func: fn(p: Vec2<i64>) -> i64) -> Self {
        Self {
            freq,
            warp,
            seed,
            dist_func,
            cache: CacheGen::new(Producer, 256),
        }
    }

    pub fn throw_dice(&self, pos: Vec2<i64>, seed: u32) -> u64 {
        // TODO: Make this actually good
        let next = 327387278321 ^ (self.seed + seed) as u64 * 1103515245 + 15341;
        let next = 327387278322 ^ (next + (pos.x + 3232782181) as u64) * 1103515245 + 12343;
        let next = 327387278321 ^ (next + (pos.y + 23728323237) as u64) * 1103515245 + 12541;
        next
    }
}

impl<'a, T: Clone, S, F> Gen<(&'a S, F)> for StructureGen<T>
where
    F: Fn(&Self, Vec2<i64>, &S) -> T + Send + Sync + 'static,
{
    type In = Vec2<i64>;
    type Out = (T, [T; 9]);

    fn sample(&self, pos: Vec2<i64>, (supplement, f): &(&S, F)) -> Self::Out {
        impl<O> StructureGen<O> {
            fn cell_pos(&self, cell_coord: Vec2<i64>) -> Vec2<i64> {
                cell_coord * self.freq as i64 + self.freq as i64 / 2 + if self.warp > 0 {
                    Vec2::new(self.throw_dice(cell_coord, 1337), self.throw_dice(cell_coord, 1338))
                        .map(|e| (e.mod_euc(self.warp)) as i64)
                        - self.warp as i64 / 2
                } else {
                    Vec2::zero()
                }
            }
        }

        let pos2di = Vec2::<i64>::from(pos);

        let cell_coord = pos2di.map(|e| e.div_euc(self.freq as i64));

        let mut near: [[Vec2<i64>; 3]; 3] = [[Vec2::zero(); 3]; 3];

        // TODO: Manually unroll this? Or not? Check to see if the compiler does automatically.
        let mut min = (cell_coord, std::i64::MAX);
        for x in -1..2 {
            for y in -1..2 {
                let cell_pos = self.cell_pos(cell_coord + Vec2::new(x, y));
                let dist = (self.dist_func)(cell_pos - pos2di);
                if dist < min.1 {
                    min = (cell_pos, dist);
                }

                near[(x + 1) as usize][(y + 1) as usize] = cell_pos;
            }
        }

        (
            self.cache.sample(min.0, &(self, *supplement, f)),
            [
                self.cache.sample(near[0][0], &(self, *supplement, f)),
                self.cache.sample(near[0][1], &(self, *supplement, f)),
                self.cache.sample(near[0][2], &(self, *supplement, f)),
                self.cache.sample(near[1][0], &(self, *supplement, f)),
                self.cache.sample(near[1][1], &(self, *supplement, f)),
                self.cache.sample(near[1][2], &(self, *supplement, f)),
                self.cache.sample(near[2][0], &(self, *supplement, f)),
                self.cache.sample(near[2][1], &(self, *supplement, f)),
                self.cache.sample(near[2][2], &(self, *supplement, f)),
            ],
        )
    }
}
