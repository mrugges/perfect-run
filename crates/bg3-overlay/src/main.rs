mod app;
mod storylines_ui;

use egui_overlay::egui;
use app::OverlayApp;
use std::path::PathBuf;

fn main() {
    let save_path = std::env::args().nth(1).map(PathBuf::from);

    let mut app = OverlayApp::new(save_path);
    app.setup_watcher();

    egui_overlay::start(PerfectRunOverlay { app });
}

struct PerfectRunOverlay {
    app: OverlayApp,
}

impl egui_overlay::EguiOverlay for PerfectRunOverlay {
    fn gui_run(
        &mut self,
        egui_context: &egui_overlay::egui::Context,
        _default_gfx_backend: &mut egui_overlay::egui_render_three_d::ThreeDBackend,
        _glfw_backend: &mut egui_overlay::egui_window_glfw_passthrough::GlfwBackend,
    ) {
        // Make background transparent
        let frame = egui::containers::Frame::none();
        egui::CentralPanel::default().frame(frame).show(egui_context, |_ui| {
            self.app.update(egui_context);
        });
    }
}
