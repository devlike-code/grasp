use std::sync::Arc;

use egui_dock::{DockArea, DockState, Style};
use grasp::internals::{Mosaic, MosaicTypelevelCRUD};
use quadtree_rs::Quadtree;

use crate::{
    editor_state_machine::EditorState,
    grasp::{GraspEditorTab, GraspEditorTabs},
};

pub struct GraspEditorState {
    document_mosaic: Arc<Mosaic>,
    tabs: GraspEditorTabs,
    dock_state: DockState<GraspEditorTab>,
}

impl GraspEditorState {
    pub fn new() -> Self {
        let document_mosaic = Mosaic::new();
        document_mosaic.new_type("Label: s32;").unwrap();
        document_mosaic
            .new_type("Position: { x: f32, y: f32 };")
            .unwrap();
        document_mosaic.new_type("Selection: unit;").unwrap();

        let dock_state = DockState::new(vec![]);
        let mut state = Self {
            document_mosaic,
            dock_state,
            tabs: GraspEditorTabs::default(),
        };

        let tab = state.new_tab();
        state.dock_state.main_surface_mut().push_to_first_leaf(tab);

        state
    }

    pub fn new_tab(&mut self) -> GraspEditorTab {
        GraspEditorTab {
            name: format!("Untitled {}", self.tabs.increment()),
            quadtree: Quadtree::new(16),
            document_mosaic: Arc::clone(&self.document_mosaic),
            node_area: Default::default(),
            editor_data: Default::default(),
            state: EditorState::Idle,
        }
    }

    fn tabs(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        DockArea::new(&mut self.dock_state)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show(ctx, &mut self.tabs);
    }

    fn left_sidebar(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("tree")
            .default_width(200.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.separator();
            });
    }

    fn menu_bar(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("topbar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Document", |ui| {
                    if ui.button("New Tab").clicked() {
                        let tab = self.new_tab();
                        self.dock_state.main_surface_mut().push_to_first_leaf(tab);

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
}

impl eframe::App for GraspEditorState {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.menu_bar(ctx, frame);
        self.left_sidebar(ctx, frame);
        self.tabs(ctx, frame);
    }
}
