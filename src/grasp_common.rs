use eframe::{egui, NativeOptions};
use ini::Ini;
use mosaic::internals::default_vals;

use crate::editor_state_machine::EditorState;
use ::mosaic::internals::{
    self_val, EntityId, Mosaic, MosaicCRUD, MosaicIO, Tile, TileFieldGetter, Value,
};
use egui::{ahash::HashMap, Ui, Vec2, WidgetText};
use egui::{Pos2, Rect};
use egui_dock::TabViewer;
use itertools::Itertools;
use quadtree_rs::entry::Entry;
use quadtree_rs::{
    area::{Area, AreaBuilder},
    Quadtree,
};
use std::{ops::Add, sync::Arc};

#[allow(clippy::field_reassign_with_default)]
pub fn create_native_options() -> NativeOptions {
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
        .unwrap_or("true")
        .parse()
        .unwrap_or(true);

    if !options.maximized {
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
        options.initial_window_size = Some(egui::Vec2 { x: w, y: h });
    }

    options
}

#[derive(Default, Debug)]
pub struct GraspEditorData {
    pub pan: Vec2,
    pub previous_pan: Vec2,
    pub selected: Vec<Tile>,
    pub cursor: Pos2,
    pub cursor_delta: Vec2,
    pub rect_delta: Option<Vec2>,
    pub tab_offset: Pos2,
    pub link_start_pos: Option<Pos2>,
    pub link_end: Option<Tile>,
    pub rect_start_pos: Option<Pos2>,
    pub renaming: Option<EntityId>,
    pub text: String,
    pub previous_text: String,
}

#[derive(Debug)]
pub struct GraspEditorTab {
    pub name: String,
    pub state: EditorState,
    pub quadtree: Quadtree<i32, EntityId>,
    pub document_mosaic: Arc<Mosaic>,
    pub node_area: HashMap<EntityId, u64>,

    pub editor_data: GraspEditorData,
}

pub trait QuadTreeFetch {
    fn fetch_tiles(&self, mosaic: &Arc<Mosaic>) -> Vec<Tile>;
    fn fetch_tile(&self, mosaic: &Arc<Mosaic>) -> Tile;
}

impl QuadTreeFetch for Vec<&Entry<i32, EntityId>> {
    fn fetch_tiles(&self, mosaic: &Arc<Mosaic>) -> Vec<Tile> {
        self.iter()
            .flat_map(|next| mosaic.get(*next.value_ref()))
            .collect_vec()
    }

    fn fetch_tile(&self, mosaic: &Arc<Mosaic>) -> Tile {
        mosaic.get(*self.first().unwrap().value_ref()).unwrap()
    }
}

pub trait UiKeyDownExtract {
    fn alt_down(&self) -> bool;
}

impl UiKeyDownExtract for Ui {
    fn alt_down(&self) -> bool {
        let mut alt_down = false;
        self.input(|input_state| {
            alt_down = input_state.modifiers.alt;
        });
        alt_down
    }
}

impl GraspEditorTab {
    pub fn pos_add_editor_offset(&self, v: Pos2) -> Pos2 {
        v.add(self.editor_data.tab_offset.to_vec2())
    }

    pub fn build_circle_area(&self, pos: Pos2, size: i32) -> Area<i32> {
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

    pub fn build_rect_area(&self, rect: Rect) -> Area<i32> {
        let min = self.pos_add_editor_offset(rect.min);
        let max = self.pos_add_editor_offset(rect.max);
        let rect = Rect::from_two_pos(min, max);
        let dim_x = (rect.max.x - rect.min.x) as i32;
        let dim_y = (rect.max.y - rect.min.y) as i32;

        AreaBuilder::default()
            .anchor((rect.min.x as i32, rect.min.y as i32).into())
            .dimensions((
                if dim_x < 1 { 1 } else { dim_x },
                if dim_y < 1 { 1 } else { dim_y },
            ))
            .build()
            .unwrap()
    }

    pub fn pos_with_pan(&self, v: Pos2) -> Pos2 {
        v.add(self.editor_data.pan)
    }
}

impl GraspEditorTab {
    pub fn create_new_object(&mut self, pos: Pos2) {
        let obj = self.document_mosaic.new_object(
            "Position",
            vec![
                ("x".into(), Value::F32(pos.x)),
                ("y".into(), Value::F32(pos.y)),
            ],
        );

        self.document_mosaic
            .new_descriptor(&obj, "Label", self_val(Value::S32("Label!".into())));

        let region = self.build_circle_area(pos, 10);

        if let Some(area_id) = self.quadtree.insert(region, obj.id) {
            self.node_area.insert(obj.id, area_id);
        }
    }

    pub fn create_new_arrow(&mut self, source: &Tile, target: &Tile) {
        let _arr = self
            .document_mosaic
            .new_arrow(source, target, "Arrow", default_vals());
    }
}

#[derive(Default, Debug)]
pub struct GraspEditorTabs {
    pub current_tab: u32,
}

impl GraspEditorTabs {
    pub fn increment(&mut self) -> u32 {
        self.current_tab += 1;
        self.current_tab
    }
}

pub fn get_pos_from_tile(tile: &Tile) -> Option<Pos2> {
    if let (Value::F32(x), Value::F32(y)) = tile.get(("x", "y")) {
        Some(Pos2::new(x, y))
    } else {
        None
    }
}

impl TabViewer for GraspEditorTabs {
    type Tab = GraspEditorTab;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        tab.name.as_str().into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        let xy = ui.clip_rect().left_top();
        tab.editor_data.tab_offset = xy;

        tab.sense(ui);
        tab.render(ui);
        tab.update(ui);
    }
}
