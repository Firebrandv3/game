// Library
use vek::*;

// Local
use terrain::{
    chunk::Block,
    Volume, ReadVolume, ConstructVolume, AnyVolume, SerializeVolume, Voxel, VoxelRelVec,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HomogeneousData {
    size: VoxelRelVec,
    voxel: Block,
}

impl HomogeneousData {
    pub fn new() -> Self {
        HomogeneousData {
            size: Vec3::from((0, 0, 0)),
            voxel: Block::empty(),
        }
    }

    pub fn mut_size(&mut self) -> &mut VoxelRelVec {
        &mut self.size
    }

    pub fn mut_voxel(&mut self) -> &mut Block {
        &mut self.voxel
    }
}

impl Volume for HomogeneousData {
    type VoxelType = Block;

    fn size(&self) -> VoxelRelVec { self.size }
}

impl ReadVolume for HomogeneousData {
    fn at_unsafe(&self, _off: VoxelRelVec) -> Block {
        self.voxel
    }
}

impl ConstructVolume for HomogeneousData {
    fn filled(size: VoxelRelVec, vox: Self::VoxelType) -> HomogeneousData {
        HomogeneousData{
            size,
            voxel: vox,
        }
    }

    fn empty(size: VoxelRelVec) -> HomogeneousData {
        Self::filled(size, Block::empty())
    }
}
