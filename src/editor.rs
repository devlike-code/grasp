use std::sync::{Arc, Mutex};

use crate::{
    pane::{create_pane_tree, Pane},
    tile_manager::TileManager,
};
use egui::{FontData, FontDefinitions, FontFamily};
use grasp::internals::{Mosaic, MosaicTypelevelCRUD};

pub struct GraspEditor {
    pub frame_count: Mutex<usize>,
    pub tree: egui_tiles::Tree<Pane>,
    pub tile_manager: TileManager,
    pub mosaic: Arc<Mosaic>,
}

pub trait GraspEditorIO {
    fn update_topbar(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame);
    fn update_left_sidebar(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame);
    fn update_editors(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame);
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
        fonts
            .font_data
            .insert("dejavu-sans-condensed".to_owned(), font_data);

        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, "dejavu-sans-condensed".to_owned());

        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .push("dejavu-sans-condensed".to_owned());

        cc.egui_ctx.set_fonts(fonts);
        cc.egui_ctx.set_pixels_per_point(1.0);
    }

    pub fn new(cc: &eframe::CreationContext<'_>) -> Arc<Mutex<Self>> {
        Self::initialize_font(cc);

        let mut frame_count = 0;
        let tile_manager = TileManager::default();
        let tree = create_pane_tree(&mut frame_count);
        let editor = Arc::new(Mutex::new(Self {
            tree,
            tile_manager,
            frame_count: Mutex::new(frame_count),
            mosaic: Mosaic::new(),
        }));

        let binding = Arc::clone(&editor);
        let mut editor_ptr = binding.lock().unwrap();
        editor_ptr
            .mosaic
            .new_type("Position : {x : f32 , y: f32, is_selected: bool};");
        let frame = editor_ptr.next_frame();
        let new_child = editor_ptr.tree.tiles.insert_pane(Pane::new(frame, &editor));

        editor
    }
}

impl GraspEditorIO for Arc<Mutex<GraspEditor>> {
    fn update_topbar(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("topbar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("≡", |ui| {
                    if ui.button("New View").clicked() {
                        let mut editor = self.lock().unwrap();
                        let frame = editor.next_frame();
                        let tree = &mut editor.tree;
                        let new_child = tree.tiles.insert_pane(Pane::new(frame, self));
                        if let Some(root) = tree.root() {
                            match tree.tiles.get_mut(root) {
                                Some(egui_tiles::Tile::Container(egui_tiles::Container::Tabs(
                                    tabs,
                                ))) => tabs.add_child(new_child),

                                Some(egui_tiles::Tile::Container(
                                    egui_tiles::Container::Linear(lin),
                                )) => lin.add_child(new_child),

                                Some(egui_tiles::Tile::Container(egui_tiles::Container::Grid(
                                    grid,
                                ))) => grid.add_child(new_child),

                                _ => {}
                            }
                        }

                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("Exit").clicked() {
                        ui.close_menu();
                        frame.close();
                    }
                });
            });
        });
    }

    fn update_left_sidebar(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("tree")
            .default_width(200.0)
            .resizable(true)
            .show(ctx, |ui| {
                let mut editor = self.lock().unwrap();

                if let Some(parent) = editor.tile_manager.add_child_to.take() {
                    let index = editor.next_frame();
                    let tree = &mut editor.tree;
                    let new_child = tree.tiles.insert_pane(Pane::new(index, self));
                    if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Tabs(tabs))) =
                        tree.tiles.get_mut(parent)
                    {
                        tabs.add_child(new_child);
                        tabs.set_active(new_child);
                    }
                }

                if let Some(parent) = editor.tile_manager.remove_child_from.take() {
                    let tree = &mut editor.tree;
                    if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Tabs(tabs))) =
                        tree.tiles.get_mut(parent)
                    {
                        if let Some(tab) = tabs.active {
                            tree.tiles.remove(tab);
                        }
                    }
                }

                ui.separator();
            });
    }

    fn update_editors(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut editor = self.lock().unwrap();
            if let Some(parent) = editor.tile_manager.add_child_to.take() {
                let frame = editor.next_frame();
                let tree = &mut editor.tree;
                let new_child = tree.tiles.insert_pane(Pane::new(frame, self));
                if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Tabs(tabs))) =
                    tree.tiles.get_mut(parent)
                {
                    tabs.add_child(new_child);
                    tabs.set_active(new_child);
                }
            }
            editor.tree.ui(editor.tile_manager, ui);
        });
    }
}

pub struct GraspEditorCreationContext {
    pub editor: Arc<Mutex<GraspEditor>>,
}

impl GraspEditorCreationContext {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let editor = GraspEditor::new(cc);
        GraspEditorCreationContext { editor: editor }
    }
}

impl eframe::App for GraspEditorCreationContext {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.editor.update_topbar(ctx, frame);
        self.editor.update_left_sidebar(ctx, frame);
        self.editor.update_editors(ctx, frame);
    }
}
