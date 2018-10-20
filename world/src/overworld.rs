// Standard
use std::ops::{Add, Sub, Div, Mul, Neg, Rem};

// Library
use vek::*;
use noise::{NoiseFn, SuperSimplex, HybridMulti, Seedable, MultiFractal};

// Local
use Gen;
use cachegen::CacheGen;

#[derive(Copy, Clone)]
pub struct Sample {
    pub dry: f64,
    pub temp: f64,
    pub chaos: f64,

    pub river: f64,
    pub hill: f64,
    pub ridge: f64,
    pub cliff_height: f64,
}

pub struct OverworldGen {
    dry_nz: HybridMulti,
    temp_nz: HybridMulti,
    temp_vari_nz: HybridMulti,
    chaos_nz: SuperSimplex,
    hill_nz: HybridMulti,
    ridge_nz: HybridMulti,
    cliff_height_nz: SuperSimplex,
}

impl OverworldGen {
    pub fn new() -> CacheGen<Self> {
        let mut seed = 0;
        let mut new_seed = || { seed += 1; seed };

        CacheGen::new(Self {
            dry_nz: HybridMulti::new()
                .set_seed(new_seed())
                .set_octaves(4),
            temp_nz: HybridMulti::new()
                .set_seed(new_seed())
                .set_octaves(3),
            temp_vari_nz: HybridMulti::new()
                .set_seed(new_seed())
                .set_octaves(2),
            chaos_nz: SuperSimplex::new()
                .set_seed(new_seed()),
            hill_nz: HybridMulti::new()
                .set_seed(new_seed())
                .set_octaves(3),
            ridge_nz: HybridMulti::new()
                .set_seed(new_seed())
                .set_octaves(3),
            cliff_height_nz: SuperSimplex::new()
                .set_seed(new_seed()),
        }, 64)
    }

    // 0.0 = wet, 1.0 = dry
    fn get_dry(&self, pos: Vec2<f64>) -> f64 {
        let scale = 2048.0;
        self.dry_nz.get(pos.div(scale).into_array()).mul(1.5).abs().min(1.0)
    }

    // -1.0 = coldest, 0.0 = avg, 1.0 = hottest
    fn get_temp(&self, pos: Vec2<f64>, dry: f64) -> f64 {
        let scale = 2048.0;
        let vari_scale = 32.0;
        // Dryer areas have a less stable temperature
        (
            self.temp_nz.get(pos.div(scale).into_array()) * 0.9 +
            self.temp_vari_nz.get(pos.div(vari_scale).into_array()) * 0.1
        ).mul(0.5 + dry * 0.5)
    }

    // 0.0 = normal/low, 1.0 = high
    fn get_chaos(&self, pos: Vec2<f64>, dry: f64) -> f64 {
        let scale = 1024.0;
        self.chaos_nz.get(pos.div(scale).into_array()).mul(dry).powf(2.0).mul(4.0).max(0.0).min(1.0)
    }

    // 0.0 = normal/flat, max_depth = deepest
    fn get_river(&self, dry: f64) -> f64 {
        let depth = 24.0;
        let max_depth = 8.0;

        if dry < 0.15 {
            dry.mul(20.0).cos().mul(max_depth).max(0.0)
        } else {
            0.0
        }
    }

    // -amp = lowest, amp = highest
    fn get_hill(&self, pos: Vec2<f64>, dry: f64) -> f64 {
        let scale = 1024.0;
        let amp = 32.0;
        self.hill_nz.get(pos.div(scale).into_array()).mul(dry).mul(amp)
    }

    // 0.0 = lowest, height = highest
    fn get_ridge(&self, pos: Vec2<f64>, chaos: f64) -> f64 {
        let scale = 1000.0;
        let height = 200.0;
        (1.0 - self.ridge_nz.get(pos.div(scale).into_array()).abs()).mul(chaos).mul(height)
    }

    // (1.0 - vari) * height = lowest, 1.0 = avg, (1.0 + vari) * height = highest
    fn get_cliff_height(&self, pos: Vec2<f64>) -> f64 {
        let scale = 256.0;
        let vari = 0.3;
        let height = 130.0;

        self.cliff_height_nz.get(pos.div(scale).into_array()).mul(vari).add(1.0).mul(height)
    }
}

impl Gen for OverworldGen {
    type In = Vec2<i64>;
    type Out = Sample;

    fn sample(&self, pos: Vec2<i64>) -> Sample {
        let pos = pos.map(|e| e as f64);

        let dry = self.get_dry(pos);
        let temp = self.get_temp(pos, dry);
        let chaos = self.get_chaos(pos, dry);
        let river = self.get_river(dry);
        let hill = self.get_hill(pos, dry);
        let ridge = self.get_ridge(pos, chaos);
        let cliff_height = self.get_cliff_height(pos);

        Sample {
            dry,
            temp,
            chaos,

            river,
            hill,
            ridge,
            cliff_height,
        }
    }
}
