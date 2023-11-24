use eframe::{egui, NativeOptions};
use ini::Ini;

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

use ::grasp::{
    internals::{
        self_val, EntityId, Mosaic, MosaicCRUD, MosaicIO, MosaicTypelevelCRUD, Tile,
        TileFieldGetter, TileFieldSetter, Value,
    },
    iterators::{
        component_selectors::ComponentSelectors, tile_filters::TileFilters,
        tile_getters::TileGetters,
    },
};
use egui::{
    ahash::HashMap, Align2, Color32, CursorIcon, FontId, PlatformOutput, Sense, Ui, Vec2,
    WidgetText,
};
use egui::{Pos2, Rect, Rounding, Stroke};
use egui_dock::{DockArea, DockState, Style, TabViewer};
use itertools::Itertools;
use quadtree_rs::{
    area::{Area, AreaBuilder},
    point::Point,
    Quadtree,
};
use std::{
    ops::{Add, Sub},
    sync::Arc,
};

use crate::editor_state_machine::{EditorState, EditorStateTrigger, StateMachine};
#[derive(Default)]
pub struct GraspEditorData {
    pub pan: Vec2,
    pub selected: Vec<Tile>,
    pub cursor: Pos2,
    pub cursor_delta: Vec2,
    pub tab_offset: Pos2,
}

pub struct GraspEditorTab {
    pub name: String,
    pub state: EditorState,
    pub quadtree: Quadtree<i32, EntityId>,
    pub document_mosaic: Arc<Mosaic>,
    pub node_area: HashMap<EntityId, u64>,
    pub editor_data: GraspEditorData,
}

impl GraspEditorTab {
    fn pos_into_editor(&self, v: Pos2) -> Pos2 {
        v.add(self.editor_data.pan)
            .add(self.editor_data.tab_offset.to_vec2())
    }

    fn pos_from_editor(&self, v: Pos2) -> Pos2 {
        v.sub(self.editor_data.pan)
            .sub(self.editor_data.tab_offset.to_vec2())
    }

    pub fn draw_debug(&mut self, ui: &mut Ui) {
        let painter = ui.painter();

        self.quadtree.iter().for_each(|area| {
            let anchor_pos = self.pos_into_editor(Pos2 {
                x: area.anchor().x as f32,
                y: area.anchor().y as f32,
            });
            painter.rect(
                Rect {
                    min: Pos2 {
                        x: anchor_pos.x,
                        y: anchor_pos.y,
                    },
                    max: Pos2 {
                        x: (anchor_pos.x + area.width() as f32),
                        y: (anchor_pos.y + area.height() as f32),
                    },
                },
                Rounding::ZERO,
                Color32::TRANSPARENT,
                Stroke::new(1.0, Color32::RED),
            );
        });
    }

    pub fn render(&mut self, ui: &mut Ui) {
        let painter = ui.painter();

        // Rendering

        for node in self
            .document_mosaic
            .get_all()
            .filter_objects()
            .include_component("Position")
        {
            // Draw node
            let pos = Pos2::new(node.get("x").as_f32(), node.get("y").as_f32());
            painter.circle_filled(self.pos_into_editor(pos), 5.0, Color32::WHITE);

            // Maybe draw label
            if let Some(label) = node
                .into_iter()
                .get_descriptors()
                .include_component("Label")
                .next()
            {
                painter.text(
                    self.pos_into_editor(pos.add(Vec2::new(10.0, 10.0))),
                    Align2::LEFT_CENTER,
                    label.get("self").as_s32().to_string(),
                    FontId::default(),
                    Color32::WHITE,
                );
            }
        }

        for arrow in self
            .document_mosaic
            .get_all()
            .filter_arrows()
            .include_component("Position")
        {
            // painter.arrow(
            //     Pos2::new(200.0, 200.0),
            //     Vec2::new(100.0, 100.0),
            //     Stroke::new(1.0, Color32::WHITE),
            // );
        }

        self.draw_debug(ui);
    }

    pub fn sense(&mut self, ui: &mut Ui) {
        let (resp, _) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());

        if let Some(pos) = resp.hover_pos() {
            self.editor_data.cursor = self.pos_from_editor(pos);
        }

        self.editor_data.cursor_delta = resp.drag_delta();

        let result = self
            .quadtree
            .query(build_area(self.editor_data.cursor, 1))
            .collect_vec();

        if resp.double_clicked() && result.is_empty() {
            self.trigger(EditorStateTrigger::DblClickToCreate);
        } else if resp.drag_started_by(egui::PointerButton::Primary) && !result.is_empty() {
            self.editor_data.selected = result
                .into_iter()
                .flat_map(|next| self.document_mosaic.get(*next.value_ref()))
                .collect_vec();
            self.trigger(EditorStateTrigger::DragToMove);
        } else if resp.drag_started_by(egui::PointerButton::Primary) && result.is_empty() {
            self.editor_data.selected = vec![];
            self.trigger(EditorStateTrigger::DragToSelect);
        } else if resp.drag_started_by(egui::PointerButton::Secondary) {
            self.trigger(EditorStateTrigger::DragToPan);
        } else if resp.drag_released() {
            self.trigger(EditorStateTrigger::EndDrag);
        }
    }

    fn update_selected_position(&mut self, pos: Pos2) {
        for tile in &mut self.editor_data.selected {
            tile.set("x", pos.x);
            tile.set("y", pos.y);
        }
    }

    fn update_selected_quadtree(&mut self) {
        for tile in &mut self.editor_data.selected {
            if let Some(area_id) = self.node_area.get(&tile.id) {
                self.quadtree.delete_by_handle(*area_id);

                let region = build_area(self.editor_data.cursor, 5);
                if let Some(area_id) = self.quadtree.insert(region, tile.id) {
                    self.node_area.insert(tile.id, area_id);
                }
            }
        }
    }

    pub fn update(&mut self, ui: &mut Ui) {
        match &self.state {
            EditorState::Idle => {}
            EditorState::Move => {
                // TODO: delta
                self.update_selected_position(self.editor_data.cursor);
            }
            EditorState::Pan => {
                ui.ctx().set_cursor_icon(CursorIcon::Move);
                self.editor_data.pan += self.editor_data.cursor_delta;
            }
            EditorState::Link => {}
            EditorState::Rect => {}
        }
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

        let region = build_area(pos, 5);

        if let Some(area_id) = self.quadtree.insert(region, obj.id) {
            self.node_area.insert(obj.id, area_id);
        }
    }
}

impl StateMachine for GraspEditorTab {
    type Trigger = EditorStateTrigger;
    type State = EditorState;

    fn on_transition(&mut self, from: Self::State, trigger: Self::Trigger) -> Option<EditorState> {
        println!("{:?} {:?}", from, trigger);
        match (from, trigger) {
            (_, EditorStateTrigger::DblClickToCreate) => {
                self.create_new_object(self.editor_data.cursor);
                Some(EditorState::Idle)
            }

            (_, EditorStateTrigger::MouseDownOverNode) => None,
            (_, EditorStateTrigger::ClickToSelect) => None,
            (_, EditorStateTrigger::ClickToDeselect) => {
                self.editor_data.selected.clear();
                Some(EditorState::Idle)
            }
            (EditorState::Idle, EditorStateTrigger::DragToPan) => Some(EditorState::Pan),
            (EditorState::Idle, EditorStateTrigger::DragToLink) => None,
            (EditorState::Idle, EditorStateTrigger::DragToMove) => Some(EditorState::Move),
            (EditorState::Idle, EditorStateTrigger::DragToSelect) => None,
            (EditorState::Pan, EditorStateTrigger::EndDrag) => Some(EditorState::Idle),
            (EditorState::Link, EditorStateTrigger::EndDrag) => None,
            (EditorState::Move, EditorStateTrigger::EndDrag) => {
                self.update_selected_position(self.editor_data.cursor);
                self.update_selected_quadtree();

                Some(EditorState::Idle)
            }
            (EditorState::Rect, EditorStateTrigger::EndDrag) => None,
            _ => None,
        }
    }

    fn get_current_state(&self) -> Self::State {
        self.state
    }

    fn move_to(&mut self, next: Self::State) {
        self.state = next;
    }
}

#[derive(Default)]
pub struct GraspEditorTabs {
    pub current_tab: u32,
}

impl GraspEditorTabs {
    pub fn increment(&mut self) -> u32 {
        self.current_tab += 1;
        self.current_tab
    }
}

fn build_area(pos: Pos2, size: i32) -> Area<i32> {
    AreaBuilder::default()
        .anchor((pos.x as i32 - size, pos.y as i32 - size).into())
        .dimensions((size * 2, size * 2))
        .build()
        .unwrap()
}

impl TabViewer for GraspEditorTabs {
    // This associated type is used to attach some data to each tab.
    type Tab = GraspEditorTab;

    // Returns the current `tab`'s title.
    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        tab.name.as_str().into()
    }

    // Defines the contents of a given `tab`.
    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        let xy = ui.clip_rect().left_top();
        tab.editor_data.tab_offset = xy;

        tab.render(ui);
        tab.sense(ui);
        tab.update(ui);
    }
}
