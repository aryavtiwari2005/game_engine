use flecs_ecs::prelude::*;

#[derive(Component, Clone, Copy, Debug)]
#[repr(C)]
pub struct Transform3D {
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
}

#[derive(Component, Clone, Copy, Debug)]
#[repr(C)]
pub struct Velocity3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Component, Clone, Copy, Debug)]
#[repr(C)]
pub struct RenderMeshReference {
    pub mesh_id: u32,
}

#[derive(Component, Clone, Copy, Debug)]
#[repr(C)]
pub struct Camera {
    pub eye: [f32; 3],
    pub target: [f32; 3],
    pub up: [f32; 3],
    pub fov: f32,
    pub znear: f32,
    pub zfar: f32,
}

