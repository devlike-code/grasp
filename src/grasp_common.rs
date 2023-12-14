use ini::Ini;
use mosaic::internals::{EntityId, Tile};

use crate::core::math::vec2::Vec2;

#[allow(clippy::field_reassign_with_default)]
pub fn read_window_size() -> Result<(), (f32, f32)> {
    if Ini::load_from_file("config.ini").is_err() {
        let mut conf = Ini::new();

        conf.with_section(Some("Window"))
            .set("maximized", "true")
            .set("width", "1920")
            .set("height", "1080");
        conf.write_to_file("config.ini").unwrap();
    }

    let config = Ini::load_from_file("config.ini").unwrap();

    let maximized = config
        .get_from(Some("Window"), "maximized")
        .unwrap_or("true")
        .parse()
        .unwrap_or(true);

    if !maximized {
        let w = config
            .get_from(Some("Window"), "width")
            .unwrap_or("1920")
            .parse()
            .unwrap_or(1920.0f32);
        let h = config
            .get_from(Some("Window"), "height")
            .unwrap_or("1080")
            .parse()
            .unwrap_or(1080.0f32);
        Err((w, h))
    } else {
        Ok(())
    }
}

#[derive(Default, Debug)]
pub struct GraspEditorData {
    pub pan: Vec2,
    pub previous_pan: Vec2,
    pub selected: Vec<Tile>,
    pub debug: bool,
    pub cursor: Vec2,
    pub cursor_delta: Vec2,
    pub rect_delta: Option<Vec2>,
    pub tab_offset: Vec2,
    pub link_start_pos: Option<Vec2>,
    pub link_end: Option<Tile>,
    pub rect_start_pos: Option<Vec2>,
    pub tile_changing: Option<EntityId>,
    pub field_changing: Option<String>,
    pub text: String,
    pub previous_text: String,
    pub repositioning: Option<EntityId>,
    pub x_pos: String,
    pub y_pos: String,
    pub previous_x_pos: String,
    pub previous_y_pos: String,
}

// pub trait UiKeyDownExtract {
//     // Keyboard
//     fn alt_down(&self) -> bool;
//     fn delete_down(&self) -> bool;

//     //Mouse
//     fn mouse_secondary_down(&self) -> bool;
// }

// impl UiKeyDownExtract for Ui {
//     fn alt_down(&self) -> bool {
//         self.input(|input_state| input_state.modifiers.alt)
//     }
//     fn delete_down(&self) -> bool {
//         self.input(|input_state| input_state.keys_down.get(&egui::Key::Delete).is_some())
//     }

//     fn mouse_secondary_down(&self) -> bool {
//         self.input(|input| input.pointer.secondary_down())
//     }
// }
