use winit::window::Window;
use flecs_ecs::prelude::*;
use ecs_world::{Transform3D, Velocity3D, RenderMeshReference};

pub struct EditorUi {
    pub context: egui::Context,
    pub state: egui_winit::State,
    pub renderer: egui_wgpu::Renderer,
    pub selected_entity: Option<Entity>,
    pub cached_entities: Vec<Entity>,
    pub last_update_tick: u32,
    pub search_filter: String,
    pub simulate_physics: bool,
}

impl EditorUi {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        window: &Window,
    ) -> Self {
        let context = egui::Context::default();
        
        let viewport_id = egui::ViewportId::ROOT;
        let scale_factor = window.scale_factor() as f32;
        let max_texture_side = device.limits().max_texture_dimension_2d as usize;
        
        let state = egui_winit::State::new(
            context.clone(),
            viewport_id,
            window,
            Some(scale_factor),
            None,
            Some(max_texture_side),
        );
        
        let renderer = egui_wgpu::Renderer::new(
            device,
            format,
            egui_wgpu::RendererOptions::default(),
        );
        
        Self {
            context,
            state,
            renderer,
            selected_entity: None,
            cached_entities: Vec::new(),
            last_update_tick: 0,
            search_filter: String::new(),
            simulate_physics: true,
        }
    }
    
    pub fn handle_event(&mut self, window: &Window, event: &winit::event::WindowEvent) -> egui_winit::EventResponse {
        self.state.on_window_event(window, event)
    }
    
    pub fn update(
        &mut self,
        window: &Window,
        world: &World,
        fps: f64,
        entity_count: usize,
    ) -> (Vec<egui::ClippedPrimitive>, egui::TexturesDelta) {
        let raw_input = self.state.take_egui_input(window);
        let mut selected_entity = self.selected_entity;
        
        // Refresh cached entity list every 30 frames
        self.last_update_tick += 1;
        if self.last_update_tick >= 30 || self.cached_entities.is_empty() {
            self.last_update_tick = 0;
            self.cached_entities.clear();
            world.query::<&Transform3D>().build().each_entity(|entity, _| {
                self.cached_entities.push(*entity);
            });
            self.cached_entities.sort_by_key(|e| e.0);
        }
        
        let full_output = self.context.run_ui(raw_input, |ui| {
            egui::Panel::left("editor_panel")
                .resizable(true)
                .default_size(320.0)
                .show_inside(ui, |ui| {
                    ui.heading("Engine Editor Overlay");
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        ui.label(format!("FPS: {:.1}", fps));
                        ui.label(format!("Entities: {}", entity_count));
                    });
                    ui.checkbox(&mut self.simulate_physics, "Simulate Physics");
                    ui.separator();
                    
                    ui.heading("Hierarchy (World Tree)");
                    ui.horizontal(|ui| {
                        ui.label("Search:");
                        ui.text_edit_singleline(&mut self.search_filter);
                        if ui.button("Clear").clicked() {
                            self.search_filter.clear();
                        }
                    });
                    ui.separator();
                    
                    let filter = self.search_filter.trim().to_lowercase();
                    let filtered_entities: Vec<Entity> = if filter.is_empty() {
                        self.cached_entities.clone()
                    } else {
                        self.cached_entities
                            .iter()
                            .filter(|e| {
                                let id_str = e.0.to_string();
                                let entity_view = world.entity_from_id(e.0);
                                let name = entity_view.name();
                                id_str.contains(&filter) || name.to_lowercase().contains(&filter)
                            })
                            .cloned()
                            .collect()
                    };
                    
                    let row_height = ui.text_style_height(&egui::TextStyle::Body);
                    egui::ScrollArea::vertical()
                        .max_height(250.0)
                        .show_rows(ui, row_height, filtered_entities.len(), |ui, row_range| {
                            for idx in row_range {
                                let entity = filtered_entities[idx];
                                let entity_view = world.entity_from_id(entity.0);
                                let name = entity_view.name();
                                let label = if !name.is_empty() {
                                    format!("{} ({})", name, entity.0)
                                } else {
                                    format!("Entity {}", entity.0)
                                };
                                
                                let is_selected = selected_entity.as_ref().is_some_and(|se| se.0 == entity.0);
                                if ui.selectable_label(is_selected, label).clicked() {
                                    selected_entity = Some(entity);
                                }
                            }
                        });
                    
                    ui.separator();
                    
                    ui.heading("Inspector");
                    if let Some(entity) = selected_entity {
                        let entity_view = world.entity_from_id(entity.0);
                        if !entity_view.is_alive() {
                            selected_entity = None;
                            ui.label("No entity selected");
                        } else {
                            ui.label(format!("Selected: {:?}", entity.0));
                            let name = entity_view.name();
                            if !name.is_empty() {
                                ui.label(format!("Name: {}", name));
                            }
                            ui.separator();
                            
                            // Read and mutate components
                            let mut transform = None;
                            entity_view.get::<&Transform3D>(|t| {
                                transform = Some(*t);
                            });
                            
                            if let Some(mut t) = transform {
                                ui.collapsing("Transform3D", |ui| {
                                    let mut changed = false;
                                    
                                    ui.label("Position");
                                    ui.horizontal(|ui| {
                                        ui.label("X:");
                                        changed |= ui.add(egui::Slider::new(&mut t.position[0], -20.0..=20.0)).changed();
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Y:");
                                        changed |= ui.add(egui::Slider::new(&mut t.position[1], -20.0..=20.0)).changed();
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Z:");
                                        changed |= ui.add(egui::Slider::new(&mut t.position[2], -20.0..=20.0)).changed();
                                    });
                                    
                                    ui.label("Rotation");
                                    ui.horizontal(|ui| {
                                        ui.label("X:");
                                        changed |= ui.add(egui::Slider::new(&mut t.rotation[0], -180.0..=180.0)).changed();
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Y:");
                                        changed |= ui.add(egui::Slider::new(&mut t.rotation[1], -180.0..=180.0)).changed();
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Z:");
                                        changed |= ui.add(egui::Slider::new(&mut t.rotation[2], -180.0..=180.0)).changed();
                                    });
                                    
                                    ui.label("Scale");
                                    ui.horizontal(|ui| {
                                        ui.label("X:");
                                        changed |= ui.add(egui::Slider::new(&mut t.scale[0], 0.1..=10.0)).changed();
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Y:");
                                        changed |= ui.add(egui::Slider::new(&mut t.scale[1], 0.1..=10.0)).changed();
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Z:");
                                        changed |= ui.add(egui::Slider::new(&mut t.scale[2], 0.1..=10.0)).changed();
                                    });
                                    
                                    if changed {
                                        entity_view.set(t);
                                    }
                                });
                            }
                            
                            let mut velocity = None;
                            entity_view.get::<&Velocity3D>(|v| {
                                velocity = Some(*v);
                            });
                            
                            if let Some(mut v) = velocity {
                                ui.collapsing("Velocity3D", |ui| {
                                    let mut changed = false;
                                    
                                    ui.horizontal(|ui| {
                                        ui.label("X:");
                                        changed |= ui.add(egui::Slider::new(&mut v.x, -10.0..=10.0)).changed();
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Y:");
                                        changed |= ui.add(egui::Slider::new(&mut v.y, -10.0..=10.0)).changed();
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Z:");
                                        changed |= ui.add(egui::Slider::new(&mut v.z, -10.0..=10.0)).changed();
                                    });
                                    
                                    if changed {
                                        entity_view.set(v);
                                    }
                                });
                            }
                            
                            let mut mesh_ref = None;
                            entity_view.get::<&RenderMeshReference>(|m| {
                                mesh_ref = Some(*m);
                            });
                            
                            if let Some(mut m) = mesh_ref {
                                ui.collapsing("RenderMeshReference", |ui| {
                                    let mut changed = false;
                                    
                                    ui.horizontal(|ui| {
                                        ui.label("Mesh ID:");
                                        changed |= ui.add(egui::Slider::new(&mut m.mesh_id, 0..=5)).changed();
                                    });
                                    
                                    if changed {
                                        entity_view.set(m);
                                    }
                                });
                            }
                            
                            // Dynamic Camera support: check if this is a Camera
                            let mut camera_comp = None;
                            entity_view.get::<&ecs_world::Camera>(|c| {
                                camera_comp = Some(*c);
                            });
                            
                            if let Some(mut c) = camera_comp {
                                ui.collapsing("Camera", |ui| {
                                    let mut changed = false;
                                    
                                    ui.label("Eye Position");
                                    ui.horizontal(|ui| {
                                        ui.label("X:");
                                        changed |= ui.add(egui::Slider::new(&mut c.eye[0], -100.0..=100.0)).changed();
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Y:");
                                        changed |= ui.add(egui::Slider::new(&mut c.eye[1], -100.0..=100.0)).changed();
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Z:");
                                        changed |= ui.add(egui::Slider::new(&mut c.eye[2], -100.0..=100.0)).changed();
                                    });
                                    
                                    ui.label("Target Position");
                                    ui.horizontal(|ui| {
                                        ui.label("X:");
                                        changed |= ui.add(egui::Slider::new(&mut c.target[0], -100.0..=100.0)).changed();
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Y:");
                                        changed |= ui.add(egui::Slider::new(&mut c.target[1], -100.0..=100.0)).changed();
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Z:");
                                        changed |= ui.add(egui::Slider::new(&mut c.target[2], -100.0..=100.0)).changed();
                                    });
                                    
                                    ui.label("Settings");
                                    ui.horizontal(|ui| {
                                        ui.label("FOV:");
                                        changed |= ui.add(egui::Slider::new(&mut c.fov, 10.0..=120.0)).changed();
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Near:");
                                        changed |= ui.add(egui::Slider::new(&mut c.znear, 0.01..=10.0)).changed();
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Far:");
                                        changed |= ui.add(egui::Slider::new(&mut c.zfar, 10.0..=1000.0)).changed();
                                    });
                                    
                                    if changed {
                                        entity_view.set(c);
                                    }
                                });
                            }
                        }
                    } else {
                        ui.label("No entity selected");
                    }
                });
        });
        
        self.selected_entity = selected_entity;
        
        self.state.handle_platform_output(window, full_output.platform_output);
        let paint_jobs = self.context.tessellate(full_output.shapes, full_output.pixels_per_point);
        (paint_jobs, full_output.textures_delta)
    }
}
