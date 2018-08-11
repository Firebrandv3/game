use gfx::{
    self,
    traits::{FactoryExt, Pod},
};
use gfx_device_gl;

use renderer::{ColorFormat, DepthFormat, Renderer};
use voxel::{Mesh, Vertex};

gfx_defines! {
    constant GlobalConsts {
        view_mat: [[f32; 4]; 4] = "view_mat",
        proj_mat: [[f32; 4]; 4] = "proj_mat",
        play_origin: [f32; 4] = "play_origin",
        view_distance: [f32; 4] = "view_distance",
        time: [f32; 4] = "time",
    }
}

type ConstBuffer<T> = gfx::handle::Buffer<gfx_device_gl::Resources, T>;

pub struct ConstHandle<T: Copy + Pod> {
    buffer: ConstBuffer<T>,
}

impl<T: Copy + Pod> ConstHandle<T> {
    pub fn new(renderer: &mut Renderer) -> ConstHandle<T> {
        ConstHandle {
            buffer: renderer.factory_mut().create_constant_buffer(1),
        }
    }

    pub fn update(&self, renderer: &mut Renderer, consts: T) {
        renderer
            .encoder_mut()
            .update_buffer(&self.buffer, &[consts], 0)
            .unwrap();
    }

    pub fn buffer(&self) -> &ConstBuffer<T> { &self.buffer }
}
