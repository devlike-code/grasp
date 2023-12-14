use crate::editor_state_machine::EditorState;
use crate::math::rect::Rect2;
use crate::math::vec2::Vec2;
use crate::windowing::set_window_focus;
use crate::GuiState;
use ::mosaic::internals::{EntityId, Mosaic, MosaicCRUD, MosaicIO, Tile, TileFieldQuery, Value};
use imgui::{ImString, WindowFlags};
use ini::Ini;
use mosaic::capabilities::{ArchetypeSubject, QueueCapability};
use mosaic::internals::collage::Collage;
use mosaic::internals::{par, void, MosaicTypelevelCRUD, TileFieldSetter};
use mosaic::iterators::component_selectors::ComponentSelectors;
use mosaic::iterators::tile_deletion::TileDeletion;
use quadtree_rs::{
    area::{Area, AreaBuilder},
    Quadtree,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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

#[derive(Debug)]
pub struct GraspEditorWindow {
    pub name: String,
    pub tab_tile: Tile,
    pub state: EditorState,
    pub quadtree: Quadtree<i32, EntityId>,
    pub document_mosaic: Arc<Mosaic>,
    pub object_to_area: HashMap<EntityId, Vec<u64>>,
    pub collage: Box<Collage>,
    pub ruler_visible: bool,
    pub grid_visible: bool,
    pub editor_data: GraspEditorData,
}

impl PartialEq for GraspEditorWindow {
    fn eq(&self, other: &Self) -> bool {
        self.tab_tile.id == other.tab_tile.id
    }
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

impl GraspEditorWindow {
    pub fn show(&self, s: &GuiState) {
        s.ui.window(self.name.as_str())
            .size([700.0, 500.0], imgui::Condition::Appearing)
            .position(
                [
                    200.0 + 50.0 * (self.tab_tile.id % 5) as f32,
                    200.0 - 20.0 * (self.tab_tile.id % 5) as f32,
                ],
                imgui::Condition::Appearing,
            )
            .build(|| {
                if s.ui.is_window_focused() {
                    for mut focus in self
                        .document_mosaic
                        .get_all()
                        .include_component("EditorStateFocusedWindow")
                    {
                        focus.set("self", self.tab_tile.id as u64);
                    }
                }
                if let Some(request) = self.document_mosaic.dequeue(&self.tab_tile) {
                    set_window_focus(&self.name);
                    request.iter().delete();
                }
                s.ui.label_text("This is a graph window", "");
            });
    }
}

impl GraspEditorWindow {
    pub fn pos_add_editor_offset(&self, v: Vec2) -> Vec2 {
        v + self.editor_data.tab_offset
    }

    pub fn build_circle_area(&self, pos: Vec2, size: i32) -> Area<i32> {
        let pos = self.pos_add_editor_offset(pos);
        AreaBuilder::default()
            .anchor((pos.x as i32 - size, pos.y as i32 - size).into())
            .dimensions((size * 2, size * 2))
            .build()
            .unwrap()
    }

    pub fn build_cursor_area(&self) -> Area<i32> {
        self.build_circle_area(self.editor_data.cursor, 1)
    }

    pub fn build_rect_area(&self, rect: Rect2) -> Area<i32> {
        let min = rect.min();
        let max = rect.max();
        let min = self.pos_add_editor_offset(min);
        let max = self.pos_add_editor_offset(max);
        let rect = Rect2::from_two_pos(min, max);
        let dim_x = (max.x - min.x) as i32;
        let dim_y = (max.y - min.y) as i32;

        AreaBuilder::default()
            .anchor((min.x as i32, min.y as i32).into())
            .dimensions((
                if dim_x < 1 { 1 } else { dim_x },
                if dim_y < 1 { 1 } else { dim_y },
            ))
            .build()
            .unwrap()
    }

    pub fn pos_with_pan(&self, v: Vec2) -> Vec2 {
        v + self.editor_data.pan
    }
}

impl GraspEditorWindow {
    pub fn create_new_object(&mut self, pos: Vec2) {
        self.document_mosaic.new_type("Node: unit;").unwrap();

        let obj = self.document_mosaic.new_object("Node", void());

        obj.add_component(
            "Position",
            vec![
                ("x".into(), Value::F32(pos.x)),
                ("y".into(), Value::F32(pos.y)),
            ],
        );
        obj.add_component("Label", par("<Label>"));

        let region = self.build_circle_area(pos, 10);

        if let Some(area_id) = self.quadtree.insert(region, obj.id) {
            self.object_to_area.insert(obj.id, vec![area_id]);
        }
    }

    pub fn create_new_arrow(
        &mut self,
        source: &Tile,
        target: &Tile,
        middle_pos: Vec2,
        bezier_rects: Vec<Rect2>,
    ) {
        println!("Bezier rects: {:?}", bezier_rects);

        let arr = self
            .document_mosaic
            .new_arrow(source, target, "Arrow", void());

        arr.add_component(
            "Position",
            vec![
                ("x".into(), Value::F32(middle_pos.x)),
                ("y".into(), Value::F32(middle_pos.y)),
            ],
        );
        arr.add_component("Label", par("<Label>"));

        let region = self.build_circle_area(middle_pos, 10);

        if let Some(area_id) = self.quadtree.insert(region, arr.id) {
            if let Some(areas_vec) = self.object_to_area.get_mut(&arr.id) {
                areas_vec.push(area_id);
            } else {
                self.object_to_area.insert(arr.id, vec![area_id]);
            }
        }

        for r in bezier_rects {
            let region = self.build_rect_area(r);
            if let Some(area_id) = self.quadtree.insert(region, arr.id) {
                if let Some(areas_vec) = self.object_to_area.get_mut(&arr.id) {
                    areas_vec.push(area_id);
                    //self.object_to_area.insert(arr.id, areas_vec.to_owned());
                }
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct GraspEditorWindows {
    pub current_index: u32,
    pub windows: Vec<GraspEditorWindow>,
    pub focused: Mutex<EntityId>,
}

impl GraspEditorWindows {
    pub fn increment(&mut self) -> u32 {
        self.current_index += 1;
        self.current_index
    }

    pub fn show(&self, s: &GuiState) {
        for window in &self.windows {
            window.show(s);
        }
    }

    pub fn focus(&self, name: &str) {
        if let Some(pos) = self.windows.iter().position(|w| w.name.as_str() == name) {
            let window = self.windows.get(pos).unwrap();
            let request = window.document_mosaic.new_object("void", void());
            window.document_mosaic.enqueue(&window.tab_tile, &request);
            *self.focused.lock().unwrap() = window.tab_tile.id;
        }
    }
}

pub fn get_pos_from_tile(tile: &Tile) -> Option<Vec2> {
    if let Some(tile_pos_component) = tile.get_component("Position") {
        if let (Value::F32(x), Value::F32(y)) = tile_pos_component.get_by(("x", "y")) {
            Some(Vec2::new(x, y))
        } else {
            None
        }
    } else {
        None
    }
}
