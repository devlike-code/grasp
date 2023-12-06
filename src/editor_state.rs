use std::sync::Arc;

use egui::{
    ahash::{HashMap, HashMapExt},
    Align, CollapsingHeader, Color32, Layout, Response, RichText, TextEdit, Ui,
};
use egui_dock::{DockArea, DockState, Style};
use mosaic::{
    internals::{
        tiles, void, Collage, Mosaic, MosaicCRUD, MosaicIO, MosaicTypelevelCRUD, Tile,
        TileFieldQuery, Value, S32,
    },
    iterators::{
        component_selectors::ComponentSelectors, tile_deletion::TileDeletion,
        tile_getters::TileGetters,
    },
};
use quadtree_rs::Quadtree;

use crate::{
    editor_state_machine::{EditorState, EditorStateTrigger, StateMachine},
    grasp_common::{GraspEditorTab, GraspEditorTabs},
};
use mosaic::capabilities::ArchetypeSubject;
use mosaic::capabilities::CollageImportCapability;
use mosaic::capabilities::QueueCapability;

type ComponentRenderer = Box<dyn Fn(&mut Ui, &mut GraspEditorTab, Tile)>;

//#[derive(Debug)]
pub struct GraspEditorState {
    document_mosaic: Arc<Mosaic>,
    component_renderers: HashMap<S32, ComponentRenderer>,
    tabs: GraspEditorTabs,
    dock_state: DockState<GraspEditorTab>,
    editor_state_tile: Tile,
    new_tab_request_queue: Tile,
    refresh_quadtree_queue: Tile,
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
        document_mosaic.new_type("EditorState: unit;").unwrap();
        document_mosaic.new_type("EditorTab: unit;").unwrap();
        document_mosaic.new_type("ToTab: unit;").unwrap();
        document_mosaic
            .new_type("NewTabRequestQueue: unit;")
            .unwrap();
        document_mosaic
            .new_type("RefreshQuadtreeQueue: unit;")
            .unwrap();

        let editor_state_tile = document_mosaic.new_object("EditorState", void());

        let new_tab_request_queue = document_mosaic.make_queue();
        new_tab_request_queue.add_component("NewTabRequestQueue", void());

        let refresh_quadtree_queue = document_mosaic.make_queue();
        refresh_quadtree_queue.add_component("RefreshQuadtreeQueue", void());

        let dock_state = DockState::new(vec![]);

        // add here default renderers
        let mut state = Self {
            document_mosaic,
            component_renderers: HashMap::new(),
            dock_state,
            editor_state_tile,
            new_tab_request_queue,
            refresh_quadtree_queue,
            tabs: GraspEditorTabs::default(),
        };

        state
            .component_renderers
            .insert("Label".into(), Box::new(Self::draw_label_property));

        state
            .component_renderers
            .insert("Position".into(), Box::new(Self::draw_position_property));

        let tab = state.new_tab(tiles());
        state.dock_state.main_surface_mut().push_to_first_leaf(tab);

        state
    }

    pub fn new_tab(&mut self, collage: Box<Collage>) -> GraspEditorTab {
        let tab_tile = self.document_mosaic.make_queue();
        tab_tile.add_component("EditorTab", void());

        self.document_mosaic
            .new_arrow(&self.editor_state_tile, &tab_tile, "ToTab", void());

        GraspEditorTab {
            name: format!("Untitled {}", self.tabs.increment()),
            tab_tile,
            quadtree: Quadtree::new_with_anchor((-1000, -1000).into(), 16),
            document_mosaic: Arc::clone(&self.document_mosaic),
            collage,
            node_to_area: Default::default(),
            editor_data: Default::default(),
            state: EditorState::Idle,
            grid_visible: false,
            ruler_visible: false,
            response: None,
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
                    let selected = tab.editor_data.selected.clone();
                    for t in selected {
                        CollapsingHeader::new(RichText::from(format!(
                            "ID:{} {}",
                            t.id, "PROPERTIES"
                        )))
                        .default_open(true)
                        .show(ui, |ui| {
                            if t.match_archetype(&["Position", "Label"]) {
                                let values = t.get_archetype(&["Position", "Label"]);
                                let pos = values.get("Position").unwrap().clone();
                                let lab = values.get("Label").unwrap().clone();

                                if let Some(renderer) =
                                    self.component_renderers.get(&"Position".into())
                                {
                                    renderer(ui, tab, pos);
                                }

                                if let Some(renderer) =
                                    self.component_renderers.get(&"Label".into())
                                {
                                    renderer(ui, tab, lab);
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
                let tab = self.new_tab(tiles());
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

    fn draw_label_property(ui: &mut Ui, tab: &mut GraspEditorTab, d: Tile) {
        println!("DRAW LABEL STATE: {:?}", tab.state);
        if let Some(label) = d.get_component("Label") {
            let mut label_text = label.get("self").as_s32().to_string();

            CollapsingHeader::new(RichText::from(format!("ID:{} {}", d.id, "LABEL")))
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if tab.state == EditorState::Rename
                            && tab.editor_data.renaming == Some(d.id)
                        {
                            let mut label = tab.editor_data.text.clone();

                            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                ui.text_edit_singleline(&mut label);
                            });

                            tab.editor_data.text = label;

                            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                tab.trigger(EditorStateTrigger::EndDrag);
                            } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                tab.editor_data.text = tab.editor_data.previous_text.clone();
                                tab.trigger(EditorStateTrigger::EndDrag);
                            }
                        } else {
                            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                let l_response = ui.text_edit_singleline(&mut label_text);

                                if l_response.has_focus() {
                                    tab.editor_data.renaming = Some(d.id);

                                    tab.editor_data.text = label_text;
                                    tab.editor_data.previous_text = tab.editor_data.text.clone();

                                    tab.trigger(EditorStateTrigger::DblClickToRename);
                                }
                            });
                        }
                    });
                });
        }
    }

    fn draw_position_property(ui: &mut Ui, tab: &mut GraspEditorTab, d: Tile) {
        println!("DRAW POSITION STATE: {:?}", tab.state);
        if let (Value::F32(x), Value::F32(y)) = d.get_by(("x", "y")) {
            let mut x_text = format!("{}", x);
            let mut y_text = format!("{}", y);
            CollapsingHeader::new(RichText::from(format!("ID:{} {}", d.id, "POSITION")))
                .default_open(true)
                .show(ui, |ui| {
                    //    ui.with_layout(Layout::left_to_right(Align::Center).with_cross_justify(true), |ui|{
                    ui.horizontal(|ui| {
                        if tab.state == EditorState::Reposition
                            && tab.editor_data.repositioning == Some(d.id)
                        {
                            let mut x = tab.editor_data.x_pos.clone();
                            let mut y = tab.editor_data.y_pos.clone();

                            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                egui::Grid::new("pos_size").show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label("x:");
                                        ui.add(TextEdit::singleline(&mut x));
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("y:");
                                        ui.add(TextEdit::singleline(&mut y));
                                    });
                                    ui.end_row();
                                });
                            });

                            if let Ok(x_parsed) = x.parse::<f32>() {
                                if !x.ends_with('.') {
                                    tab.editor_data.x_pos = x_parsed.to_string();
                                } else {
                                    tab.editor_data.x_pos = x.clone();
                                }
                            }

                            if let Ok(y_parsed) = y.parse::<f32>() {
                                if !y.ends_with('.') {
                                    tab.editor_data.y_pos = y_parsed.to_string();
                                } else {
                                    tab.editor_data.y_pos = y.clone();
                                }
                            }

                            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                tab.trigger(EditorStateTrigger::EndDrag);
                            } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                tab.editor_data.repositioning = None;
                                tab.trigger(EditorStateTrigger::EndDrag);
                            }
                        } else {
                            ui.with_layout(
                                Layout::left_to_right(Align::Center)
                                    .with_cross_align(Align::Center),
                                |ui| {
                                    egui::Grid::new("pos_size").show(ui, |ui| {
                                        let mut x_response: Option<Response> = None;
                                        let mut y_response: Option<Response> = None;

                                        ui.horizontal(|ui| {
                                            ui.label("x:");
                                            x_response =
                                                Some(ui.add(TextEdit::singleline(&mut x_text)));
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("y:");
                                            y_response =
                                                Some(ui.add(TextEdit::singleline(&mut y_text)));
                                        });
                                        ui.end_row();

                                        if x_response.unwrap().has_focus()
                                            || y_response.unwrap().has_focus()
                                        {
                                            tab.editor_data.repositioning = Some(d.id);

                                            tab.editor_data.x_pos = x.to_string();
                                            tab.editor_data.y_pos = y.to_string();
                                            tab.editor_data.previous_x_pos =
                                                tab.editor_data.x_pos.clone();
                                            tab.editor_data.previous_x_pos =
                                                tab.editor_data.y_pos.clone();

                                            tab.trigger(EditorStateTrigger::ClickToReposition);
                                        }
                                    });
                                },
                            );
                        }
                    });
                });
        }
    }
}

impl eframe::App for GraspEditorState {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());

        self.menu_bar(ctx, frame);
        self.left_sidebar(ctx, frame);
        self.right_sidebar(ctx, frame);

        while let Some(request) = self.document_mosaic.dequeue(&self.new_tab_request_queue) {
            if let Some(collage) = request.to_collage() {
                let tab = self.new_tab(collage);
                self.dock_state.main_surface_mut().push_to_first_leaf(tab);
            }
        }

        while let Some(request) = self.document_mosaic.dequeue(&self.refresh_quadtree_queue) {
            for tab in self
                .editor_state_tile
                .iter()
                .get_arrows_from()
                .include_component("ToTab")
                .get_targets()
            {
                self.document_mosaic
                    .enqueue(&tab, &self.document_mosaic.new_object("void", void()));

                request.iter().delete();
            }
        }

        self.show_tabs(ctx, frame);
    }
}
