mod app;
mod storylines_ui;

use egui_overlay::egui;
use app::OverlayApp;
use std::path::PathBuf;

fn main() {
    let save_path = std::env::args().nth(1).map(PathBuf::from);

    let mut app = OverlayApp::new(save_path);
    app.setup_watcher();

    egui_overlay::start(PerfectRunOverlay {
        app,
        resized: false,
    });
}

struct PerfectRunOverlay {
    app: OverlayApp,
    resized: bool,
}

impl egui_overlay::EguiOverlay for PerfectRunOverlay {
    fn gui_run(
        &mut self,
        egui_context: &egui_overlay::egui::Context,
        _default_gfx_backend: &mut egui_overlay::egui_render_three_d::ThreeDBackend,
        glfw_backend: &mut egui_overlay::egui_window_glfw_passthrough::GlfwBackend,
    ) {
        // On first frame, resize window to cover the full primary monitor
        if !self.resized {
            self.resized = true;
            glfw_backend.glfw.with_primary_monitor(|_, monitor| {
                if let Some(monitor) = monitor {
                    let mode = monitor.get_video_mode().unwrap();
                    glfw_backend.window.set_size(
                        mode.width as i32,
                        mode.height as i32,
                    );
                    glfw_backend.window.set_pos(0, 0);
                }
            });
        }

        // Make background transparent
        let frame = egui::containers::Frame::none();
        egui::CentralPanel::default().frame(frame).show(egui_context, |_ui| {
            self.app.update(egui_context);
        });

        if self.app.should_exit() {
            glfw_backend.window.set_should_close(true);
        }
    }
}
