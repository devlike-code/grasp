use egui::{Pos2, Vec2};
use mosaic::{
    internals::{MosaicIO, TileFieldQuery, TileFieldSetter, Value, S32},
    iterators::{component_selectors::ComponentSelectors, tile_getters::TileGetters},
};

use crate::{
    editor_state_machine::{EditorState, EditorStateTrigger, StateMachine},
    grasp_common::{get_pos_from_tile, GraspEditorTab},
};

impl StateMachine for GraspEditorTab {
    type Trigger = EditorStateTrigger;
    type State = EditorState;

    fn on_transition(&mut self, from: Self::State, trigger: Self::Trigger) -> Option<EditorState> {
        println!("ON TRANSITION {:?} --{:?}--> ", from, trigger);
        match (from, trigger) {
            (_, EditorStateTrigger::DblClickToCreate) => {
                self.create_new_object(self.editor_data.cursor);
                Some(EditorState::Idle)
            }

            (_, EditorStateTrigger::DblClickToRename) => Some(EditorState::Rename),

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

            (EditorState::Idle, EditorStateTrigger::DragToSelect) => {
                self.editor_data.rect_delta = Some(Vec2::ZERO);
                self.editor_data.rect_start_pos = Some(self.editor_data.cursor);
                Some(EditorState::Rect)
            }

            (EditorState::Pan, EditorStateTrigger::EndDrag) => Some(EditorState::Idle),
            (EditorState::Link, EditorStateTrigger::EndDrag) => {
                if let Some(tile) = self.editor_data.link_end.take() {
                    let start = self.editor_data.selected.first().unwrap().clone();
                    self.create_new_arrow(&start, &tile);
                }
                self.editor_data.selected.clear();
                self.editor_data.link_start_pos = None;
                self.editor_data.link_end = None;
                Some(EditorState::Idle)
            }
            (EditorState::Move, EditorStateTrigger::EndDrag) => {
                self.update_selected_positions_by(self.editor_data.cursor_delta);
                self.update_quadtree_for_selected();
                Some(EditorState::Idle)
            }
            (EditorState::Rect, EditorStateTrigger::EndDrag) => {
                self.editor_data.rect_start_pos = None;
                self.editor_data.rect_delta = None;
                Some(EditorState::Idle)
            }

            (EditorState::Rename, _) => {
                if let Some(tile) = self.editor_data.renaming {
                    if let Some(mut label) = self
                        .document_mosaic
                        .get(tile)
                        .unwrap()
                        .iter()
                        .get_descriptors()
                        .include_component("Label")
                        .next()
                    {
                        TileFieldSetter::<S32>::set(
                            &mut label,
                            "self",
                            self.editor_data.text.as_str().into(),
                        )
                    }
                }

                self.editor_data.renaming = None;
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
            if let (Value::F32(x), Value::F32(y)) = tile.get_by(("x", "y")) {
                tile.set("x", x + dp.x);
                tile.set("y", y + dp.y);
            }
        }
    }

    pub fn update_quadtree_for_selected(&mut self) {
        for tile in &self.editor_data.selected {
            if let (Value::F32(x), Value::F32(y)) = tile.get_by(("x", "y")) {
                if let Some(area_id) = self.node_to_area.get(&tile.id) {
                    self.quadtree.delete_by_handle(*area_id);

                    let region = self.build_circle_area(Pos2::new(x, y), 10);
                    if let Some(area_id) = self.quadtree.insert(region, tile.id) {
                        self.node_to_area.insert(tile.id, area_id);
                    }
                }
            }
        }
    }
}
