use std::sync::Mutex;

use egui::{FontDefinitions, FontData, FontFamily};
use egui_tiles::TileId;

use crate::{pane::{Pane, create_pane_tree}, tile_manager::{TileManager, tile_manager_tree_ui}};

#[derive(Default)]
pub struct GraspEditor {
    pub(crate) frame_count: Mutex<usize>,
    pub(crate) tree: egui_tiles::Tree<Pane>,
    pub(crate) tile_manager: TileManager,
}

impl GraspEditor { 
    pub fn next_frame(&self) -> usize {
        let mut count = self.frame_count.lock().unwrap();
        *count += 1;
        *count
    }

    fn initialize_font(cc: &eframe::CreationContext<'_>) {
        let mut fonts = FontDefinitions::default();

        let font_data = FontData::from_static(include_bytes!("../fonts/DejaVuSansCondensed.ttf"));
        fonts.font_data.insert("dejavu-sans-condensed".to_owned(), font_data);

        fonts.families.get_mut(&FontFamily::Proportional).unwrap()
            .insert(0, "dejavu-sans-condensed".to_owned());

        fonts.families.get_mut(&FontFamily::Monospace).unwrap()
            .push("dejavu-sans-condensed".to_owned());

        cc.egui_ctx.set_fonts(fonts);
        cc.egui_ctx.set_pixels_per_point(1.0);
    }

    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::initialize_font(cc);
        
        let mut frame_count = 0;
        let tile_manager = TileManager::default();
        let tree = create_pane_tree(&mut frame_count, &tile_manager);
        Self { tree, tile_manager, frame_count: Mutex::new(frame_count) }
    }

    pub fn update_topbar(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("topbar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("≡", |ui| {
                    if ui.button("Exit").clicked() {
                        frame.close();
                    }
                });
            });
        });
    }

    pub fn update_left_sidebar(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::SidePanel::left("tree").show(ctx, |ui| {
            if ui.button("Reset").clicked() {
                *self = Default::default();
            }
            self.tile_manager.ui(ui);
            ui.separator();

            if let Some(root) = self.tree.root() {
                tile_manager_tree_ui(ui, &mut self.tile_manager, &mut self.tree.tiles, root);
            }

            if let Some(parent) = self.tile_manager.add_child_to.take() {
                let index = self.next_frame();
                let new_child = self.tree.tiles.insert_pane(Pane::new(index));
                if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Tabs(tabs))) =
                    self.tree.tiles.get_mut(parent)
                {
                    tabs.add_child(new_child);
                    tabs.set_active(new_child);
                }
            }

            if let Some(parent) = self.tile_manager.remove_child_from.take() {
                
                if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Tabs(tabs))) = self.tree.tiles.get_mut(parent) {
                    if let Some(tab) = tabs.active {
                        self.tree.tiles.remove(tab);
                    }
                }
            }
        });
    }

    pub fn update_editors(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(parent) = self.tile_manager.add_child_to.take() {
                let mut frame = self.frame_count.lock().unwrap();
                let new_child = self.tree.tiles.insert_pane(Pane::new(*frame));
                *frame += 1;
                if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Tabs(tabs))) =
                    self.tree.tiles.get_mut(parent)
                {
                    tabs.add_child(new_child);
                    tabs.set_active(new_child);
                }
            }

            self.tree.ui(&mut self.tile_manager, ui);
        });
    }

}

impl eframe::App for GraspEditor {
   fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.update_topbar(ctx, frame);
        self.update_left_sidebar(ctx, frame);
        self.update_editors(ctx, frame);
   }
}