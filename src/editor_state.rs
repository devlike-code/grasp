use std::{
    collections::HashMap,
    env, fs,
    sync::{Arc, Mutex},
};

use imgui::{Condition, ImString, TreeNodeFlags, Ui};
use itertools::Itertools;
use mosaic::{
    capabilities::QueueTile,
    internals::{
        all_tiles, par, void, Collage, Mosaic, MosaicCRUD, MosaicIO, MosaicTypelevelCRUD, Tile, S32,
    },
    iterators::component_selectors::ComponentSelectors,
};
use quadtree_rs::Quadtree;

use crate::{
    core::gui::docking::GuiViewport, editor_state_machine::EditorState,
    grasp_editor_window::GraspEditorWindow, grasp_editor_window_list::GraspEditorWindowList,
    grasp_transitions::QuadtreeUpdateCapability, GuiState,
};
use mosaic::capabilities::ArchetypeSubject;
use mosaic::capabilities::QueueCapability;

type ComponentRenderer = Box<dyn Fn(&mut Ui, &mut GraspEditorWindow, Tile)>;

// pub trait ToastCapability {
//     fn send_toast(&self, text: &str);
// }

// impl ToastCapability for Arc<Mosaic> {
//     fn send_toast(&self, text: &str) {
//         if text.len() >= 32 {
//             println!(
//                 "ERROR: Toast message must be shorter than 32 bytes, in:\n{}",
//                 text
//             );
//         } else {
//             let queue = self
//                 .get_all()
//                 .include_component("ToastRequestQueue")
//                 .get_targets()
//                 .next()
//                 .unwrap();
//             let request = self.new_object("ToastRequest", par(text));
//             self.enqueue(&queue, &request);
//             println!("ToastRequest enqueued");
//         }
//     }
// }

pub struct GraspEditorState {
    pub(crate) document_mosaic: Arc<Mosaic>,
    pub(crate) component_renderers: HashMap<S32, ComponentRenderer>,
    pub(crate) window_list: GraspEditorWindowList,
    pub(crate) editor_state_tile: Tile,
    pub(crate) new_tab_request_queue: QueueTile,
    pub(crate) refresh_quadtree_queue: QueueTile,
    pub(crate) toast_request_queue: QueueTile,
    show_tabview: bool,
}

impl GraspEditorState {
    pub fn snapshot(&self) {
        let content = self.document_mosaic.dot();
        open::that(format!(
            "https://dreampuf.github.io/GraphvizOnline/#{}",
            urlencoding::encode(content.as_str())
        ))
        .unwrap();
    }

    pub fn prepare_mosaic(mosaic: Arc<Mosaic>) -> Arc<Mosaic> {
        mosaic.new_type("Arrow: unit;").unwrap();
        mosaic.new_type("Label: s32;").unwrap();
        mosaic.new_type("Position: { x: f32, y: f32 };").unwrap();
        mosaic.new_type("Selection: unit;").unwrap();
        mosaic.new_type("EditorState: unit;").unwrap();
        mosaic.new_type("EditorStateFocusedWindow: u64;").unwrap();
        mosaic.new_type("EditorTab: unit;").unwrap();
        mosaic.new_type("ToTab: unit;").unwrap();
        mosaic.new_type("NewWindowRequestQueue: unit;").unwrap();
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
        document_mosaic.new_extension(&editor_state_tile, "EditorStateFocusedWindow", par(0u64));

        let new_window_request_queue = document_mosaic.make_queue();
        new_window_request_queue.add_component("NewWindowRequestQueue", void());

        let refresh_quadtree_queue = document_mosaic.make_queue();
        refresh_quadtree_queue.add_component("RefreshQuadtreeQueue", void());

        let toast_request_queue = document_mosaic.make_queue();
        toast_request_queue.add_component("ToastRequestQueue", void());

        //let dock_state = DockState::new(vec![]);

        // add here default renderers
        let state = Self {
            document_mosaic,
            component_renderers: HashMap::new(),
            editor_state_tile,
            new_tab_request_queue: new_window_request_queue,
            refresh_quadtree_queue,
            toast_request_queue,
            window_list: GraspEditorWindowList::default(),
            show_tabview: false,
        };

        // state
        //     .component_renderers
        //     .insert("Label".into(), Box::new(Self::draw_label_property));

        // state
        //     .component_renderers
        //     .insert("Position".into(), Box::new(Self::draw_position_property));

        //        let tab = state.new_tab(all_tiles());
        //        state.dock_state.main_surface_mut().push_to_first_leaf(tab);

        state
    }

    pub fn new_window(&mut self, collage: Box<Collage>) {
        let window_tile = self.document_mosaic.make_queue();
        window_tile.add_component("EditorTab", void());

        self.document_mosaic
            .new_arrow(&self.editor_state_tile, &window_tile, "ToTab", void());

        let new_index = self.window_list.increment();
        self.window_list.windows.push(GraspEditorWindow {
            name: format!("Untitled {}", new_index),
            window_tile,
            quadtree: Mutex::new(Quadtree::new_with_anchor((-1000, -1000).into(), 16)),
            document_mosaic: Arc::clone(&self.document_mosaic),
            collage,
            object_to_area: Default::default(),
            editor_data: Default::default(),
            state: EditorState::Idle,
            grid_visible: false,
            ruler_visible: false,
        });
    }

    pub fn show(&mut self, s: &GuiState) {
        self.show_left_sidebar(s);
        self.show_right_sidebar(s);
        self.show_menu_bar(s);
        self.window_list.show(s);
    }

    fn show_left_sidebar(&mut self, s: &GuiState) {
        let viewport = GuiViewport::get_main_viewport();
        if let Some(w) =
            s.ui.window(ImString::new("Hierarchy"))
                .position([0.0, 18.0], Condition::FirstUseEver)
                .size(
                    [viewport.size().x, viewport.size().y - 18.0],
                    Condition::FirstUseEver,
                )
                .begin()
        {
            if s.ui
                .collapsing_header("Windows", TreeNodeFlags::DEFAULT_OPEN)
            {
                if s.ui.button("[+] New Window") {
                    self.new_window(all_tiles());
                }

                let focus = self
                    .document_mosaic
                    .get_all()
                    .include_component("EditorStateFocusedWindow")
                    .next()
                    .unwrap();

                let focused_index = focus.get("self").as_u64() as usize;
                let mut i = self
                    .window_list
                    .windows
                    .iter()
                    .position(|w| w.window_tile.id == focused_index)
                    .unwrap_or_default() as i32;

                let items = self
                    .window_list
                    .windows
                    .iter()
                    .map(|w| w.name.as_str())
                    .collect_vec();

                s.ui.set_next_item_width(-1.0);
                if s.ui.list_box("##", &mut i, items.as_slice(), 20) {
                    let item: &str = items.get(i as usize).unwrap();
                    self.window_list.focus(item);
                }
            }

            w.end();
        }
    }

    fn show_right_sidebar(&mut self, s: &GuiState) {
        let viewport = GuiViewport::get_main_viewport();
        if let Some(w) =
            s.ui.window(ImString::new("Properties"))
                .position([viewport.size().x - 300.0, 18.0], Condition::FirstUseEver)
                .size([300.0, viewport.size().y - 18.0], Condition::FirstUseEver)
                .begin()
        {
            w.end();
        }
    }

    /*
       fn right_sidebar(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
           egui::SidePanel::right("properties")
               .default_width(250.0)
               .resizable(true)
               .show(ctx, |ui| {
                   if let Some((_, tab)) = self.dock_state.find_active_focused() {
                       let selected = tab
                           .editor_data
                           .selected
                           .clone()
                           .into_iter()
                           .unique()
                           .collect_vec();
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
    */
    pub fn show_menu_bar(&mut self, s: &GuiState) {
        if let Some(m) = s.begin_main_menu_bar() {
            self.show_document_menu(s);
            self.show_view_menu(s);
            m.end();
        }
    }

    fn show_document_menu(&mut self, s: &GuiState) {
        if let Some(f) = s.begin_menu("Document") {
            if s.menu_item("New Tab") {
                self.new_window(all_tiles());
            }

            if s.menu_item("Open") {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("Mosaic", &["mos"])
                    .set_directory(env::current_dir().unwrap())
                    .pick_file()
                {
                    self.document_mosaic.clear();
                    Self::prepare_mosaic(Arc::clone(&self.document_mosaic));

                    self.document_mosaic.load(&fs::read(file).unwrap()).unwrap();
                    // self.document_mosaic.send_toast("Document loaded");

                    self.document_mosaic.request_quadtree_update();
                }
            }

            if s.menu_item("Save") {
                let document = self.document_mosaic.save();
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("Mosaic", &["mos"])
                    .set_directory(env::current_dir().unwrap())
                    .save_file()
                {
                    fs::write(file, document).unwrap();
                    // self.document_mosaic.send_toast("Document saved");
                }
            }

            s.separator();

            if s.menu_item("Exit") {
                s.exit();
            }

            f.end();
        }
    }

    fn show_view_menu(&mut self, s: &GuiState) {
        if let Some(f) = s.begin_menu("View") {
            let tabview_on = {
                if self.show_tabview {
                    "x"
                } else {
                    ""
                }
            };

            if s.menu_item(format!("Show Tab View {}", tabview_on)) {
                self.show_tabview = !self.show_tabview;
            }

            let ruler_on = {
                let mut checked = "";
                // if let Some((_, tab)) = self.dock_state.find_active_focused() {
                //     if tab.ruler_visible {
                //         checked = "x";
                //     }
                // }
                checked
            };

            if s.menu_item(format!("Toggle Ruler {}", ruler_on)) {

                // if let Some((_, tab)) = self.dock_state.find_active_focused() {
                //     tab.ruler_visible = !tab.ruler_visible;
                // }
            }

            if s.menu_item(format!("Toggle Debug Draw {}", ruler_on)) {
                //if let Some((_, tab)) = self.dock_state.find_active_focused() {
                //    tab.editor_data.debug = !tab.editor_data.debug;
                //}
            }

            let grid_on = {
                let mut checked = "";
                // if let Some((_, tab)) = self.dock_state.find_active_focused() {
                //     if tab.grid_visible {
                //         checked = "x";
                //     }
                // }
                checked
            };

            if s.menu_item(format!("Toggle Grid {}", grid_on)) {
                // if let Some((_, tab)) = self.dock_state.find_active_focused() {
                //     tab.grid_visible = !tab.grid_visible;
                // }
            }

            f.end();
        }
    }
}

/*
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
        //self.menu_bar(ctx, frame);
        self.left_sidebar(ctx, frame);
        self.right_sidebar(ctx, frame);

        self.process_requests();

        self.show_tabs(ctx, frame);
        self.toasts.show(ctx);
    }
     */
