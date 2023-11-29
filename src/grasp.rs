use eframe::{egui, NativeOptions};
use ini::Ini;
use mosaic::{capabilities::SelectionCapability, internals::default_vals};

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

use ::mosaic::{
    internals::{
        self_val, EntityId, Mosaic, MosaicCRUD, MosaicIO, Tile, TileFieldGetter, TileFieldSetter,
        Value,
    },
    iterators::{
        component_selectors::ComponentSelectors, tile_filters::TileFilters,
        tile_getters::TileGetters,
    },
};
use egui::{ahash::HashMap, Align2, Color32, CursorIcon, FontId, Sense, Ui, Vec2, WidgetText};
use egui::{Pos2, Rect, Rounding, Stroke};
use egui_dock::TabViewer;
use itertools::Itertools;
use quadtree_rs::{
    area::{Area, AreaBuilder},
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
    //pub selection_owner: Option<Tile>,
    pub cursor: Pos2,
    pub cursor_delta: Vec2,
    pub rect_delta: Option<Vec2>,
    pub tab_offset: Pos2,
    pub link_start_pos: Option<Pos2>,
    pub link_end: Option<Tile>,
    pub rect_start_pos: Option<Pos2>,
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
    pub(crate) fn draw_arrow(
        painter: &egui::Painter,
        origin: Pos2,
        vec: Vec2,
        stroke: Stroke,
        start_offset: f32,
        end_offset: f32,
    ) {
        let rot = egui::emath::Rot2::from_angle(std::f32::consts::TAU / 15.0);
        let tip_length = 15.0;
        let dir = vec.normalized();
        println!("{:?}", dir);
        let a_start: Pos2 = origin + dir * start_offset;
        let tip = a_start + vec - dir * (start_offset + end_offset);
        let middle = a_start.lerp(tip, 0.5);

        let shape = egui::epaint::QuadraticBezierShape {
            points: [a_start, middle, tip],
            closed: false,
            fill: Color32::TRANSPARENT,
            stroke: Stroke {
                width: 2.0,
                color: Color32::LIGHT_BLUE,
            },
        };
        painter.add(shape);
        painter.line_segment([tip, tip - tip_length * (rot * dir)], stroke);
        painter.line_segment([tip, tip - tip_length * (rot.inverse() * dir)], stroke);
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
                Stroke::new(1.0, Color32::BLUE),
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
            painter.circle_filled(self.pos_into_editor(pos), 10.0, Color32::GRAY);

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
                    Color32::BLACK,
                );
            }
        }

        for arrow in self.document_mosaic.get_all().filter_arrows() {
            let source_pos = self.pos_into_editor(
                get_pos_from_tile(&self.document_mosaic.get(arrow.source_id()).unwrap()).unwrap(),
            );
            let target_pos = self.pos_into_editor(
                get_pos_from_tile(&self.document_mosaic.get(arrow.target_id()).unwrap()).unwrap(),
            );

            Self::draw_arrow(
                &painter,
                source_pos,
                target_pos - source_pos,
                Stroke::new(1.0, Color32::LIGHT_BLUE),
                10.0,
                10.0,
            );
        }
        // if let Some(owner) = &self.editor_data.selection_owner {
        for selected in &self.editor_data.selected {
            let stroke = Stroke {
                width: 0.5,
                color: Color32::RED,
            };
            println!("SELECTED {:?}", selected);
            let selected_pos = Pos2::new(selected.get("x").as_f32(), selected.get("y").as_f32());

            painter.circle(
                self.pos_into_editor(selected_pos),
                11.0,
                Color32::RED,
                stroke,
            );
            // }
        }

        //self.draw_debug(ui);
    }

    pub fn sense(&mut self, ui: &mut Ui) {
        let (resp, _) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());

        if let Some(pos) = resp.hover_pos() {
            self.editor_data.cursor = self.pos_from_editor(pos);
        }

        self.editor_data.cursor_delta = resp.drag_delta();

        if let Some(mut rect_delta) = self.editor_data.rect_delta {
            rect_delta += resp.drag_delta();
            self.editor_data.rect_delta = Some(rect_delta);
        } else {
            self.editor_data.rect_delta = Some(resp.drag_delta());
        }

        let result = self
            .quadtree
            .query(build_area(self.editor_data.cursor, 1))
            .collect_vec();

        if resp.double_clicked() && result.is_empty() {
            self.trigger(EditorStateTrigger::DblClickToCreate);
        } else if resp.drag_started_by(egui::PointerButton::Primary) && !result.is_empty() {
            let is_alt_down = {
                let mut alt_down = false;
                ui.input(|input_state| {
                    alt_down = input_state.modifiers.alt;
                });
                alt_down
            };

            if is_alt_down {
                self.editor_data.selected = vec![self
                    .document_mosaic
                    .get(*result.first().unwrap().value_ref())
                    .unwrap()];
                self.trigger(EditorStateTrigger::DragToLink);
            } else {
                self.editor_data.selected = result
                    .into_iter()
                    .flat_map(|next| self.document_mosaic.get(*next.value_ref()))
                    .collect_vec();
                self.trigger(EditorStateTrigger::DragToMove);
            }
            println!("---------------DRAAAG");
        } else if resp.drag_started_by(egui::PointerButton::Primary) && result.is_empty() {
            self.editor_data.selected = vec![];

            self.editor_data.rect_start_pos = Some(self.editor_data.cursor);

            self.trigger(EditorStateTrigger::DragToSelect);
        } else if resp.drag_started_by(egui::PointerButton::Secondary) {
            self.trigger(EditorStateTrigger::DragToPan);
        } else if resp.drag_released() {
            self.trigger(EditorStateTrigger::EndDrag);
        }
    }

    fn update_position_for_selected(&mut self, pos: Pos2) {
        for tile in &mut self.editor_data.selected {
            tile.set("x", pos.x);
            tile.set("y", pos.y);
        }
    }

    fn update_quadtree_for_selected(&mut self) {
        for tile in &mut self.editor_data.selected {
            if let Some(area_id) = self.node_area.get(&tile.id) {
                self.quadtree.delete_by_handle(*area_id);

                let region = build_area(self.editor_data.cursor, 10);
                if let Some(area_id) = self.quadtree.insert(region, tile.id) {
                    self.node_area.insert(tile.id, area_id);
                }
            }
        }
    }

    pub fn update(&mut self, ui: &mut Ui) {
        println!("self.state {:?}", self.state);
        match &self.state {
            EditorState::Idle => {}
            EditorState::Move => {
                // TODO: delta
                self.update_position_for_selected(self.editor_data.cursor);
            }
            EditorState::Pan => {
                ui.ctx().set_cursor_icon(CursorIcon::Move);
                self.editor_data.pan += self.editor_data.cursor_delta;
            }
            EditorState::Link => {
                if let Some(start_pos) = self.editor_data.link_start_pos {
                    let mut end_pos = self.editor_data.cursor;
                    let mut end_offset = 0.0;
                    if let Some(end) = &self.editor_data.link_end {
                        end_pos = get_pos_from_tile(end).unwrap();
                        end_offset = 10.0;
                    }

                    Self::draw_arrow(
                        ui.painter(),
                        self.pos_into_editor(start_pos),
                        end_pos - start_pos,
                        Stroke::new(2.0, Color32::LIGHT_GREEN),
                        10.0,
                        end_offset,
                    )
                }

                let region = build_area(self.editor_data.cursor, 1);
                let query = self.quadtree.query(region).collect_vec();
                if !query.is_empty() {
                    let tile_id = query.first().unwrap().value_ref();
                    self.editor_data.link_end = self.document_mosaic.get(*tile_id);
                } else {
                    self.editor_data.link_end = None;
                }
            }
            EditorState::Rect => {
                if let Some(min) = self.editor_data.rect_start_pos {
                    if let Some(delta) = self.editor_data.rect_delta {
                        let end_pos = min + delta;
                        let rect = Rect::from_two_pos(min, end_pos);
                        let semi_transparent_light_yellow =
                            Color32::from_rgba_unmultiplied(255, 255, 120, 2);
                        let semi_transparent_light_blue =
                            Color32::from_rgba_unmultiplied(255, 120, 255, 2);

                        let stroke = Stroke {
                            width: 0.5,
                            color: Color32::LIGHT_BLUE,
                        };

                        // ui.painter().rect(
                        //     rect,
                        //     Rounding::default(),
                        //     semi_transparent_light_yellow,
                        //     stroke,
                        // );

                        let rect_area = Rect::from_two_pos(
                            self.pos_into_editor(min),
                            self.pos_into_editor(end_pos),
                        );

                        ui.painter().rect(
                            rect_area,
                            Rounding::default(),
                            semi_transparent_light_blue,
                            stroke,
                        );
                        let region = build_rect_area(rect);
                        let query = self.quadtree.query(region).collect_vec();
                        if !query.is_empty() {
                            self.editor_data.selected = query
                                .into_iter()
                                .flat_map(|e| self.document_mosaic.get(e.value_ref().clone()))
                                .collect_vec();
                        } else {
                            self.editor_data.selected = vec![];
                        }
                    }
                }
            }
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

        let region = build_area(pos, 10);

        if let Some(area_id) = self.quadtree.insert(region, obj.id) {
            self.node_area.insert(obj.id, area_id);
        }
    }

    pub fn create_new_arrow(&mut self, source: &Tile, target: &Tile) {
        let _arr = self
            .document_mosaic
            .new_arrow(source, target, "Arrow", default_vals());

        // TODO: add quadtree representation
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
            (EditorState::Idle, EditorStateTrigger::DragToLink) => {
                self.editor_data.link_start_pos =
                    get_pos_from_tile(self.editor_data.selected.first().unwrap());
                Some(EditorState::Link)
            }
            (EditorState::Idle, EditorStateTrigger::DragToMove) => Some(EditorState::Move),
            (EditorState::Idle, EditorStateTrigger::DragToSelect) => Some(EditorState::Rect),
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
                self.update_position_for_selected(self.editor_data.cursor);
                self.update_quadtree_for_selected();

                Some(EditorState::Idle)
            }
            (EditorState::Rect, EditorStateTrigger::EndDrag) => {
                self.editor_data.rect_start_pos = None;
                self.editor_data.rect_delta = None;
                // if let Some(owner) = &self.editor_data.selection_owner {
                //     self.document_mosaic
                //         .fill_selection(owner, self.editor_data.selected.as_slice())
                // } else {
                //     let owner = self.document_mosaic.make_selection();
                //     self.document_mosaic
                //         .fill_selection(&owner, &self.editor_data.selected.as_slice());
                //     self.editor_data.selection_owner = Some(owner);
                // }

                Some(EditorState::Idle)
            }
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

fn build_rect_area(rect: Rect) -> Area<i32> {
    let dim_x = rect.max.x - rect.min.x;
    let dim_y = rect.max.y - rect.min.y;
    println!("{:?}, {:?}", dim_x, dim_y);
    AreaBuilder::default()
        .anchor((rect.min.x as i32, rect.min.y as i32).into())
        .dimensions((
            (rect.max.x - rect.min.x).abs() as i32 + 1,
            (rect.max.y - rect.min.y).abs() as i32 + 1,
        ))
        .build()
        .unwrap()
}

fn get_pos_from_tile(tile: &Tile) -> Option<Pos2> {
    if let (Value::F32(x), Value::F32(y)) = tile.get(("x", "y")) {
        Some(Pos2::new(x, y))
    } else {
        None
    }
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
