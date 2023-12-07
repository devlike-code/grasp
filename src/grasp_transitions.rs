use egui::{Pos2, Vec2};
use mosaic::{
    capabilities::{ArchetypeSubject, QueueCapability},
    internals::{void, MosaicCRUD, MosaicIO, TileFieldQuery, TileFieldSetter, Value, S32},
    iterators::{component_selectors::ComponentSelectors, tile_getters::TileGetters},
};

use crate::{
    editor_state_machine::{EditorState, EditorStateTrigger, StateMachine},
    grasp_common::{get_pos_from_tile, GraspEditorTab},
};

impl GraspEditorTab {
    pub(crate) fn request_quadtree_update(&self) {
        let queue = self
            .document_mosaic
            .get_all()
            .include_component("RefreshQuadtreeQueue")
            .get_targets()
            .next()
            .unwrap();
        let request = self.document_mosaic.new_object("void", void());
        self.document_mosaic.enqueue(&queue, &request);
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
                self.request_quadtree_update();
                Some(EditorState::Idle)
            }

            (_, EditorStateTrigger::DblClickToRename) => Some(EditorState::Rename),
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
                    self.create_new_arrow(&start, &tile);
                }
                self.editor_data.selected.clear();
                self.editor_data.link_start_pos = None;
                self.editor_data.link_end = None;
                self.request_quadtree_update();
                Some(EditorState::Idle)
            }
            (EditorState::Move, EditorStateTrigger::EndDrag) => {
                self.update_selected_positions_by(self.editor_data.cursor_delta);
                self.request_quadtree_update();
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
                        .get_component("Label")
                    {
                        //SET LABEL VALUE
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

    pub fn update_quadtree(&mut self) {
        for tile in self
            .document_mosaic
            .get_all()
            .include_component("Position")
            .get_targets()
        {
            let selected_pos_component = tile.get_component("Position").unwrap();

            if let (Value::F32(x), Value::F32(y)) = selected_pos_component.get_by(("x", "y")) {
                if let Some(area_id) = self.node_to_area.get(&tile.id) {
                    self.quadtree.delete_by_handle(*area_id);

                    let region = self.build_circle_area(Pos2::new(x, y), 10);
                    if let Some(area_id) = self.quadtree.insert(region, tile.id) {
                        self.node_to_area.insert(tile.id, area_id);
                    }
                } else {
                    let region = self.build_circle_area(Pos2::new(x, y), 10);
                    if let Some(area_id) = self.quadtree.insert(region, tile.id) {
                        self.node_to_area.insert(tile.id, area_id);
                    }
                }
            }
        }
    }
}
