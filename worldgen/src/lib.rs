extern crate noise;
#[macro_use]
extern crate euler;

mod gen;

use euler::*;

use gen::Generator;

#[derive(Copy, Clone)]
pub enum Biome {
    Grassland,
    Ocean,
    Sand,
    River,
    Mountain,
}

pub struct MacroChunk {
    alt: u32,
    biome: Biome,
    wind: Vec2,
}

impl MacroChunk {
    pub fn altitude(&self) -> u32 { self.alt }
    pub fn biome(&self) -> Biome { self.biome }

    pub fn wind(&self) -> Vec2 { self.wind }

    pub fn calc_wind(&mut self, gen: Generator, coords: [u32; 3]) {
        self.wind = gen.wind(coords);
    }
}

pub struct MacroWorld {
    seed: u32,
    gen: Generator,

    time: f64,

    size: u32,
    chunks: Vec<MacroChunk>,
}

impl MacroWorld {
    pub fn new(seed: u32, size: u32) -> MacroWorld {
        let mut chunks = Vec::new();

        let gen = Generator::new(seed);

        for x in 0..size {
            for y in 0..size {
                chunks.push(MacroChunk {
                    alt: gen.altitude([x, y]),
                    biome: gen.biome([x, y]),
                    wind: gen.wind([x, y, 0]),
                });
            }
        }

        MacroWorld {
            seed,
            gen,
            time: 0.0,
            size,
            chunks,
        }
    }

    pub fn tick(&mut self, dt: f64) {
        self.time += dt;
    }

    pub fn calc_wind(&mut self) {
        let gen = self.gen;
        let time = self.time;
        for x in 0..self.size {
            for y in 0..self.size {
                match self.get_mut(x, y) {
                    Some(c) => c.calc_wind(gen, [x, y, time as u32]),
                    None => {},
                }
            }
        }
    }

    pub fn size(&self) -> u32 { self.size }

    pub fn get<'a>(&'a self, x: u32, y: u32) -> Option<&'a MacroChunk> {
        self.chunks.get(self.size as usize * x as usize + y as usize)
    }

    pub fn get_mut<'a>(&'a mut self, x: u32, y: u32) -> Option<&'a mut MacroChunk> {
        self.chunks.get_mut(self.size as usize * x as usize + y as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::MacroWorld;

    #[test]
    fn new_world() {
        let _mw = MacroWorld::new(1337, 4);
    }
}
