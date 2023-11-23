use std::sync::{Arc, Mutex, MutexGuard};

use egui::{Color32, Key, Pos2, Rect, Sense, Ui, Vec2};
use grasp::internals::TileFieldGetter;
use grasp::internals::TileFieldSetter;
use grasp::{
    internals::{default_vals, Mosaic, MosaicIO, MosaicTypelevelCRUD, TileGetById, Value},
    iterators::{component_selectors::ComponentSelectors, tile_filters::TileFilters},
};

use crate::{grasp_data::GraspMetaData, tile_manager::TileManager};

#[derive(Debug)]
pub struct Pane {
    pub(crate) number: usize,
    pub grasp_data: GraspMetaData,
    pub drag_start_postion: Vec2,
}

impl Pane {
    pub fn new(number: usize) -> Pane {
        let meta = GraspMetaData::default();
        meta.mosaic
            .new_type("Position : {x : f32 , y: f32, is_selected: bool};")
            .unwrap();
        // meta.mosaic.new_object("Position", default_vals());
        // meta.mosaic.get_all().filter_objects().include_component("Position").for_each(|node|{println!("Isao");});
        Pane {
            number: number,
            grasp_data: meta,
            drag_start_postion: Default::default(),
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) -> egui_tiles::UiResponse {
        let color = egui::epaint::Hsva::new(0.103 * self.number as f32, 0.5, 0.5, 1.0);
        ui.painter().rect_filled(ui.max_rect(), 0.0, color);

        let response = ui.allocate_rect(ui.max_rect(), egui::Sense::click_and_drag());

        self.mockup_test(ui);

        // ui.put(
        //     Rect {
        //         min: Pos2 { x: 300.0, y: 100.0 },
        //         max: Pos2 {
        //             x: 450.0,
        //             y: 150.0,
        //         },
        //     },
        //     egui::Label::new("test").sense(Sense::click()),
        // );

        if ui
            .button("sssss")
            .double_clicked_by(egui::PointerButton::Primary)
        {
            println!("Button clicked");
        }

        if response.dragged_by(egui::PointerButton::Middle) {
            response.on_hover_cursor(egui::CursorIcon::Grab);
            return egui_tiles::UiResponse::DragStarted;
        } else {
            if response.double_clicked_by(egui::PointerButton::Primary) {
                println!("view clicked");
            }

            if response.clicked_by(egui::PointerButton::Primary) {
                println!("clickeeeed");
                let click_position = response.interact_pointer_pos().unwrap();
                self.drag_start_postion = Vec2::new(click_position.x, click_position.y);
            }

            if response.dragged_by(egui::PointerButton::Primary) {
                ui.put(
                    Rect {
                        min: Pos2 {
                            x: self.drag_start_postion.x,
                            y: self.drag_start_postion.y,
                        },
                        max: response.interact_pointer_pos().unwrap(),
                    },
                    egui::Button::new("pera"),
                );
            }
            egui_tiles::UiResponse::None
        }
    }

    pub fn mockup_test(&mut self, ui: &mut Ui) {
        let mut is_selection_dragged = false;
        let mut position_delta = Vec2::new(0.0, 0.0);
        let scope = ui.scope(|ui| {
            let new_button: egui::Response = ui.put(
                Rect {
                    min: Pos2 { x: 300.0, y: 100.0 },
                    max: Pos2 { x: 450.0, y: 150.0 },
                },
                egui::Button::new("Add node").sense(Sense::click()),
            );

            if new_button.clicked() {
                let tile = self
                    .grasp_data
                    .mosaic
                    .new_object("Position", default_vals());
            }

            let mut count = 0;
            // let x = (f)
            // self.grasp_data.graps_objects.iter_mut().for_each
            self.grasp_data
                .mosaic
                .get_all()
                .filter_objects()
                .include_component("Position")
                .for_each(|mut node| {
                    println!("pera");
                    let is_selected = node.get("is_selected").as_bool();
                    let color_button = if is_selected {
                        Color32::WHITE
                    } else {
                        Color32::RED
                    };

                    let node_x = node.get("x").as_f32();
                    let node_y = node.get("y").as_f32();
                    let test_button = ui.put(
                        Rect {
                            min: Pos2 {
                                x: node_x,
                                y: node_y,
                            },
                            max: Pos2 {
                                x: node_x + 150.0,
                                y: node_y + 100.0,
                            },
                        },
                        egui::Button::new(format!("Square '{}'", count))
                            .sense(Sense::click_and_drag())
                            .fill(color_button),
                    );
                    count += 1;
                    if test_button.clicked_by(egui::PointerButton::Secondary) {
                        node.set("is_selected", !is_selected);
                    }

                    if ui.ctx().input(|i| i.key_down(Key::ArrowRight)) {
                        ui.label("Right arrow pressed");
                        if is_selected {
                            node.set("x", node_x + 10.0);
                        }
                    }

                    if ui.ctx().input(|i| i.key_down(Key::ArrowLeft)) {
                        ui.label("Left arrow pressed");
                        if is_selected {
                            node.set("x", node_x - 10.0);
                        }
                    }

                    if ui.ctx().input(|i| i.key_down(Key::ArrowUp)) {
                        ui.label("Up arrow pressed");
                        if is_selected {
                            node.set("y", node_y - 10.0);
                        }
                    }

                    if ui.ctx().input(|i| i.key_down(Key::ArrowDown)) {
                        ui.label("Right down pressed");
                        if is_selected {
                            node.set("y", node_y + 10.0);
                        }
                    }

                    if test_button.dragged_by(egui::PointerButton::Primary) {
                        if let Some(new_drag_start_position) = test_button.interact_pointer_pos() {
                            is_selection_dragged = true;
                            position_delta = Vec2::new(
                                new_drag_start_position.x - node_x,
                                new_drag_start_position.y - node_y,
                            );
                        }
                    }

                    if test_button.drag_released_by(egui::PointerButton::Primary) {
                        position_delta.x = 0.0;
                        position_delta.y = 0.0;
                        is_selection_dragged = false;
                    }
                });
        });

        if is_selection_dragged {
            self.grasp_data
                .mosaic
                .get_all()
                .filter_objects()
                .include_component("Position")
                .for_each(|mut node| {
                    if node.get("is_seleted").as_bool() {
                        let node_x = node.get("x").as_f32();
                        let node_y = node.get("y").as_f32();
                        node.set("x", node_x + position_delta.x);
                        node.set("y", node_y + position_delta.y);
                    }
                });
        }
    }
}

impl egui_tiles::Behavior<Pane> for Pane {
    fn tab_title_for_pane(&mut self, pane: &Pane) -> egui::WidgetText {
        format!("Pane {}", pane.number).into()
    }

    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: egui_tiles::TileId,
        pane: &mut Pane,
    ) -> egui_tiles::UiResponse {
        pane.ui(ui)
    }
}

pub fn create_pane_tree<'a, 'b>(
    frame: &mut usize,
    _manager: &TileManager,
) -> egui_tiles::Tree<Pane> {
    let mut gen_pane = || {
        let pane = Pane::new(0);
        *frame += 1;
        pane
    };

    let mut tiles = egui_tiles::Tiles::default();

    let mut tabs = vec![];
    tabs.push(tiles.insert_pane(gen_pane()));

    let root = tiles.insert_tab_tile(tabs);

    egui_tiles::Tree::new(root, tiles)
}
