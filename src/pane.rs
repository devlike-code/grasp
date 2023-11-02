use egui::Ui;

use crate::tile_manager::TileManager;

pub struct Pane { 
    pub(crate) number: usize, 
}

impl Pane {
    pub fn new(number: usize) -> Pane {
        Pane { number }
    }

    pub fn ui(&mut self, ui: &mut Ui) -> egui_tiles::UiResponse {
        let color = egui::epaint::Hsva::new(0.103 * self.number as f32, 0.5, 0.5, 1.0);
        ui.painter().rect_filled(ui.max_rect(), 0.0, color);

        let response = ui.allocate_rect(ui.max_rect(), egui::Sense::click_and_drag());

        if response.on_hover_cursor(egui::CursorIcon::Grab)
            .dragged_by(egui::PointerButton::Middle) {

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

pub fn create_pane_tree<'a, 'b>(frame: &mut usize, manager: &TileManager) -> egui_tiles::Tree<Pane> {
    let mut gen_pane = || {
        let pane = Pane { number: *frame, };
        *frame += 1;
        pane
    };

    let mut tiles = egui_tiles::Tiles::default();

    let mut tabs = vec![];
    // tabs.push({
    //     let children = (0..7).map(|_| tiles.insert_pane(gen_pane())).collect();
    //     tiles.insert_horizontal_tile(children)
    // });
    // tabs.push({
    //     let cells = (0..11).map(|_| tiles.insert_pane(gen_pane())).collect();
    //     tiles.insert_grid_tile(cells)
    // });
    tabs.push(tiles.insert_pane(gen_pane()));

    let root = tiles.insert_tab_tile(tabs);

    egui_tiles::Tree::new(root, tiles)
}