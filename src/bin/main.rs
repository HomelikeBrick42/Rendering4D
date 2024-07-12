use eframe::wgpu;
use rendering4d::App;
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    eframe::run_native(
        "4D Rendering",
        eframe::NativeOptions {
            vsync: false,
            renderer: eframe::Renderer::Wgpu,
            wgpu_options: eframe::egui_wgpu::WgpuConfiguration {
                present_mode: wgpu::PresentMode::AutoNoVsync,
                power_preference: wgpu::PowerPreference::HighPerformance,
                device_descriptor: Arc::new(|_| wgpu::DeviceDescriptor {
                    label: Some("Device"),
                    required_features: wgpu::Features::default()
                        | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                    required_limits: wgpu::Limits::default(),
                }),
                ..Default::default()
            },
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(App::new(cc)?))),
    )?;
    Ok(())
}
