use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};
use winit::event::WindowEvent;

use renderer::{RenderContext, Renderer, RawInstance};
use ecs_world::{Transform3D, Velocity3D, RenderMeshReference, Camera};
use editor::EditorUi;
use flecs_ecs::prelude::*;
use core::time::EngineClock;

struct SimpleRng {
    state: u32,
}

impl SimpleRng {
    fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u32() & 0xFFFFFF) as f32 / 16777216.0
    }

    fn range(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }
}

fn create_model_matrix(position: [f32; 3], rotation_deg: [f32; 3], scale: [f32; 3]) -> [[f32; 4]; 4] {
    let rx = rotation_deg[0].to_radians();
    let ry = rotation_deg[1].to_radians();
    let rz = rotation_deg[2].to_radians();

    let sx = scale[0];
    let sy = scale[1];
    let sz = scale[2];

    let cx = rx.cos();
    let sx_rot = rx.sin();
    let cy = ry.cos();
    let sy_rot = ry.sin();
    let cz = rz.cos();
    let sz_rot = rz.sin();

    let mat_t = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [position[0], position[1], position[2], 1.0],
    ];

    let mat_s = [
        [sx, 0.0, 0.0, 0.0],
        [0.0, sy, 0.0, 0.0],
        [0.0, 0.0, sz, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    let mat_rx = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, cx, sx_rot, 0.0],
        [0.0, -sx_rot, cx, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    let mat_ry = [
        [cy, 0.0, -sy_rot, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [sy_rot, 0.0, cy, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    let mat_rz = [
        [cz, sz_rot, 0.0, 0.0],
        [-sz_rot, cz, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    let mul = |a: [[f32; 4]; 4], b: [[f32; 4]; 4]| -> [[f32; 4]; 4] {
        let mut r = [[0.0; 4]; 4];
        for col in 0..4 {
            for row in 0..4 {
                r[col][row] = a[0][row] * b[col][0]
                            + a[1][row] * b[col][1]
                            + a[2][row] * b[col][2]
                            + a[3][row] * b[col][3];
            }
        }
        r
    };

    let r_z_s = mul(mat_rz, mat_s);
    let r_x_r_z_s = mul(mat_rx, r_z_s);
    let r_y_r_x_r_z_s = mul(mat_ry, r_x_r_z_s);
    mul(mat_t, r_y_r_x_r_z_s)
}

#[derive(Debug, Clone, Copy)]
struct Plane {
    normal: [f32; 3],
    d: f32,
}

impl Plane {
    fn new(a: f32, b: f32, c: f32, d: f32) -> Self {
        let length = (a * a + b * b + c * c).sqrt();
        if length > 0.0 {
            Self {
                normal: [a / length, b / length, c / length],
                d: d / length,
            }
        } else {
            Self {
                normal: [0.0, 0.0, 0.0],
                d: 0.0,
            }
        }
    }

    fn distance(&self, point: [f32; 3]) -> f32 {
        self.normal[0] * point[0] + self.normal[1] * point[1] + self.normal[2] * point[2] + self.d
    }
}

struct Frustum {
    planes: [Plane; 6],
}

impl Frustum {
    fn from_matrix(m: [[f32; 4]; 4]) -> Self {
        // Left: row 3 + row 0
        let left = Plane::new(
            m[0][3] + m[0][0],
            m[1][3] + m[1][0],
            m[2][3] + m[2][0],
            m[3][3] + m[3][0],
        );
        // Right: row 3 - row 0
        let right = Plane::new(
            m[0][3] - m[0][0],
            m[1][3] - m[1][0],
            m[2][3] - m[2][0],
            m[3][3] - m[3][0],
        );
        // Bottom: row 3 + row 1
        let bottom = Plane::new(
            m[0][3] + m[0][1],
            m[1][3] + m[1][1],
            m[2][3] + m[2][1],
            m[3][3] + m[3][1],
        );
        // Top: row 3 - row 1
        let top = Plane::new(
            m[0][3] - m[0][1],
            m[1][3] - m[1][1],
            m[2][3] - m[2][1],
            m[3][3] - m[3][1],
        );
        // Near: row 2
        let near = Plane::new(
            m[0][2],
            m[1][2],
            m[2][2],
            m[3][2],
        );
        // Far: row 3 - row 2
        let far = Plane::new(
            m[0][3] - m[0][2],
            m[1][3] - m[1][2],
            m[2][3] - m[2][2],
            m[3][3] - m[3][2],
        );

        Self {
            planes: [left, right, bottom, top, near, far],
        }
    }

    fn intersects_sphere(&self, center: [f32; 3], radius: f32) -> bool {
        for plane in &self.planes {
            if plane.distance(center) < -radius {
                return false;
            }
        }
        true
    }
}

struct AppRunner {
    window: Option<Arc<Window>>,
    render_context: Option<RenderContext>,
    renderer: Option<Renderer>,
    editor_ui: Option<EditorUi>,
    ecs_world: World,
    clock: EngineClock,
    focused: bool,
    fps: f64,
    frame_count: u32,
    fps_timer: Instant,
    last_synced_eye: [f32; 3],
}

impl AppRunner {
    fn new() -> Self {
        let ecs_world = World::new();

        // Spawn 10,000 entities
        let mut rng = SimpleRng::new(1337);
        for _i in 0..10000 {
            let entity = ecs_world.entity();
            entity.set(Transform3D {
                position: [
                    rng.range(-15.0, 15.0),
                    rng.range(-15.0, 15.0),
                    rng.range(-15.0, 15.0),
                ],
                rotation: [
                    rng.range(0.0, 360.0),
                    rng.range(0.0, 360.0),
                    rng.range(0.0, 360.0),
                ],
                scale: [0.3, 0.3, 0.3],
            });
            entity.set(Velocity3D {
                x: rng.range(-2.0, 2.0),
                y: rng.range(-2.0, 2.0),
                z: rng.range(-2.0, 2.0),
            });
            entity.set(RenderMeshReference {
                mesh_id: 0,
            });
        }

        // Spawn "Main Camera" entity
        let camera_entity = ecs_world.entity_named("Main Camera");
        camera_entity.set(Transform3D {
            position: [0.0, 15.0, 30.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        });
        camera_entity.set(Camera {
            eye: [0.0, 15.0, 30.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            fov: 45.0,
            znear: 0.1,
            zfar: 100.0,
        });

        Self {
            window: None,
            render_context: None,
            renderer: None,
            editor_ui: None,
            ecs_world,
            clock: EngineClock::new(60),
            focused: true,
            fps: 0.0,
            frame_count: 0,
            fps_timer: Instant::now(),
            last_synced_eye: [0.0, 15.0, 30.0],
        }
    }

    fn render_frame(&mut self) {
        let render_context = match &mut self.render_context {
            Some(rc) => rc,
            None => return,
        };
        let renderer = match &mut self.renderer {
            Some(r) => r,
            None => return,
        };
        let editor_ui = match &mut self.editor_ui {
            Some(ui) => ui,
            None => return,
        };
        let window = match &self.window {
            Some(w) => w,
            None => return,
        };

        // Tick clock
        let _dt = self.clock.tick();

        // Calculate FPS
        self.frame_count += 1;
        let elapsed = self.fps_timer.elapsed();
        if elapsed >= std::time::Duration::from_secs(1) {
            self.fps = self.frame_count as f64 / elapsed.as_secs_f64();
            self.frame_count = 0;
            self.fps_timer = Instant::now();
        }

        // Run fixed physics update systems
        let fixed_dt = self.clock.fixed_timestep_seconds() as f32;
        let simulate = editor_ui.simulate_physics;
        while self.clock.should_fixed_update() {
            if simulate {
                // Movement Simulation Query
                self.ecs_world.query::<(&mut Transform3D, &mut Velocity3D)>().build().each(|(t, v)| {
                    t.position[0] += v.x * fixed_dt;
                    t.position[1] += v.y * fixed_dt;
                    t.position[2] += v.z * fixed_dt;

                    // Bounce on boundaries
                    if t.position[0] < -15.0 {
                        t.position[0] = -15.0;
                        v.x = -v.x;
                    } else if t.position[0] > 15.0 {
                        t.position[0] = 15.0;
                        v.x = -v.x;
                    }

                    if t.position[1] < -15.0 {
                        t.position[1] = -15.0;
                        v.y = -v.y;
                    } else if t.position[1] > 15.0 {
                        t.position[1] = 15.0;
                        v.y = -v.y;
                    }

                    if t.position[2] < -15.0 {
                        t.position[2] = -15.0;
                        v.z = -v.z;
                    } else if t.position[2] > 15.0 {
                        t.position[2] = 15.0;
                        v.z = -v.z;
                    }

                    // Slow rotation
                    t.rotation[0] = (t.rotation[0] + 15.0 * fixed_dt) % 360.0;
                    t.rotation[1] = (t.rotation[1] + 10.0 * fixed_dt) % 360.0;
                    t.rotation[2] = (t.rotation[2] + 5.0 * fixed_dt) % 360.0;
                });
            }
        }

        // WGPU Surface Texture Acquisition first
        // If it fails, configure surface and return early.
        // This prevents consuming egui input and discarding textures_delta.
        let output = match render_context.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                render_context.surface.configure(&render_context.device, &render_context.config);
                texture
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                render_context.surface.configure(&render_context.device, &render_context.config);
                return;
            }
            _ => return,
        };

        let mut camera_comp = None;
        let camera_entity = self.ecs_world.entity_named("Main Camera");
        if camera_entity.is_alive() {
            let mut t_opt = None;
            let mut c_opt = None;
            camera_entity.get::<&Transform3D>(|t| t_opt = Some(*t));
            camera_entity.get::<&Camera>(|c| c_opt = Some(*c));
            if let (Some(mut t), Some(mut c)) = (t_opt, c_opt) {
                if t.position != self.last_synced_eye {
                    c.eye = t.position;
                    camera_entity.set(c);
                    self.last_synced_eye = t.position;
                } else if c.eye != self.last_synced_eye {
                    t.position = c.eye;
                    camera_entity.set(t);
                    self.last_synced_eye = c.eye;
                }
                camera_comp = Some(c);
            }
        }

        let c = camera_comp.unwrap_or(Camera {
            eye: [0.0, 15.0, 30.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            fov: 45.0,
            znear: 0.1,
            zfar: 100.0,
        });

        let render_camera = renderer::RenderCamera {
            eye: c.eye,
            target: c.target,
            up: c.up,
            fov: c.fov,
            znear: c.znear,
            zfar: c.zfar,
        };

        let aspect_ratio = render_context.size.width as f32 / render_context.size.height as f32;
        renderer.update_camera(render_context, &render_camera, aspect_ratio);

        // Compute frustum planes for culling
        let view_proj = renderer::create_dynamic_camera_matrix(&render_camera, aspect_ratio);
        let frustum = Frustum::from_matrix(view_proj);

        // Render Editor UI frame and get paint jobs
        let entity_count = self.ecs_world.query::<&Transform3D>().build().count() as usize;
        let (paint_jobs, textures_delta) = editor_ui.update(window, &self.ecs_world, self.fps, entity_count);

        // Query ECS to collect instancing data (with 6-plane frustum culling)
        let mut raw_instances = Vec::with_capacity(10000);
        self.ecs_world.query::<(&Transform3D, &RenderMeshReference)>().build().each(|(t, _m)| {
            let center = t.position;
            let radius = t.scale[0] * 1.732 / 2.0;
            if frustum.intersects_sphere(center, radius) {
                let model_matrix = create_model_matrix(t.position, t.rotation, t.scale);
                raw_instances.push(RawInstance { model_matrix });
            }
        });

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = render_context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Frame Encoder"),
        });

        // 1. Update egui textures and buffers
        for (id, image_delta) in &textures_delta.set {
            editor_ui.renderer.update_texture(&render_context.device, &render_context.queue, *id, image_delta);
        }
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [render_context.size.width, render_context.size.height],
            pixels_per_point: window.scale_factor() as f32,
        };
        editor_ui.renderer.update_buffers(
            &render_context.device,
            &render_context.queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        // 2. Upload instance data (with dynamic buffer scaling)
        let instance_count = raw_instances.len();
        if instance_count > renderer.max_instances {
            let new_max = instance_count.next_power_of_two().max(10000);
            renderer.instance_buffer = render_context.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Dynamic Instance Buffer"),
                size: (new_max * std::mem::size_of::<RawInstance>()) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            renderer.max_instances = new_max;
        }
        if instance_count > 0 {
            render_context.queue.write_buffer(
                &renderer.instance_buffer,
                0,
                bytemuck::cast_slice(&raw_instances[..instance_count]),
            );
        }

        // 3. Render 3D Scene
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("3D Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.05,
                            b: 0.08,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &render_context.depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            if instance_count > 0 {
                render_pass.set_pipeline(&renderer.render_pipeline);
                render_pass.set_bind_group(0, &renderer.camera_bind_group, &[]);
                render_pass.set_vertex_buffer(0, renderer.vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, renderer.instance_buffer.slice(..));
                render_pass.set_index_buffer(renderer.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..renderer.num_indices, 0, 0..instance_count as u32);
            }
        }

        // 4. Render Egui Overlay
        {
            let rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Egui Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            let mut rpass = rpass.forget_lifetime();
            editor_ui.renderer.render(&mut rpass, &paint_jobs, &screen_descriptor);
        }

        // 5. Submit and present
        render_context.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // 6. Free textures
        for id in &textures_delta.free {
            editor_ui.renderer.free_texture(id);
        }
    }
}

impl ApplicationHandler for AppRunner {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attributes = Window::default_attributes()
            .with_title("3D ECS Engine & Editor")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0));
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.window = Some(window.clone());

        // Initialize WGPU renderer
        let render_context = pollster::block_on(RenderContext::new(window.clone()));
        let renderer = Renderer::new(&render_context);

        // Initialize Editor UI
        let editor_ui = EditorUi::new(
            &render_context.device,
            render_context.config.format,
            &window,
        );

        self.render_context = Some(render_context);
        self.renderer = Some(renderer);
        self.editor_ui = Some(editor_ui);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let window = match &self.window {
            Some(w) => w,
            None => return,
        };

        // Pass event to editor UI
        if let Some(editor_ui) = &mut self.editor_ui {
            let _response = editor_ui.handle_event(window, &event);
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                if let Some(render_context) = &mut self.render_context {
                    render_context.resize(physical_size);
                }
                if let (Some(renderer), Some(render_context)) = (&mut self.renderer, &self.render_context) {
                    let mut camera_comp = None;
                    let camera_entity = self.ecs_world.entity_named("Main Camera");
                    if camera_entity.is_alive() {
                        camera_entity.get::<&Camera>(|c| {
                            camera_comp = Some(*c);
                        });
                    }
                    let c = camera_comp.unwrap_or(Camera {
                        eye: [0.0, 15.0, 30.0],
                        target: [0.0, 0.0, 0.0],
                        up: [0.0, 1.0, 0.0],
                        fov: 45.0,
                        znear: 0.1,
                        zfar: 100.0,
                    });
                    let render_camera = renderer::RenderCamera {
                        eye: c.eye,
                        target: c.target,
                        up: c.up,
                        fov: c.fov,
                        znear: c.znear,
                        zfar: c.zfar,
                    };
                    let aspect_ratio = render_context.size.width as f32 / render_context.size.height as f32;
                    renderer.update_camera(render_context, &render_camera, aspect_ratio);
                }
            }
            WindowEvent::Focused(focused) => {
                self.focused = focused;
                if focused {
                    event_loop.set_control_flow(ControlFlow::Poll);
                } else {
                    event_loop.set_control_flow(ControlFlow::Wait);
                }
            }
            WindowEvent::RedrawRequested => {
                self.render_frame();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let (true, Some(window)) = (self.focused, &self.window) {
            window.request_redraw();
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = AppRunner::new();
    event_loop.run_app(&mut app).unwrap();
}
