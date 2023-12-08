use std::sync::Arc;

use egui::{epaint::QuadraticBezierShape, Color32, Pos2, Stroke, Vec2};
use mosaic::{
    capabilities::{ArchetypeSubject, QueueCapability},
    internals::{void, Mosaic, MosaicCRUD, MosaicIO, TileFieldQuery, TileFieldSetter, Value},
    iterators::{component_selectors::ComponentSelectors, tile_getters::TileGetters},
};
use rand::distributions::uniform::UniformSampler;

use crate::{
    editor_state_machine::{EditorState, EditorStateTrigger, StateMachine},
    grasp_common::{get_pos_from_tile, GraspEditorTab},
};

pub trait QuadtreeUpdateCapability {
    fn request_quadtree_update(&self);
}

impl QuadtreeUpdateCapability for Arc<Mosaic> {
    fn request_quadtree_update(&self) {
        let queue = self
            .get_all()
            .include_component("RefreshQuadtreeQueue")
            .get_targets()
            .next()
            .unwrap();
        let request = self.new_object("void", void());
        self.enqueue(&queue, &request);
    }
}

impl StateMachine for GraspEditorTab {
    type Trigger = EditorStateTrigger;
    type State = EditorState;

    fn on_transition(&mut self, from: Self::State, trigger: Self::Trigger) -> Option<EditorState> {
        println!("TRANSITIION FROM = {:?}, TRIGGER -> {:?}", from, trigger);
        match (from, trigger) {
            (_, EditorStateTrigger::DblClickToCreate) => {
                self.create_new_object(self.editor_data.cursor);
                self.document_mosaic.request_quadtree_update();
                Some(EditorState::Idle)
            }

            (_, EditorStateTrigger::DblClickToRename) => Some(EditorState::PropertyChanging),
            (_, EditorStateTrigger::ClickToReposition) => Some(EditorState::Reposition),

            (_, EditorStateTrigger::MouseDownOverNode) => None,
            (_, EditorStateTrigger::ClickToSelect) => Some(EditorState::Idle),
            (_, EditorStateTrigger::ClickToDeselect) => {
                self.editor_data.selected.clear();
                Some(EditorState::Idle)
            }
            (_, EditorStateTrigger::DragToPan) => {
                self.editor_data.previous_pan = self.editor_data.pan;
                Some(EditorState::Pan)
            }
            (_, EditorStateTrigger::DragToLink) => {
                self.editor_data.link_start_pos =
                    get_pos_from_tile(self.editor_data.selected.first().unwrap());
                Some(EditorState::Link)
            }
            (_, EditorStateTrigger::DragToMove) => Some(EditorState::Move),
            (_, EditorStateTrigger::ClickToContextMenu) => Some(EditorState::ContextMenu),
            (EditorState::ContextMenu, _) => {
                self.response = None;
                Some(EditorState::Idle)
            }
            (EditorState::Idle, EditorStateTrigger::DragToSelect) => {
                self.editor_data.rect_delta = Some(Vec2::ZERO);
                self.editor_data.rect_start_pos = Some(self.editor_data.cursor);
                Some(EditorState::Rect)
            }

            (EditorState::Pan, EditorStateTrigger::EndDrag) => Some(EditorState::Idle),
            (EditorState::Link, EditorStateTrigger::EndDrag) => {
                if let Some(tile) = self.editor_data.link_end.take() {
                    let start = self.editor_data.selected.first().unwrap().clone();
                    let mut src_pos = Pos2::default();
                    let mut tgt_pos = Pos2::default();
                    if let (
                        (Value::F32(s_x), Value::F32(s_y)),
                        (Value::F32(t_x), Value::F32(t_y)),
                    ) = (
                        start.get_component("Position").unwrap().get_by(("x", "y")),
                        tile.get_component("Position").unwrap().get_by(("x", "y")),
                    ) {
                        src_pos = Pos2::new(s_x, s_y);
                        tgt_pos = Pos2::new(t_x, t_y);
                    }

                    let mid_pos = src_pos.lerp(tgt_pos, 0.5);

                    let qb = QuadraticBezierShape::from_points_stroke(
                        [src_pos, mid_pos, tgt_pos],
                        false,
                        Color32::TRANSPARENT,
                        Stroke::new(1.0, Color32::LIGHT_BLUE),
                    );
                    let sample = qb.flatten(Some(0.1));
                    let mut rects = vec![];

                    for i in (0..sample.len() - 2).step_by(2) {
                        let rect = egui::Rect::from_two_pos(sample[i], sample[i + 2]);
                        rects.push(rect);

                        let rect = egui::Rect::from_two_pos(sample[i + 1], sample[i + 3]);
                        rects.push(rect);
                    }

                    self.create_new_arrow(&start, &tile, mid_pos, rects);
                }

                self.editor_data.selected.clear();
                self.editor_data.link_start_pos = None;
                self.editor_data.link_end = None;
                self.document_mosaic.request_quadtree_update();
                Some(EditorState::Idle)
            }
            (EditorState::Move, EditorStateTrigger::EndDrag) => {
                self.update_selected_positions_by(self.editor_data.cursor_delta);
                self.document_mosaic.request_quadtree_update();
                Some(EditorState::Idle)
            }
            (EditorState::Rect, EditorStateTrigger::EndDrag) => {
                self.editor_data.rect_start_pos = None;
                self.editor_data.rect_delta = None;
                Some(EditorState::Idle)
            }

            (EditorState::PropertyChanging, _) => {
                self.editor_data.tile_changing = None;
                self.editor_data.field_changing = None;
                self.editor_data.previous_text.clear();
                self.editor_data.text.clear();
                Some(EditorState::Idle)
            }

            (EditorState::Reposition, _) => {
                if let Some(tile_id) = self.editor_data.repositioning {
                    if self.document_mosaic.is_tile_valid(&tile_id) {
                        if let Some(mut pos) = self
                            .document_mosaic
                            .get(tile_id)
                            .unwrap()
                            .get_component("Position")
                        {
                            pos.set(
                                "x",
                                self.editor_data.x_pos.parse::<f32>().unwrap_or_default(),
                            );
                            pos.set(
                                "y",
                                self.editor_data.y_pos.parse::<f32>().unwrap_or_default(),
                            );
                        }
                    }
                }

                self.update_quadtree_for_selected();

                self.editor_data.tile_changing = None;
                self.editor_data.previous_text.clear();
                self.editor_data.text.clear();
                Some(EditorState::Idle)
            }

            (s, t) => {
                println!("TRANSITION NOT DEALT WITH: {:?} {:?}!", s, t);
                None
            }
        }
    }

    fn get_current_state(&self) -> Self::State {
        self.state
    }

    fn move_to(&mut self, next: Self::State) {
        self.state = next;
    }
}

impl GraspEditorTab {
    pub fn update_selected_positions_by(&mut self, dp: Vec2) {
        for tile in &mut self.editor_data.selected {
            let mut selected_pos_component = tile.get_component("Position").unwrap();
            if let (Value::F32(x), Value::F32(y)) = selected_pos_component.get_by(("x", "y")) {
                selected_pos_component.set("x", x + dp.x);
                selected_pos_component.set("y", y + dp.y);
            }
        }
    }

    pub fn update_quadtree_for_selected(&mut self) {
        for tile in &self.editor_data.selected {
            let selected_pos_component = tile.get_component("Position").unwrap();

            if let (Value::F32(x), Value::F32(y)) = selected_pos_component.get_by(("x", "y")) {
                if let Some(area_id) = self.object_to_area.get(&tile.id) {
                    self.quadtree.delete_by_handle(*area_id);

                    let region = self.build_circle_area(Pos2::new(x, y), 10);
                    if let Some(area_id) = self.quadtree.insert(region, tile.id) {
                        self.object_to_area.insert(tile.id, area_id);
                    }
                }
            }
        }
    }

    pub fn update_quadtree(&mut self) {
        for tile in self
            .document_mosaic
            .get_all()
            .include_component("Position")
            .get_targets()
        {
            let selected_pos_component = tile.get_component("Position").unwrap();

            if let (Value::F32(x), Value::F32(y)) = selected_pos_component.get_by(("x", "y")) {
                if let Some(area_id) = self.object_to_area.get(&tile.id) {
                    self.quadtree.delete_by_handle(*area_id);

                    let region = self.build_circle_area(Pos2::new(x, y), 10);
                    if let Some(area_id) = self.quadtree.insert(region, tile.id) {
                        self.object_to_area.insert(tile.id, area_id);
                    }
                } else {
                    let region = self.build_circle_area(Pos2::new(x, y), 10);
                    if let Some(area_id) = self.quadtree.insert(region, tile.id) {
                        self.object_to_area.insert(tile.id, area_id);
                    }
                }
            }
        }
    }
}
