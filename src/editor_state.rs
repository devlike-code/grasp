use std::{fmt::format, sync::Arc};

use egui::{
    ahash::{HashMap, HashMapExt},
    CollapsingHeader, Color32, RichText, TextEdit, Ui,
};
use egui_dock::{DockArea, DockState, Style};
use mosaic::{
    internals::{Mosaic, MosaicTypelevelCRUD, Tile, TileFieldQuery, Value, S32},
    iterators::tile_getters::TileGetters,
};
use quadtree_rs::Quadtree;

use crate::{
    editor_state_machine::EditorState,
    grasp_common::{GraspEditorTab, GraspEditorTabs},
};

type ComponentRenderer = Box<dyn Fn(&mut Ui, &Tile)>;
//#[derive(Debug)]
pub struct GraspEditorState {
    document_mosaic: Arc<Mosaic>,
    component_renderers: HashMap<S32, ComponentRenderer>,
    tabs: GraspEditorTabs,
    dock_state: DockState<GraspEditorTab>,
}

impl GraspEditorState {
    pub fn new() -> Self {
        let document_mosaic = Mosaic::new();
        document_mosaic.new_type("Arrow: unit;").unwrap();
        document_mosaic.new_type("Label: s32;").unwrap();
        document_mosaic
            .new_type("Position: { x: f32, y: f32 };")
            .unwrap();
        document_mosaic.new_type("Selection: unit;").unwrap();

        let dock_state = DockState::new(vec![]);

        // add here default renderers
        let mut state = Self {
            document_mosaic,
            component_renderers: HashMap::new(),
            dock_state,
            tabs: GraspEditorTabs::default(),
        };

        state
            .component_renderers
            .insert("Label".into(), Box::new(Self::draw_label_property));

        state
            .component_renderers
            .insert("Position".into(), Box::new(Self::draw_position_property));

        let tab = state.new_tab();
        state.dock_state.main_surface_mut().push_to_first_leaf(tab);

        state
    }

    pub fn new_tab(&mut self) -> GraspEditorTab {
        GraspEditorTab {
            name: format!("Untitled {}", self.tabs.increment()),
            quadtree: Quadtree::new_with_anchor((-1000, -1000).into(), 16),
            document_mosaic: Arc::clone(&self.document_mosaic),
            node_to_area: Default::default(),
            editor_data: Default::default(),
            state: EditorState::Idle,
            grid_visible: false,
            ruler_visible: false,
        }
    }

    fn show_tabs(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
    fn right_sidebar(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::right("properties")
            .default_width(250.0)
            .resizable(true)
            .show(ctx, |ui| {
                if let Some((_, tab)) = self.dock_state.find_active_focused() {
                    let selected = &tab.editor_data.selected;
                    for t in selected {
                        CollapsingHeader::new(RichText::from(format!(
                            "ID:{} {}",
                            t.id, "PROPERTIES"
                        )))
                        .default_open(true)
                        .show(ui, |ui| {
                            for d in t.iter().get_descriptors() {
                                if d.component.to_string() == "Label" {
                                    if let Some(renderer) =
                                        self.component_renderers.get(&d.component)
                                    {
                                        renderer(ui, &d);
                                    }
                                }
                            }

                            if t.component.to_string() == "Position" {
                                if let Some(renderer) = self.component_renderers.get(&t.component) {
                                    renderer(ui, t);
                                }
                            }
                        });
                    }
                }
                ui.separator();
            });
    }

    fn menu_bar(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("topbar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                self.show_document(ui, frame);
                self.show_view(ui);
            });
        });
    }

    fn show_document(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
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
    }

    fn show_view(&mut self, ui: &mut Ui) {
        ui.menu_button("View", |ui| {
            let ruler_on = {
                let mut checked = "";
                if let Some((_, tab)) = self.dock_state.find_active_focused() {
                    if tab.ruler_visible {
                        checked = "✔";
                    }
                }
                checked
            };
            if ui.button(format!("Toggle ruler {}", ruler_on)).clicked() {
                if let Some((_, tab)) = self.dock_state.find_active_focused() {
                    tab.ruler_visible = !tab.ruler_visible;
                    ui.close_menu();
                }
            }

            let grid_on = {
                let mut checked = "";
                if let Some((_, tab)) = self.dock_state.find_active_focused() {
                    if tab.grid_visible {
                        checked = "✔";
                    }
                }
                checked
            };
            if ui.button(format!("Toggle grid {}", grid_on)).clicked() {
                if let Some((_, tab)) = self.dock_state.find_active_focused() {
                    tab.grid_visible = !tab.grid_visible;
                    ui.close_menu();
                }
            }
        });
    }

    fn draw_label_property(ui: &mut Ui, d: &Tile) {
        ui.heading(
            RichText::from(format!(
                "{} --> {:?}",
                d.component.to_string(),
                d.get("self")
            ))
            .italics()
            .size(15.0)
            .color(Color32::LIGHT_YELLOW),
        );

        // Add more widgets as needed.
    }

    fn draw_position_property(ui: &mut Ui, d: &Tile) {
        if let (Value::F32(x), Value::F32(y)) = d.query(("x", "y")) {
            let text = RichText::from(format!(
                "{} : ({:.2}, {:.2})",
                d.component.to_string(),
                x,
                y
            ))
            .size(15.0)
            .color(Color32::LIGHT_YELLOW);
            ui.heading(text);
        }
        // if ui.heading(text).double_clicked() {
        //     let text_edit = TextEdit::singleline(&mut self.editor_data.text)
        //         .char_limit(30)
        //         .cursor_at_end(true);
        //     let text_edit_response = ui.put(
        //         Rect::from_two_pos(
        //             floating_pos.add(Vec2::new(0.0, -5.0)),
        //             floating_pos.add(Vec2::new(60.0, 20.0)),
        //         ),
        //         text_edit,
        //     );
        // if let Some((_, tab)) = self.dock_state.find_active_focused() {
        //     tab.editor_data.renaming = Some(d.id);
        //     tab.editor_data.selected = vec![d];
        //     tab.editor_data.text = label.to_string();
        //     tab.editor_data.previous_text = label.to_string();
        // }

        // self.trigger(DblClickToRename);
        //}
        //}
    }
}

impl eframe::App for GraspEditorState {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());
        self.menu_bar(ctx, frame);
        self.left_sidebar(ctx, frame);
        self.right_sidebar(ctx, frame);
        self.show_tabs(ctx, frame);
    }
}
