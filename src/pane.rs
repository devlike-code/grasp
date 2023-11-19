use egui::{Ui, Vec2, Rect, Pos2, Sense, Color32, Key};

use crate::{tile_manager::TileManager, grasp_data::GraspMetaData};

#[derive(Debug)]
pub struct Pane { 
    pub(crate) number: usize,
    pub grasp_data : GraspMetaData,
}

impl Pane {
    pub fn new(number: usize) -> Pane {
        Pane { number: number , grasp_data : GraspMetaData::default()}
    }

    pub fn ui(&mut self, ui: &mut Ui) -> egui_tiles::UiResponse {
        let color = egui::epaint::Hsva::new(0.103 * self.number as f32, 0.5, 0.5, 1.0);
        ui.painter().rect_filled(ui.max_rect(), 0.0, color);

        let response = ui.allocate_rect(ui.max_rect(), egui::Sense::click_and_drag());

        let mut is_selection_dragged = false;
        let mut position_delta = Vec2::new(0.0,0.0);
        ui.scope(|ui|{
            let new_button: egui::Response = ui.put(
                Rect {
                    min: Pos2 { x: 300.0, y: 100.0 },
                    max: Pos2 {
                        x: 450.0,
                        y: 150.0,
                    },
                },
                egui::Button::new("Add node").sense(Sense::click()),
            );

            if new_button.clicked(){
                self.grasp_data.add_tile_object();
            }
            
            let mut count = 0;
            self.grasp_data.graps_objects.iter_mut().for_each(|node|{
                
                    let color_button = if node.is_selected{
                        Color32::WHITE
                        } else {
                        Color32::RED
                        };

                    let test_button = ui.put(
                        Rect {
                            min: Pos2 { x: node.position.x, y: node.position.y },
                            max: Pos2 {
                                x: node.position.x + 150.0,
                                y: node.position.y + 100.0,
                            },
                        },
                        egui::Button::new(format!("Square '{}'", count)).sense(Sense::click_and_drag()).fill(color_button),
                    );
                    count += 1;
                    if test_button.clicked_by(egui::PointerButton::Secondary){
                        node.is_selected = !node.is_selected;
                    }
        
                    if ui.ctx().input(|i| i.key_down(Key::ArrowRight)){
                        ui.label("Right arrow pressed");
                        if node.is_selected{
                            node.position = Vec2::new(node.position.x + 10.0, node.position.y);
                        }
                    }
                    
                    if ui.ctx().input(|i| i.key_down(Key::ArrowLeft)){
                        ui.label("Left arrow pressed");
                        if node.is_selected{
                            node.position = Vec2::new(node.position.x - 10.0, node.position.y);
                        }
                    }
        
                    if ui.ctx().input(|i| i.key_down(Key::ArrowUp)){
                        ui.label("Up arrow pressed");
                        if node.is_selected{
                            node.position = Vec2::new(node.position.x, node.position.y - 10.0);
                        }
                    }
        
                    if ui.ctx().input(|i| i.key_down(Key::ArrowDown)){
                        ui.label("Right down pressed");
                        if node.is_selected{
                            node.position = Vec2::new(node.position.x, node.position.y + 10.0);
                        }
                    }
        
                    if test_button.dragged_by(egui::PointerButton::Primary)
                    {
                        if let Some(new_drag_start_position) =
                        test_button.interact_pointer_pos()
                    {
                        is_selection_dragged = true;
                        position_delta = Vec2::new( new_drag_start_position.x - node.position.x, 
                            new_drag_start_position.y - node.position.y);
                    }
                    }

                    if test_button.drag_released_by(egui::PointerButton::Primary)
                    {
                        position_delta.x  = 0.0;
                        position_delta.y = 0.0;
                        is_selection_dragged = false;
                    }
                });
        });

        if is_selection_dragged {
            self.grasp_data.graps_objects.iter_mut().for_each(|node|{
                if node.is_selected{
                    node.position += position_delta;
                }

            });
        }


        if response.dragged_by(egui::PointerButton::Middle) {
            response.on_hover_cursor(egui::CursorIcon::Grab);
            return egui_tiles::UiResponse::DragStarted;
        } else {
            egui_tiles::UiResponse::None
        }
    }
}

impl egui_tiles::Behavior<Pane> for Pane {
    fn tab_title_for_pane(&mut self, pane: &Pane) -> egui::WidgetText {
        format!("Pane {}", pane.number).into()
    }

    fn pane_ui(&mut self, ui: &mut egui::Ui, _tile_id: egui_tiles::TileId, pane: &mut Pane) -> egui_tiles::UiResponse {
        pane.ui(ui)
    }
}

pub fn create_pane_tree<'a, 'b>(frame: &mut usize, _manager: &TileManager) -> egui_tiles::Tree<Pane> {
    let mut gen_pane = || {
        let pane = Pane { number: *frame, grasp_data: GraspMetaData::default()};
        *frame += 1;
        pane
    };

    let mut tiles = egui_tiles::Tiles::default();

    let mut tabs = vec![];
    tabs.push(tiles.insert_pane(gen_pane()));

    let root = tiles.insert_tab_tile(tabs);

    egui_tiles::Tree::new(root, tiles)
}