mod pane;
mod editor;
mod tile_manager;

use editor::GraspEditor;
use eframe::{egui, NativeOptions};
use ini::Ini;

fn create_native_options() -> NativeOptions {
    if Ini::load_from_file("config.ini").is_err() {
        let mut conf = Ini::new();
        
        conf.with_section(Some("Window"))
            .set("maximized", "true")
            .set("width", "1920")
            .set("height", "1080");
        conf.write_to_file("config.ini").unwrap();
    }
    
    let config = Ini::load_from_file("config.ini").unwrap();

    let mut options = eframe::NativeOptions::default();

    options.maximized = config
        .get_from(Some("Window"), "maximized")
        .unwrap_or("true").parse().unwrap_or(true);

    if !options.maximized {
        let w = config
            .get_from(Some("Window"), "width")
            .unwrap_or("1920").parse().unwrap_or(1920.0f32);
        let h = config
            .get_from(Some("Window"), "height")
            .unwrap_or("1080").parse().unwrap_or(1080.0f32);
        options.initial_window_size = Some(egui::Vec2 { x: w, y: h });
    }
    
    options
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    let app_name = "GRASP";
    let native_options = create_native_options();

    eframe::run_native(
        app_name,
        native_options,
        Box::new(|cc| 
            Box::new(GraspEditor::new(cc))
        )
    )
}

