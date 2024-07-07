#![deny(rust_2018_idioms, rust_2024_compatibility)]

use eframe::{egui, egui_wgpu, wgpu};

pub struct App {
    main_texture: wgpu::Texture,
    main_texture_id: egui::TextureId,
    main_texture_bind_group_layout: wgpu::BindGroupLayout,
    main_texture_bind_group: wgpu::BindGroup,
    pipeline: wgpu::ComputePipeline,
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

        let shader = device.create_shader_module(wgpu::include_wgsl!("./shader.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&main_texture_bind_group_layout],
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
            main_texture,
            main_texture_id,
            main_texture_bind_group_layout,
            main_texture_bind_group,
            pipeline,
        })
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
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
                            depth_or_array_layers: 1,
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
                            ..Default::default()
                        });
                    compute_pass.set_pipeline(&self.pipeline);
                    compute_pass.set_bind_group(0, &self.main_texture_bind_group, &[]);
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
