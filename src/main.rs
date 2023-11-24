use std::{sync::Arc, ops::{Add, Sub}};

use egui::{WidgetText, Ui, Color32, Sense, Vec2, Align2, FontId};
use egui_dock::{DockArea, DockState, Style, TabViewer};
use grasp::create_native_options;
use ::grasp::{internals::{Mosaic, MosaicIO, MosaicTypelevelCRUD, TileFieldGetter, Value, MosaicCRUD, self_val, EntityId}, iterators::{tile_getters::TileGetters, component_selectors::ComponentSelectors, tile_filters::TileFilters}};
use egui::Pos2;
use quadtree_rs::Quadtree;
mod grasp;

pub struct GraspEditorTab {
    pub name: String,
    pub quadtree: Quadtree<i32, EntityId>,
    pub document_mosaic: Arc<Mosaic>,
    pub pan: Vec2,
}

#[derive(Default)]
struct GraspEditorTabs {
    pub current_tab: u32,
}

impl GraspEditorTabs {
    pub fn increment(&mut self) -> u32 {
        self.current_tab += 1;
        self.current_tab
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
        let painter = ui.painter();

        // Rendering

        for node in tab.document_mosaic.get_all().filter_objects().include_component("Position") {
            // Draw node
            let pos = Pos2::new(node.get("x").as_f32(), node.get("y").as_f32());
            painter.circle_filled(pos.add(tab.pan).add(xy.to_vec2()), 10.0, Color32::WHITE);

            // Maybe draw label
            if let Some(label) = node.into_iter().get_descriptors().include_component("Label").next() {
                painter.text(pos.add(Vec2::new(10.0, 10.0)).add(tab.pan).add(xy.to_vec2()), 
                    Align2::LEFT_CENTER, 
                    label.get("self").as_s32().to_string(), 
                    FontId::default(), Color32::WHITE);
            }
        }

        // TODO: render arrows between nodes

        // Sense

        let (resp, _) = ui.allocate_painter(ui.available_size(), Sense::click());
        // TODO: check against quadtree to see whether we're selecting or deselecting
        if resp.double_clicked() {
            let pos = resp.interact_pointer_pos().unwrap().sub(tab.pan).sub(xy.to_vec2());
            let obj = tab.document_mosaic.new_object("Position", vec![ 
                ("x".into(), Value::F32(pos.x)), 
                ("y".into(), Value::F32(pos.y)) 
            ]);
            tab.document_mosaic.new_descriptor(&obj, "Label", self_val(Value::S32("Label!".into())));
            // TODO: insert point into quadtree
        }

        // TODO: create new sense painter to check for drag _if_ there were no clicks, to check for pan/move
    }
}

// Here is a simple example of how you can manage a `DockState` of your application.
struct GraspEditorState {
    document_mosaic: Arc<Mosaic>,
    tabs: GraspEditorTabs,
    dock_state: DockState<GraspEditorTab>
}

impl GraspEditorState {
    pub fn new() -> Self {
        let document_mosaic = Mosaic::new();
        document_mosaic.new_type("Label: s32;").unwrap();
        document_mosaic.new_type("Position: { x: f32, y: f32 };").unwrap();
        document_mosaic.new_type("Selection: unit;").unwrap();
        
        let dock_state = DockState::new(vec![]);
        let mut state = Self { document_mosaic, dock_state, tabs: GraspEditorTabs::default() };

        let tab = state.new_tab();
        state.dock_state.main_surface_mut().push_to_first_leaf(tab);

        state
    }

    pub fn new_tab(&mut self) -> GraspEditorTab {
        GraspEditorTab {
            name: format!("Untitled {}", self.tabs.increment()), 
            quadtree: Quadtree::new(16),
            document_mosaic: Arc::clone(&self.document_mosaic),
            pan: Vec2::ZERO,
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

fn main() -> Result<(), eframe::Error> {
    env_logger::init();
    
    let app_name = "GRASP";
    let native_options = create_native_options();
    
    eframe::run_native(
        app_name,
        native_options,
        Box::new(|_| Box::new(GraspEditorState::new())),
    )
}
