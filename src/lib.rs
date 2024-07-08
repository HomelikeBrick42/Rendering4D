#![deny(rust_2018_idioms, rust_2024_compatibility)]

use eframe::{egui, egui_wgpu, wgpu};
use encase::{ArrayLength, ShaderSize, ShaderType, StorageBuffer, UniformBuffer};

#[derive(ShaderType)]
struct GpuCamera {
    position: cgmath::Vector4<f32>,
    tan_half_fov: f32,
    up_sky_color: cgmath::Vector3<f32>,
    down_sky_color: cgmath::Vector3<f32>,
}

#[derive(ShaderType)]
struct GpuHyperSphere {
    position: cgmath::Vector4<f32>,
    color: cgmath::Vector3<f32>,
    radius: f32,
}

#[derive(ShaderType)]
struct GpuHyperSpheres<'a> {
    count: ArrayLength,
    #[size(runtime)]
    data: &'a [GpuHyperSphere],
}

struct Camera {
    position: cgmath::Vector4<f32>,
    fov: f32,
    up_sky_color: cgmath::Vector3<f32>,
    down_sky_color: cgmath::Vector3<f32>,
}

struct HyperSphere {
    name: String,
    id: usize,
    position: cgmath::Vector4<f32>,
    color: cgmath::Vector3<f32>,
    radius: f32,
}

pub struct App {
    camera: Camera,
    main_texture: wgpu::Texture,
    main_texture_id: egui::TextureId,
    main_texture_bind_group_layout: wgpu::BindGroupLayout,
    main_texture_bind_group: wgpu::BindGroup,
    camera_uniform_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    hyper_spheres: Vec<HyperSphere>,
    hyper_sphere_next_id: usize,
    hyper_spheres_storage_buffer: wgpu::Buffer,
    hyper_spheres_bind_group_layout: wgpu::BindGroupLayout,
    hyper_spheres_bind_group: wgpu::BindGroup,
    pipeline: wgpu::ComputePipeline,
}

fn vec4_ui(ui: &mut egui::Ui, value: &mut cgmath::Vector4<f32>) {
    ui.add(egui::DragValue::new(&mut value.x).speed(0.1).prefix("x:"))
        .changed();
    ui.add(egui::DragValue::new(&mut value.y).speed(0.1).prefix("y:"))
        .changed();
    ui.add(egui::DragValue::new(&mut value.z).speed(0.1).prefix("z:"))
        .changed();
    ui.add(egui::DragValue::new(&mut value.w).speed(0.1).prefix("w:"))
        .changed();
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> anyhow::Result<Self> {
        let egui_wgpu::RenderState {
            device, renderer, ..
        } = cc.wgpu_render_state.as_ref().unwrap();

        let main_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Main Texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let main_texture_view = main_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let main_texture_id = renderer.write().register_native_texture(
            device,
            &main_texture_view,
            wgpu::FilterMode::Nearest,
        );

        let main_texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: main_texture.format(),
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                }],
            });
        let main_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Bind Group"),
            layout: &main_texture_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&main_texture_view),
            }],
        });

        let camera_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Uniform Buffer"),
            size: GpuCamera::SHADER_SIZE.get(),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(GpuCamera::SHADER_SIZE),
                    },
                    count: None,
                }],
            });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_uniform_buffer.as_entire_binding(),
            }],
        });

        let hyper_spheres_storage_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Hyper Spheres Storage Buffer"),
            size: GpuHyperSpheres::min_size().get(),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let hyper_spheres_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Hyper Spheres Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: Some(GpuHyperSpheres::min_size()),
                    },
                    count: None,
                }],
            });
        let hyper_spheres_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Hyper Spheres Bind Group"),
            layout: &hyper_spheres_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: hyper_spheres_storage_buffer.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("./shader.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[
                &main_texture_bind_group_layout,
                &camera_bind_group_layout,
                &hyper_spheres_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        });

        Ok(Self {
            camera: Camera {
                position: cgmath::vec4(0.0, 0.0, 0.0, 0.0),
                fov: 90.0,
                up_sky_color: cgmath::vec3(0.2, 0.7, 0.9),
                down_sky_color: cgmath::vec3(0.2, 0.2, 0.2),
            },
            main_texture,
            main_texture_id,
            main_texture_bind_group_layout,
            main_texture_bind_group,
            camera_uniform_buffer,
            camera_bind_group,
            hyper_spheres: vec![HyperSphere {
                name: "Default Hyper Sphere".into(),
                id: 0,
                position: cgmath::vec4(2.0, 0.0, 0.0, 0.0),
                color: cgmath::vec3(1.0, 0.0, 0.0),
                radius: 1.0,
            }],
            hyper_sphere_next_id: 1,
            hyper_spheres_storage_buffer,
            hyper_spheres_bind_group_layout,
            hyper_spheres_bind_group,
            pipeline,
        })
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        egui::Window::new("Camera").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Position:");
                vec4_ui(ui, &mut self.camera.position);
            });
            ui.horizontal(|ui| {
                ui.label("Fov:");
                ui.add(
                    egui::DragValue::new(&mut self.camera.fov)
                        .speed(0.1)
                        .range(1.0..=179.0),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Up Sky Color:");
                ui.color_edit_button_rgb(self.camera.up_sky_color.as_mut());
            });
            ui.horizontal(|ui| {
                ui.label("Down Sky Color:");
                ui.color_edit_button_rgb(self.camera.down_sky_color.as_mut());
            });
            ui.allocate_space(ui.available_size());
        });

        egui::Window::new("Hyper Spheres")
            .hscroll(false)
            .vscroll(false)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        self.hyper_spheres.retain_mut(|hyper_sphere| {
                            let mut delete = false;
                            egui::CollapsingHeader::new(&hyper_sphere.name)
                                .id_source(hyper_sphere.id)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label("Name:");
                                        ui.text_edit_singleline(&mut hyper_sphere.name);
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Position:");
                                        vec4_ui(ui, &mut hyper_sphere.position);
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Radius:");
                                        ui.add(
                                            egui::DragValue::new(&mut hyper_sphere.radius)
                                                .speed(0.1),
                                        );
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Color:");
                                        ui.color_edit_button_rgb(hyper_sphere.color.as_mut());
                                    });
                                    if ui.button("Delete").clicked() {
                                        delete = true;
                                    }
                                });
                            !delete
                        });
                        if ui.button("New Hyper Sphere").clicked() {
                            self.hyper_spheres.push(HyperSphere {
                                name: "New Hyper Sphere".into(),
                                id: self.hyper_sphere_next_id,
                                position: cgmath::vec4(2.0, 0.0, 0.0, 0.0),
                                color: cgmath::vec3(1.0, 1.0, 1.0),
                                radius: 1.0,
                            });
                            self.hyper_sphere_next_id += 1;
                        }
                    });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(255, 0, 255)))
            .show(ctx, |ui| {
                let egui_wgpu::RenderState {
                    device,
                    queue,
                    renderer,
                    ..
                } = frame.wgpu_render_state().unwrap();

                let limits = device.limits();
                let available_size = ui.available_size().clamp(
                    egui::Vec2::ZERO,
                    egui::vec2(
                        limits.max_texture_dimension_2d as _,
                        limits.max_texture_dimension_2d as _,
                    ),
                );
                let (rect, _response) = ui.allocate_exact_size(available_size, egui::Sense::drag());
                let width = rect.width() as u32;
                let height = rect.height() as u32;

                let old_image_size = self.main_texture.size();
                if width > 0
                    && height > 0
                    && (old_image_size.width != width || old_image_size.height != height)
                {
                    self.main_texture = device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("Main Texture"),
                        size: wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: old_image_size.depth_or_array_layers,
                        },
                        mip_level_count: self.main_texture.mip_level_count(),
                        sample_count: self.main_texture.sample_count(),
                        dimension: self.main_texture.dimension(),
                        format: self.main_texture.format(),
                        usage: self.main_texture.usage(),
                        view_formats: &[],
                    });
                    let main_texture_view = self
                        .main_texture
                        .create_view(&wgpu::TextureViewDescriptor::default());

                    renderer.write().update_egui_texture_from_wgpu_texture(
                        device,
                        &main_texture_view,
                        wgpu::FilterMode::Nearest,
                        self.main_texture_id,
                    );

                    self.main_texture_bind_group =
                        device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("Main Texture Bind Group"),
                            layout: &self.main_texture_bind_group_layout,
                            entries: &[wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&main_texture_view),
                            }],
                        });
                }

                {
                    let mut buffer = UniformBuffer::new([0; GpuCamera::SHADER_SIZE.get() as _]);
                    let Camera {
                        position,
                        fov,
                        up_sky_color,
                        down_sky_color,
                    } = self.camera;
                    buffer
                        .write(&GpuCamera {
                            position,
                            tan_half_fov: f32::tan(fov.to_radians() / 2.0),
                            up_sky_color,
                            down_sky_color,
                        })
                        .unwrap();
                    queue.write_buffer(&self.camera_uniform_buffer, 0, &buffer.into_inner());
                }

                {
                    let gpu_hyper_spheres = GpuHyperSpheres {
                        count: ArrayLength,
                        data: &self
                            .hyper_spheres
                            .iter()
                            .map(
                                |&HyperSphere {
                                     name: _,
                                     id: _,
                                     position,
                                     color,
                                     radius,
                                 }| GpuHyperSphere {
                                    position,
                                    color,
                                    radius,
                                },
                            )
                            .collect::<Vec<_>>(),
                    };

                    let mut buffer = StorageBuffer::new(Vec::<u8>::with_capacity(
                        gpu_hyper_spheres.size().get() as _,
                    ));
                    buffer.write(&gpu_hyper_spheres).unwrap();
                    let buffer = buffer.into_inner();

                    let new_size = buffer.len().try_into().unwrap();
                    if self.hyper_spheres_storage_buffer.size() < new_size {
                        self.hyper_spheres_storage_buffer =
                            device.create_buffer(&wgpu::BufferDescriptor {
                                label: Some("Hyper Spheres Storage Buffer"),
                                size: new_size,
                                usage: self.hyper_spheres_storage_buffer.usage(),
                                mapped_at_creation: false,
                            });
                        self.hyper_spheres_bind_group =
                            device.create_bind_group(&wgpu::BindGroupDescriptor {
                                label: Some("Hyper Spheres Bind Group"),
                                layout: &self.hyper_spheres_bind_group_layout,
                                entries: &[wgpu::BindGroupEntry {
                                    binding: 0,
                                    resource: self.hyper_spheres_storage_buffer.as_entire_binding(),
                                }],
                            });
                    }

                    queue.write_buffer(&self.hyper_spheres_storage_buffer, 0, &buffer);
                }

                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Encoder"),
                });
                {
                    let workgroup_size = (16, 16);
                    let (dispatch_with, dispatch_height) = (
                        width.div_ceil(workgroup_size.0),
                        height.div_ceil(workgroup_size.1),
                    );
                    let mut compute_pass =
                        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("Compute pass"),
                            timestamp_writes: None,
                        });
                    compute_pass.set_pipeline(&self.pipeline);
                    compute_pass.set_bind_group(0, &self.main_texture_bind_group, &[]);
                    compute_pass.set_bind_group(1, &self.camera_bind_group, &[]);
                    compute_pass.set_bind_group(2, &self.hyper_spheres_bind_group, &[]);
                    compute_pass.dispatch_workgroups(dispatch_with as _, dispatch_height as _, 1);
                }
                queue.submit(std::iter::once(encoder.finish()));

                ui.painter().image(
                    self.main_texture_id,
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 1.0), egui::pos2(1.0, 0.0)),
                    egui::Color32::WHITE,
                );
            });

        ctx.request_repaint();
    }
}
