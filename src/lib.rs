#![deny(rust_2018_idioms, rust_2024_compatibility)]

use eframe::{egui, egui_wgpu, wgpu};

pub struct App {
    main_texture: wgpu::Texture,
    main_texture_id: egui::TextureId,
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
            usage: wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let main_texture_id = renderer.write().register_native_texture(
            device,
            &main_texture.create_view(&wgpu::TextureViewDescriptor::default()),
            wgpu::FilterMode::Nearest,
        );

        Ok(Self {
            main_texture,
            main_texture_id,
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
                    renderer.write().update_egui_texture_from_wgpu_texture(
                        device,
                        &self
                            .main_texture
                            .create_view(&wgpu::TextureViewDescriptor::default()),
                        wgpu::FilterMode::Nearest,
                        self.main_texture_id,
                    );
                }

                queue.write_texture(
                    self.main_texture.as_image_copy(),
                    &std::iter::repeat([255, 0, 0, 255])
                        .take(width as usize * height as usize)
                        .flatten()
                        .collect::<Vec<_>>(),
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(width * 4),
                        rows_per_image: None,
                    },
                    self.main_texture.size(),
                );

                ui.painter().image(
                    self.main_texture_id,
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 1.0), egui::pos2(1.0, 0.0)),
                    egui::Color32::WHITE,
                );
            });
    }
}
