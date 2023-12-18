use crate::core::gui::windowing::set_window_focus;
use crate::core::math::rect2::Rect2;
use crate::core::math::vec2::Vec2;
use crate::editor_state_machine::EditorState;
use crate::grasp_common::GraspEditorData;
use crate::grasp_render::GraspRenderer;
use crate::GuiState;
use ::mosaic::internals::{EntityId, Mosaic, MosaicCRUD, MosaicIO, Tile, Value};
use imgui::sys::ImColor;
use imgui::ImColor32;
use itertools::Itertools;
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

pub struct GraspEditorWindow {
    pub name: String,
    pub window_tile: Tile,
    pub state: EditorState,
    pub quadtree: Mutex<Quadtree<i32, EntityId>>,
    pub document_mosaic: Arc<Mosaic>,
    pub object_to_area: Mutex<HashMap<EntityId, Vec<u64>>>,
    pub collage: Box<Collage>,
    pub ruler_visible: bool,
    pub grid_visible: bool,
    pub editor_data: GraspEditorData,
    pub renderer: Box<dyn GraspRenderer>,
}

impl PartialEq for GraspEditorWindow {
    fn eq(&self, other: &Self) -> bool {
        self.window_tile.id == other.window_tile.id
    }
}

impl GraspEditorWindow {
    pub fn show(&mut self, s: &GuiState) {
        self.sense(s);
        let name = self.name.clone();
        s.ui.window(name)
            .size([700.0, 500.0], imgui::Condition::Appearing)
            .position(
                [
                    200.0 + 50.0 * (self.window_tile.id % 5) as f32,
                    200.0 - 20.0 * (self.window_tile.id % 5) as f32,
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
                        focus.set("self", self.window_tile.id as u64);
                    }
                }

                if let Some(request) = self.document_mosaic.dequeue(&self.window_tile) {
                    // todo
                    self.update_quadtree(None);
                    set_window_focus(&self.name);
                    request.iter().delete();
                }

                self.renderer.draw(&self.document_mosaic, s);
                self.draw_debug(s);
                self.update_context_menu(s);
            });
        self.update(s);
    }

    pub fn draw_debug(&self, s: &GuiState) {
        let quadtree = self.quadtree.lock().unwrap();
        quadtree.iter().for_each(|area| {
            let anchor_pos = self.pos_with_pan(Vec2 {
                x: area.anchor().x as f32,
                y: area.anchor().y as f32,
            }) - self.editor_data.tab_offset;

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
    pub fn create_new_object(&self, pos: Vec2) {
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

        let mut quadtree = self.quadtree.lock().unwrap();
        if let Some(area_id) = quadtree.insert(region, obj.id) {
            self.object_to_area
                .lock()
                .unwrap()
                .insert(obj.id, vec![area_id]);
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

        {
            let mut quadtree = self.quadtree.lock().unwrap();
            if let Some(area_id) = quadtree.insert(region, arr.id) {
                let mut object_to_area = self.object_to_area.lock().unwrap();
                if let Some(areas_vec) = object_to_area.get_mut(&arr.id) {
                    areas_vec.push(area_id);
                } else {
                    object_to_area.insert(arr.id, vec![area_id]);
                }
            }
        }

        for r in bezier_rects {
            let region = self.build_rect_area(r);
            let mut quadtree = self.quadtree.lock().unwrap();
            if let Some(area_id) = quadtree.insert(region, arr.id) {
                let mut object_to_area = self.object_to_area.lock().unwrap();
                if let Some(areas_vec) = object_to_area.get_mut(&arr.id) {
                    areas_vec.push(area_id);
                    //self.object_to_area.insert(arr.id, areas_vec.to_owned());
                }
            }
        }
    }
}
