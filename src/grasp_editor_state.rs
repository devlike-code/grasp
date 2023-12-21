use std::{
    collections::HashMap,
    env,
    fmt::Display,
    fs,
    rc::Weak,
    str::FromStr,
    sync::{Arc, Mutex},
};

use imgui::{Condition, ImString, StyleColor, TreeNodeFlags};
use itertools::Itertools;
use mosaic::{
    capabilities::QueueTile,
    internals::{
        all_tiles, par, void, Collage, Datatype, FromByteArray, Mosaic, MosaicCRUD, MosaicIO,
        MosaicTypelevelCRUD, Tile, TileFieldEmptyQuery, TileFieldSetter, ToByteArray, Value, S32,
    },
};
use quadtree_rs::Quadtree;

use crate::{
    core::{gui::docking::GuiViewport, math::Rect2},
    editor_state_machine::EditorState,
    grasp_editor_window::GraspEditorWindow,
    grasp_editor_window_list::{GetWindowFocus, GraspEditorWindowList},
    grasp_render,
    grasp_transitions::QuadtreeUpdateCapability,
    GuiState,
};
use mosaic::capabilities::ArchetypeSubject;
use mosaic::capabilities::QueueCapability;

type ComponentRenderer = Box<dyn Fn(&GuiState, &mut GraspEditorWindow, Tile)>;

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

#[allow(dead_code)]
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
        mosaic.new_type("Node: unit;").unwrap();
        mosaic.new_type("Arrow: unit;").unwrap();
        mosaic.new_type("Label: s32;").unwrap();
        mosaic.new_type("Position: { x: f32, y: f32 };").unwrap();
        mosaic.new_type("Selection: unit;").unwrap();
        mosaic.new_type("EditorState: unit;").unwrap();
        mosaic.new_type("EditorStateFocusedWindow: u64;").unwrap();
        mosaic.new_type("EditorWindowQueue: unit;").unwrap();
        mosaic.new_type("ToWindow: unit;").unwrap();
        mosaic.new_type("NewWindowRequestQueue: unit;").unwrap();
        mosaic
            .new_type("QuadtreeUpdateRequestQueue: unit;")
            .unwrap();
        mosaic.new_type("FocusWindowRequest: unit;").unwrap();
        mosaic.new_type("QuadtreeUpdateRequest: unit;").unwrap();
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
        refresh_quadtree_queue.add_component("QuadtreeUpdateRequestQueue", void());

        let toast_request_queue = document_mosaic.make_queue();
        toast_request_queue.add_component("ToastRequestQueue", void());

        //let dock_state = DockState::new(vec![]);

        // add here default renderers

        // state
        //     .component_renderers
        //     .insert("Label".into(), Box::new(Self::draw_label_property));

        // state
        //     .component_renderers
        //     .insert("Position".into(), Box::new(Self::draw_position_property));

        //        let tab = state.new_tab(all_tiles());
        //        state.dock_state.main_surface_mut().push_to_first_leaf(tab);

        Self {
            document_mosaic,
            component_renderers: HashMap::new(),
            editor_state_tile,
            new_tab_request_queue: new_window_request_queue,
            refresh_quadtree_queue,
            toast_request_queue,
            window_list: GraspEditorWindowList::default(),
            show_tabview: false,
        }
    }

    pub fn new_window(&mut self, collage: Box<Collage>) {
        //new window tile that is at the same time "Queue" component
        let window_tile = self.document_mosaic.make_queue();
        window_tile.add_component("EditorWindowQueue", void());

        //connecting all new windows with editor state tile
        self.document_mosaic
            .new_arrow(&self.editor_state_tile, &window_tile, "ToWindow", void());

        let new_index = self.window_list.increment();
        let id = self.window_list.current_index as usize - 1;
        let window = GraspEditorWindow {
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
            renderer: grasp_render::default_renderer_draw,
            left_drag_last_frame: false,
            middle_drag_last_frame: false,
            title_bar_drag: false,
            rect: Rect2 {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            },
            window_list: unsafe { Weak::from_raw(&self.window_list) },
            window_list_index: id,
        };

        self.window_list.windows.push(window);
        self.window_list
            .depth_sorted_by_index
            .lock()
            .unwrap()
            .push_front(id);
    }

    pub fn show(&mut self, s: &GuiState) {
        self.show_left_sidebar(s);
        self.show_right_sidebar(s);
        self.show_menu_bar(s);
        self.window_list.show(s);
    }

    fn get_window_by_index(&self, focused_index: usize) -> i32 {
        self.window_list
            .windows
            .iter()
            .position(|w| w.window_tile.id == focused_index)
            .unwrap_or_default() as i32
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

                if let Some(focused_index) = GetWindowFocus(&self.document_mosaic).query() {
                    let mut i = self.get_window_by_index(focused_index);

                    let items = self
                        .window_list
                        .windows
                        .iter()
                        .map(|w| w.name.as_str())
                        .collect_vec();

                    s.ui.set_next_item_width(-1.0);
                    let color =
                        s.ui.push_style_color(StyleColor::FrameBg, [0.1, 0.1, 0.15, 1.0]);

                    if s.ui.list_box("##", &mut i, items.as_slice(), 20) {
                        let item: &str = items.get(i as usize).unwrap();

                        self.window_list.focus(item);
                    }

                    color.end();

                    let items = self
                        .window_list
                        .depth_sorted_by_index
                        .lock()
                        .unwrap()
                        .iter()
                        .map(|w| self.window_list.windows.get(*w).unwrap().name.as_str())
                        .collect_vec();

                    s.ui.separator();
                    s.ui.set_next_item_width(-1.0);
                    let color =
                        s.ui.push_style_color(StyleColor::FrameBg, [0.1, 0.1, 0.15, 1.0]);
                    s.ui.list_box("##depth-index", &mut 0, items.as_slice(), 20);
                    color.end();
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
            let focused_index = GetWindowFocus(&self.document_mosaic).query();

            if let Some(focused_window) = self.window_list.get_position(focused_index) {
                let mut selected = focused_window.editor_data.selected.clone();
                selected = selected.into_iter().unique().collect_vec();

                for o in selected {
                    let header_color = s.ui.push_style_color(
                        imgui::StyleColor::Header,
                        [34.0 / 255.0, 43.0 / 255.0, 90.0 / 255.0, 1.0],
                    );
                    if s.ui.collapsing_header(
                        format!("ID: {}##{}-header", o.id, o.id),
                        TreeNodeFlags::DEFAULT_OPEN,
                    ) {
                        header_color.end();
                        s.ui.indent();
                        for (part, tiles) in &o
                            .get_full_archetype()
                            .into_iter()
                            .sorted_by(|a, b| (a.1.first().cmp(&b.1.first())))
                            .collect_vec()
                        {
                            for tile in tiles.iter().sorted_by(|a, b| a.id.cmp(&b.id)) {
                                let subheader_color = s.ui.push_style_color(
                                    imgui::StyleColor::Header,
                                    [66.0 / 255.0, 64.0 / 255.0, 123.0 / 255.0, 1.0],
                                );
                                if let Some(renderer) =
                                    self.component_renderers.get(&part.as_str().into())
                                {
                                    if s.ui.collapsing_header(
                                        format!("{} [ID: {}]", part.to_uppercase(), tile.id),
                                        TreeNodeFlags::DEFAULT_OPEN,
                                    ) {
                                        subheader_color.end();
                                        renderer(s, focused_window, tile.clone());
                                    }
                                } else if s.ui.collapsing_header(
                                    format!("{} [ID: {}]", part.to_uppercase(), tile.id),
                                    TreeNodeFlags::DEFAULT_OPEN,
                                ) {
                                    subheader_color.end();
                                    draw_default_property_renderer(s, focused_window, tile.clone());
                                }
                            }
                        }
                        s.ui.unindent();
                    }

                    s.ui.spacing();
                    s.ui.spacing();
                    s.ui.separator();
                }
            }
            w.end();
        }
    }

    pub fn show_menu_bar(&mut self, s: &GuiState) {
        if let Some(m) = s.begin_main_menu_bar() {
            self.show_document_menu(s);
            self.show_view_menu(s);
            m.end();
        }
    }

    fn show_document_menu(&mut self, s: &GuiState) {
        if let Some(f) = s.begin_menu("Document") {
            if s.menu_item("New Window") {
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
                if let Some(window) = self.window_list.get_focused_mut() {
                    window.editor_data.debug = !window.editor_data.debug;
                }
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

fn draw_default_property_renderer(ui: &GuiState, tab: &mut GraspEditorWindow, d: Tile) {
    let mosaic = &tab.document_mosaic;
    let comp = mosaic
        .component_registry
        .get_component_type(d.component)
        .unwrap();
    let fields = comp.get_fields();

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

        match value {
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
            Value::S32(v) => draw_property_value(ui, tab, &d, name.as_str(), v.to_string()),
            Value::S128(v) => {
                draw_property_value(ui, tab, &d, name.as_str(), String::from_byte_array(&v))
            }

            Value::BOOL(v) => {
                let mut b = v;
                ui.checkbox("", &mut b);
            }
        }
    }

    // }
}
/* */
fn draw_property_value<T: Display + FromStr + ToByteArray>(
    state: &GuiState,
    window: &mut GraspEditorWindow,
    tile: &Tile,
    name: &str,
    t: T,
) where
    Tile: TileFieldSetter<T>,
{
    let datatype = tile.get(name).get_datatype();
    let id = format!("##{}.{}", tile.id, name);
    let mut text = format!("{}", t);
    let previous_text = format!("{}", t);

    state.ui.columns(2, "##", false);
    let region_width = state.ui.window_content_region_max()[0];
    let max_label_width = 100.0;
    let mut label_width = region_width * 0.25;
    let text_width = if label_width > max_label_width {
        label_width = max_label_width;
        region_width - max_label_width
    } else {
        state.ui.window_content_region_max()[0] * 0.75
    };

    state.ui.set_column_width(0, label_width);
    state.ui.set_column_width(1, text_width);

    state.ui.text(name);
    state.ui.next_column();
    state.ui.set_next_item_width(-1.0);

    let color = state.ui.push_style_color(
        StyleColor::FrameBg,
        [98.0 / 255.0, 86.0 / 255.0, 160.0 / 255.0, 1.0],
    );
    match datatype {
        Datatype::S32 => {
            state
                .ui
                .input_text(id, &mut text)
                .enter_returns_true(true)
                .build();

            if text.len() >= 32 {
                text = text[0..32].to_string();
            }
        }

        Datatype::S128 => {
            state
                .ui
                .input_text_multiline(
                    id,
                    &mut window.editor_data.text,
                    state.ui.content_region_avail(),
                )
                .enter_returns_true(true)
                .build();

            if text.len() >= 128 {
                text = text[0..128].to_string();
            }
        }

        _ => {
            state
                .ui
                .input_text(id, &mut text)
                .enter_returns_true(true)
                .build();
        }
    };
    color.end();
    state.ui.columns(1, "##", false);
    if let Ok(t) = text.parse::<T>() {
        if previous_text != text {
            tile.clone().set(name, t);
            //todo discuss better solution for this, can we have something like tile changed queue?
            tile.mosaic.request_quadtree_update();
        }
    }
}
