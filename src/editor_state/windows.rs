use crate::core::gui::calc_text_size;
use crate::core::has_mosaic::HasMosaic;
use crate::core::math::rect2::Rect2;
use crate::core::math::vec2::Vec2;
use crate::editor_state_machine::EditorState;
use crate::grasp_common::GraspEditorData;
use crate::grasp_render::GraspRenderer;
use crate::GuiState;
use ::mosaic::internals::{EntityId, Mosaic, MosaicCRUD, MosaicIO, Tile, Value};
use imgui::ImColor32;
use mosaic::capabilities::ArchetypeSubject;
use mosaic::internals::{par, void, MosaicTypelevelCRUD};
use quadtree_rs::{
    area::{Area, AreaBuilder},
    Quadtree,
};
use std::collections::HashMap;
use std::ops::Add;
use std::sync::{Arc, Mutex};

use super::helpers::{QuadtreeUpdateCapability, RequireWindowFocus};

pub struct GraspEditorWindow {
    pub name: String,
    pub window_tile: Tile,
    pub state: EditorState,
    pub quadtree: Mutex<Quadtree<i32, EntityId>>,
    pub document_mosaic: Arc<Mosaic>,
    pub component_mosaic: Arc<Mosaic>,
    pub transformer_mosaic: Arc<Mosaic>,
    pub editor_mosaic: Arc<Mosaic>,
    pub object_to_area: Mutex<HashMap<EntityId, u64>>,
    pub ruler_visible: bool,
    pub grid_visible: bool,
    pub editor_data: GraspEditorData,
    pub renderer: GraspRenderer,
    pub left_drag_last_frame: bool,
    pub middle_drag_last_frame: bool,
    pub title_bar_drag: bool,
    pub rect: Rect2,
    pub window_list_index: usize,
}

impl HasMosaic for GraspEditorWindow {
    fn get_mosaic(&self) -> Arc<Mosaic> {
        Arc::clone(&self.editor_mosaic)
    }
}

impl RequireWindowFocus for GraspEditorWindow {}

impl PartialEq for GraspEditorWindow {
    fn eq(&self, other: &Self) -> bool {
        self.window_tile.id == other.window_tile.id
    }
}

impl GraspEditorWindow {
    pub fn request_quadtree_update(&self) {
        self.editor_mosaic.request_quadtree_update();
    }

    pub fn get_position_with_offset_and_pan(&self, position: Vec2) -> Vec2 {
        position
            .add(self.editor_data.pan)
            .add(self.editor_data.window_offset)
    }

    pub fn draw_debug(&self, s: &GuiState) {
        if !self.editor_data.debug {
            return;
        }

        s.ui.set_cursor_pos([10.0, 30.0]);
        s.ui.text(format!("Current state: {:?}", self.state));

        let quadtree = self.quadtree.lock().unwrap();
        quadtree.iter().for_each(|area| {
            let anchor_pos = self.pos_add_editor_offset(Vec2 {
                x: area.anchor().x as f32,
                y: area.anchor().y as f32,
            });

            let anchor_size = Vec2::new(area.width() as f32, area.height() as f32);
            let anchor_end = anchor_pos + anchor_size;
            let painter = s.ui.get_window_draw_list();
            let a: [f32; 2] = anchor_pos.into();
            let b: [f32; 2] = anchor_end.into();
            painter.add_rect(a, b, ImColor32::WHITE).build();
        });
    }
}

impl GraspEditorWindow {
    pub fn pos_add_editor_pan(&self, v: Vec2) -> Vec2 {
        v + self.editor_data.pan
    }

    pub fn pos_add_editor_offset(&self, v: Vec2) -> Vec2 {
        v + self.editor_data.window_offset
    }

    pub fn build_circle_area(&self, pos: Vec2, size: i32) -> Area<i32> {
        let pos = self.pos_add_editor_pan(pos);
        AreaBuilder::default()
            .anchor((pos.x as i32 - size, pos.y as i32 - size).into())
            .dimensions((size * 2, size * 2))
            .build()
            .unwrap()
    }

    pub fn build_cursor_area(&self) -> Area<i32> {
        self.build_circle_area(
            self.editor_data.cursor - self.editor_data.window_offset - self.editor_data.pan,
            1,
        )
    }

    fn internal_build_rect_area(min: Vec2, max: Vec2) -> Area<i32> {
        let _rect = Rect2::from_two_pos(min, max);
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

    pub fn build_label_area(&self, rect: Rect2) -> Area<i32> {
        let min = self.pos_add_editor_pan(rect.min());
        let max = self.pos_add_editor_pan(rect.max());
        Self::internal_build_rect_area(min, max)
    }

    pub fn build_rect_area(&self, rect: Rect2) -> Area<i32> {
        let min = rect.min();
        let max = rect.max();
        Self::internal_build_rect_area(min, max)
    }
}

impl GraspEditorWindow {
    fn insert_into_quadtree(&self, region: Area<i32>, obj: Tile) {
        if let Ok(mut quadtree) = self.quadtree.try_lock() {
            if let Some(area_id) = quadtree.insert(region, obj.id) {
                self.object_to_area.lock().unwrap().insert(obj.id, area_id);
            }
        } else {
            panic!("Quadtree lock poisoned!");
        }
    }

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
        let label_tile = obj.add_component("Label", par(""));
        label_tile.add_component(
            "Offset",
            vec![
                ("x".into(), Value::F32(10.0)),
                ("y".into(), Value::F32(0.0)),
            ],
        );

        let region = self.build_circle_area(pos, 12);
        let size = calc_text_size("");
        let label_region = self.build_label_area(Rect2 {
            x: pos.x,
            y: pos.y,
            width: size[0],
            height: size[1],
        });

        self.insert_into_quadtree(region, obj.clone());
        self.insert_into_quadtree(label_region, label_tile);
        self.editor_data.selected = vec![obj];
    }

    pub fn create_new_arrow(&mut self, source: &Tile, target: &Tile, middle_pos: Vec2) {
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

        arr.add_component(
            "Offset",
            vec![("x".into(), Value::F32(0.0)), ("y".into(), Value::F32(0.0))],
        );

        let label_tile = arr.add_component("Label", par(""));
        label_tile.add_component(
            "Offset",
            vec![
                ("x".into(), Value::F32(10.0)),
                ("y".into(), Value::F32(0.0)),
            ],
        );

        let region = self.build_circle_area(middle_pos, 12);
        let size = calc_text_size("");
        let label_region = self.build_label_area(Rect2 {
            x: middle_pos.x,
            y: middle_pos.y,
            width: size[0],
            height: size[1],
        });

        self.insert_into_quadtree(region, arr.clone());
        self.insert_into_quadtree(label_region, label_tile);

        self.editor_data.selected = vec![arr];
    }
}
