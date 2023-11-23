use std::sync::Arc;
use std::sync::Mutex;

use egui::{Color32, Key, Pos2, Rect, Sense, Ui, Vec2};
use grasp::internals::TileFieldGetter;
use grasp::internals::TileFieldSetter;
use grasp::{
    internals::{default_vals, MosaicIO, MosaicTypelevelCRUD},
    iterators::{component_selectors::ComponentSelectors, tile_filters::TileFilters},
};

use crate::editor::GraspEditor;
use crate::tile_manager::TileManager;

pub struct Pane {
    pub(crate) number: usize,
    pub drag_start_postion: Vec2,
    pub editor: Arc<Mutex<GraspEditor>>,
}

impl Pane {
    pub fn new(number: usize, editor: &Arc<Mutex<GraspEditor>>) -> Pane {
        Pane {
            number,
            drag_start_postion: Default::default(),
            editor: Arc::clone(editor),
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
            egui_tiles::UiResponse::DragStarted
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
        let _scope = ui.scope(|ui| {
            let new_button: egui::Response = ui.put(
                Rect {
                    min: Pos2 { x: 300.0, y: 100.0 },
                    max: Pos2 { x: 450.0, y: 150.0 },
                },
                egui::Button::new("Add node").sense(Sense::click()),
            );

            let editor = self.editor.lock().unwrap();
            let mosaic = editor.mosaic;

            if new_button.clicked() {
                let _tile = editor.mosaic.new_object("Position", default_vals());
            }

            let mut count = 0;
            // let x = (f)
            // editor.graps_objects.iter_mut().for_each
            editor
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
            self.editor
                .lock()
                .unwrap()
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

pub fn create_pane_tree<'a, 'b>(frame: &mut usize) -> egui_tiles::Tree<Pane> {
    let mut tiles = egui_tiles::Tiles::default();

    let mut tabs = vec![];

    let root = tiles.insert_tab_tile(tabs);

    egui_tiles::Tree::new(root, tiles)
}
