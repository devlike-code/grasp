use std::{env, fmt::Display, fs, str::FromStr, sync::Arc};

use egui::{
    ahash::{HashMap, HashMapExt},
    Align, Align2, CollapsingHeader, Context, Label, Layout, Pos2, RichText, Sense, TextEdit, Ui,
};

use egui_dock::{DockArea, DockState, Style};
use egui_extras::Size;
use egui_grid::GridBuilder;
use egui_toast::{Toast, ToastKind, ToastOptions, Toasts};
use itertools::Itertools;
use mosaic::{
    capabilities::QueueTile,
    internals::{
        par, tiles, void, Collage, Datatype, FromByteArray, Mosaic, MosaicCRUD, MosaicIO,
        MosaicTypelevelCRUD, Tile, TileFieldSetter, ToByteArray, Value, S32,
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
    grasp_transitions::QuadtreeUpdateCapability,
};
use mosaic::capabilities::ArchetypeSubject;
use mosaic::capabilities::CollageImportCapability;
use mosaic::capabilities::QueueCapability;

type ComponentRenderer = Box<dyn Fn(&mut Ui, &mut GraspEditorTab, Tile)>;

pub trait ToastCapability {
    fn send_toast(&self, text: &str);
}

impl ToastCapability for Arc<Mosaic> {
    fn send_toast(&self, text: &str) {
        if text.len() >= 32 {
            println!(
                "ERROR: Toast message must be shorter than 32 bytes, in:\n{}",
                text
            );
        } else {
            let queue = self
                .get_all()
                .include_component("ToastRequestQueue")
                .get_targets()
                .next()
                .unwrap();
            let request = self.new_object("ToastRequest", par(text));
            self.enqueue(&queue, &request);
            println!("ToastRequest enqueued");
        }
    }
}

pub struct GraspEditorState {
    document_mosaic: Arc<Mosaic>,
    component_renderers: HashMap<S32, ComponentRenderer>,
    tabs: GraspEditorTabs,
    dock_state: DockState<GraspEditorTab>,
    toasts: Toasts,
    editor_state_tile: Tile,
    new_tab_request_queue: QueueTile,
    refresh_quadtree_queue: QueueTile,
    toast_request_queue: QueueTile,
}

impl GraspEditorState {
    pub fn prepare_mosaic(mosaic: Arc<Mosaic>) -> Arc<Mosaic> {
        mosaic.new_type("Arrow: unit;").unwrap();
        mosaic.new_type("Label: s32;").unwrap();
        mosaic.new_type("Position: { x: f32, y: f32 };").unwrap();
        mosaic.new_type("Selection: unit;").unwrap();
        mosaic.new_type("EditorState: unit;").unwrap();
        mosaic.new_type("EditorTab: unit;").unwrap();
        mosaic.new_type("ToTab: unit;").unwrap();
        mosaic.new_type("NewTabRequestQueue: unit;").unwrap();
        mosaic.new_type("RefreshQuadtreeQueue: unit;").unwrap();
        mosaic.new_type("ToastRequestQueue: unit;").unwrap();
        mosaic.new_type("ToastRequest: s32;").unwrap();
        println!("Mosaic ready for use in Grasp!");
        mosaic
    }

    pub fn new() -> Self {
        let document_mosaic = Mosaic::new();
        Self::prepare_mosaic(Arc::clone(&document_mosaic));

        let editor_state_tile = document_mosaic.new_object("EditorState", void());

        let new_tab_request_queue = document_mosaic.make_queue();
        new_tab_request_queue.add_component("NewTabRequestQueue", void());

        let refresh_quadtree_queue = document_mosaic.make_queue();
        refresh_quadtree_queue.add_component("RefreshQuadtreeQueue", void());

        let toast_request_queue = document_mosaic.make_queue();
        toast_request_queue.add_component("ToastRequestQueue", void());

        let toasts = Toasts::new().anchor(Align2::RIGHT_TOP, Pos2::new(-10.0, 10.0));
        let dock_state = DockState::new(vec![]);

        // add here default renderers
        let mut state = Self {
            document_mosaic,
            component_renderers: HashMap::new(),
            dock_state,
            toasts,
            editor_state_tile,
            new_tab_request_queue,
            refresh_quadtree_queue,
            toast_request_queue,
            tabs: GraspEditorTabs::default(),
        };

        // state
        //     .component_renderers
        //     .insert("Label".into(), Box::new(Self::draw_label_property));

        // state
        //     .component_renderers
        //     .insert("Position".into(), Box::new(Self::draw_position_property));

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
            object_to_area: Default::default(),
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
                            "[ID:{}] {}",
                            t.id, "PROPERTIES"
                        )))
                        .default_open(true)
                        .show(ui, |ui| {
                            for (part, tiles) in
                                &t.get_full_archetype().into_iter().sorted().collect_vec()
                            {
                                let mut draw_separator = tiles.len() - 1;
                                for tile in tiles.iter().sorted() {
                                    if let Some(renderer) =
                                        self.component_renderers.get(&part.as_str().into())
                                    {
                                        CollapsingHeader::new(RichText::from(format!(
                                            "[ID: {}] {}",
                                            tile.id,
                                            part.to_uppercase()
                                        )))
                                        .default_open(true)
                                        .show(ui, |ui| {
                                            renderer(ui, tab, tile.clone());
                                        });
                                    } else {
                                        CollapsingHeader::new(RichText::from(format!(
                                            "[ID: {}] {}",
                                            tile.id,
                                            part.to_uppercase()
                                        )))
                                        .default_open(true)
                                        .show(ui, |ui| {
                                            draw_default_renderer(ui, tab, tile.clone());
                                        });
                                    }

                                    if draw_separator > 0 {
                                        ui.separator();
                                        draw_separator -= 1;
                                    }
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

    fn show_document(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        ui.menu_button("Document", |ui| {
            if ui.button("New Tab").clicked() {
                let tab = self.new_tab(tiles());
                self.dock_state.main_surface_mut().push_to_first_leaf(tab);

                ui.close_menu();
            }

            ui.separator();

            if ui.button("Open").clicked() {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("Mosaic", &["mos"])
                    .set_directory(env::current_dir().unwrap())
                    .pick_file()
                {
                    self.document_mosaic.clear();
                    Self::prepare_mosaic(Arc::clone(&self.document_mosaic));

                    self.document_mosaic.load(&fs::read(file).unwrap()).unwrap();
                    self.document_mosaic.send_toast("Document loaded");

                    self.document_mosaic.request_quadtree_update();
                }
                ui.close_menu();
            }

            if ui.button("Save").clicked() {
                let document = self.document_mosaic.save();
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("Mosaic", &["mos"])
                    .set_directory(env::current_dir().unwrap())
                    .save_file()
                {
                    fs::write(file, document).unwrap();
                    self.document_mosaic.send_toast("Document saved");
                }
                ui.close_menu();
            }

            ui.separator();

            if ui.button("Exit").clicked() {
                ui.close_menu();
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
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
            if ui.button(format!("Toggle Ruler {}", ruler_on)).clicked() {
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
            if ui.button(format!("Toggle Grid {}", grid_on)).clicked() {
                if let Some((_, tab)) = self.dock_state.find_active_focused() {
                    tab.grid_visible = !tab.grid_visible;
                    ui.close_menu();
                }
            }
        });
    }
}

fn draw_default_renderer(ui: &mut Ui, tab: &mut GraspEditorTab, d: Tile) {
    let mosaic = &tab.document_mosaic;
    let comp = mosaic
        .component_registry
        .get_component_type(d.component)
        .unwrap();
    let fields = comp.get_fields();

    ui.vertical(|ui| {
        let mut grid_builder = GridBuilder::new();
        for _i in 0..fields.len() {
            grid_builder = grid_builder
                .new_row(Size::initial(18.0))
                .cell(Size::exact(60.0))
                .cell(Size::remainder().at_least(120.0));
        }

        grid_builder.show(ui, |mut grid| {
            for field in &fields {
                let name = if comp.is_alias() {
                    "self".to_string()
                } else {
                    let name = field.name;
                    name.to_string()
                };

                let datatype = field.datatype.clone();

                if datatype == Datatype::UNIT {
                    continue;
                }

                let value = d.get(name.as_str());

                {
                    grid.cell(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(name.clone());
                        });
                    });
                }

                grid.cell(|ui| {
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| match value {
                        Value::UNIT => {}
                        Value::I8(v) => draw_property_value(ui, tab, &d, name.as_str(), v),
                        Value::I16(v) => draw_property_value(ui, tab, &d, name.as_str(), v),
                        Value::I32(v) => draw_property_value(ui, tab, &d, name.as_str(), v),
                        Value::I64(v) => draw_property_value(ui, tab, &d, name.as_str(), v),
                        Value::U8(v) => draw_property_value(ui, tab, &d, name.as_str(), v),
                        Value::U16(v) => draw_property_value(ui, tab, &d, name.as_str(), v),
                        Value::U32(v) => draw_property_value(ui, tab, &d, name.as_str(), v),
                        Value::U64(v) => draw_property_value(ui, tab, &d, name.as_str(), v),
                        Value::F32(v) => draw_property_value(ui, tab, &d, name.as_str(), v),
                        Value::F64(v) => draw_property_value(ui, tab, &d, name.as_str(), v),
                        Value::S32(v) => {
                            draw_property_value(ui, tab, &d, name.as_str(), v.to_string())
                        }
                        Value::S128(v) => draw_property_value(
                            ui,
                            tab,
                            &d,
                            name.as_str(),
                            String::from_byte_array(&v),
                        ),

                        Value::BOOL(v) => {
                            let mut b = v;
                            ui.checkbox(&mut b, "");
                        }
                    });
                });
            }
        })
    });
}

fn draw_property_value<T: Display + FromStr + ToByteArray>(
    ui: &mut Ui,
    tab: &mut GraspEditorTab,
    tile: &Tile,
    name: &str,
    t: T,
) where
    Tile: TileFieldSetter<T>,
{
    let changing: bool = tab.state == EditorState::PropertyChanging && {
        match (
            tab.editor_data.tile_changing,
            &tab.editor_data.field_changing,
        ) {
            (Some(tile_id), Some(field_name)) => tile_id == tile.id && field_name.as_str() == name,
            _ => false,
        }
    };

    if !changing {
        let text = format!("{}", t);
        let label = Label::new(text.clone()).wrap(true).sense(Sense::click());

        if ui.add(label).double_clicked() {
            tab.editor_data.tile_changing = Some(tile.id);
            tab.editor_data.field_changing = Some(name.to_string());
            tab.editor_data.previous_text = text.clone();
            tab.editor_data.text = text;
            tab.trigger(EditorStateTrigger::DblClickToRename);
        }
    } else {
        let mut text = tab.editor_data.text.clone();
        let datatype = tile.get(name).get_datatype();

        let widget = match datatype {
            Datatype::S32 => {
                TextEdit::singleline(&mut text)
                    .char_limit(32)
                    .show(ui)
                    .response
            }

            Datatype::S128 => {
                TextEdit::multiline(&mut text)
                    .char_limit(128)
                    .show(ui)
                    .response
            }

            _ => ui.text_edit_singleline(&mut text),
        };

        if widget.changed() {
            tab.editor_data.text = text.clone();
        }

        if widget.lost_focus() {
            let mut tile = tile.clone();
            if let Ok(parsed) = text.parse::<T>() {
                tile.set(name, parsed);
                tile.mosaic.request_quadtree_update();
            }

            tab.trigger(EditorStateTrigger::EndDrag);
        }
    }
}

impl eframe::App for GraspEditorState {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());
        self.menu_bar(ctx, frame);
        self.left_sidebar(ctx, frame);
        self.right_sidebar(ctx, frame);

        while let Some(request) = self.document_mosaic.dequeue(&self.toast_request_queue) {
            let toast_message = request.get("self").as_s32();
            self.toasts.add(Toast {
                text: toast_message.to_string().into(),
                kind: ToastKind::Info,
                options: ToastOptions::default()
                    .duration_in_seconds(5.0)
                    .show_icon(false)
                    .show_progress(true),
            });

            request.iter().delete();
        }

        while let Some(request) = self.document_mosaic.dequeue(&self.new_tab_request_queue) {
            if let Some(collage) = request.to_collage() {
                let tab = self.new_tab(collage);
                self.dock_state.main_surface_mut().push_to_first_leaf(tab);

                request.iter().delete();
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
        self.toasts.show(ctx);
    }
}
