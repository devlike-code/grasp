use std::{
    collections::HashMap,
    env,
    fmt::Display,
    fs,
    path::PathBuf,
    str::FromStr,
    sync::Weak,
    sync::{Arc, Mutex},
    thread,
};

use imgui::{Condition, ImString, MouseButton, StyleColor, TreeNodeFlags};
use itertools::Itertools;
use layout::{
    backends::svg::SVGWriter,
    core::utils::save_to_file,
    gv::{self, GraphBuilder},
    topo::layout::VisualGraph,
};
use mosaic::{
    capabilities::QueueTile,
    internals::{
        par, void, Datatype, FromByteArray, Mosaic, MosaicCRUD, MosaicIO, MosaicTypelevelCRUD,
        Tile, TileFieldEmptyQuery, TileFieldSetter, ToByteArray, Value, S32,
    },
    iterators::{
        component_selectors::ComponentSelectors, tile_filters::TileFilters,
        tile_getters::TileGetters,
    },
};
use quadtree_rs::Quadtree;

use crate::{
    core::{gui::docking::GuiViewport, math::Rect2, queues},
    editor_state_machine::EditorState,
    grasp_editor_window::GraspEditorWindow,
    grasp_editor_window_list::GraspEditorWindowList,
    grasp_queues::ToastRequestQueue,
    grasp_render,
    grasp_transitions::QuadtreeUpdateCapability,
    utilities::Label,
    GuiState,
};
use mosaic::capabilities::ArchetypeSubject;
use mosaic::capabilities::QueueCapability;

type ComponentRenderer = Box<dyn Fn(&GuiState, &mut GraspEditorWindow, Tile) + Send + Sync>;

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
            // queues::enqueue(
            //     ToastRequestQueue,
            //     self.new_object("ToastRequest", par(text)),
            // );
            println!("ToastRequest enqueued");
        }
    }
}

#[allow(dead_code)]
pub struct GraspEditorState {
    pub editor_mosaic: Arc<Mosaic>,
    component_renderers: HashMap<S32, ComponentRenderer>,
    pub window_list: GraspEditorWindowList,
    pub editor_state_tile: Tile,
    pub new_tab_request_queue: QueueTile,
    pub refresh_quadtree_queue: QueueTile,
    pub toast_request_queue: QueueTile,
    pub loaded_categories: Vec<ComponentCategory>,
    show_tabview: bool,
    locked_components: Vec<S32>,
    queued_component_delete: Option<usize>,
}

#[derive(Default, Debug, Clone)]
pub struct ComponentEntry {
    pub name: String,
    pub display: String,
    pub hidden: bool,
}

#[derive(Default, Debug, Clone)]
pub struct ComponentCategory {
    pub name: String,
    pub components: Vec<ComponentEntry>,
    pub hidden: bool,
}

pub struct DisplayName<'a>(pub &'a Tile);
impl<'a> TileFieldEmptyQuery for DisplayName<'a> {
    type Output = Option<String>;
    fn query(&self) -> Self::Output {
        if let Some(pos_component) = self.0.get_component("DisplayName") {
            if let Value::S32(s) = pos_component.get("self") {
                return Some(s.to_string());
            }
        }

        None
    }
}

impl GraspEditorState {
    #[allow(clippy::comparison_chain)]
    pub fn close_window(&mut self, window_tile: Tile) {
        if let Some(pos) = self
            .window_list
            .windows
            .iter()
            .position(|w| w.window_tile == window_tile)
        {
            let window = self.window_list.windows.get(pos).unwrap();
            println!("Deleting {:?}", window.name);

            let p = self
                .window_list
                .named_windows
                .iter()
                .position(|n| n == &window.name)
                .unwrap();
            self.window_list.named_windows.remove(p);

            self.window_list.windows.remove(pos);
            self.editor_mosaic.delete_tile(window_tile);

            if let Some(first) = self.window_list.named_windows.first() {
                self.window_list.request_focus(first);
            }
        }
    }

    pub fn snapshot_all(&self, name: &str) {
        self.snapshot(format!("{}_EDITOR", name).as_str(), &self.editor_mosaic);

        for window in &self.window_list.windows {
            self.snapshot(
                format!("{}_WINDOW_{}", name, window.window_tile.id).as_str(),
                &window.document_mosaic,
            );
        }
    }

    fn generate_svg(graph: &mut VisualGraph, name: &str) {
        let mut svg = SVGWriter::new();
        graph.do_it(false, false, false, &mut svg);
        let content = svg.finalize();

        let output_path = format!(".//{}.svg", name);
        let res = save_to_file(output_path.as_str(), &content);
        if let Result::Err(err) = res {
            log::error!("Could not write the file {}", output_path.as_str());
            log::error!("Error {}", err);
        }
    }

    pub fn snapshot(&self, name: &str, mosaic: &Arc<Mosaic>) {
        let content = mosaic.dot(name);
        let mut parser = gv::DotParser::new(&content);
        match parser.process() {
            Ok(ast) => {
                println!("{:?}", ast);
                let mut gb = GraphBuilder::new();
                gb.visit_graph(&ast);
                let mut vg = gb.get();
                Self::generate_svg(&mut vg, name);
            }
            Err(err) => panic!("{:?}", err),
        }

        // let content = mosaic.dot(name);
        // open::that(format!(
        //     "https://dreampuf.github.io/GraphvizOnline/#{}",
        //     urlencoding::encode(content.as_str())
        // ))
        // .unwrap();
    }

    fn load_mosaic_components_from_file(
        mosaic: &Arc<Mosaic>,
        file: PathBuf,
    ) -> Vec<ComponentCategory> {
        let mut component_categories = vec![];
        let loader_mosaic = Mosaic::new();
        loader_mosaic.new_type("Node: unit;").unwrap();
        loader_mosaic.new_type("Arrow: unit;").unwrap();
        loader_mosaic.new_type("Label: s32;").unwrap();

        loader_mosaic.new_type("Hidden: unit;").unwrap();
        loader_mosaic.new_type("DisplayName: s32;").unwrap();

        loader_mosaic.load(&fs::read(file).unwrap()).unwrap();

        let categories = loader_mosaic.get_all().filter(|t| {
            t.is_object() && t.iter().get_arrows_into().len() == 0 && t.match_archetype(&["Label"])
        });

        categories.for_each(|menu| {
            let mut category = ComponentCategory {
                name: Label(&menu).query(),
                ..Default::default()
            };

            println!("\tCategory name: {:?}", category.name);
            if menu.match_archetype(&["Hidden"]) {
                category.hidden = true;
            }

            let items = menu.iter().get_arrows_from().get_targets();

            for item in items {
                let component_name = Label(&item).query();
                println!("\t\tItem: {:?}", item);
                assert_eq!(item.mosaic, loader_mosaic);
                println!("\t\tArrows: {:?}", item.iter().get_arrows_from());
                println!("\t\tComponent name: {:?}", component_name);

                let mut component_entry = ComponentEntry {
                    name: component_name.clone(),
                    display: component_name.clone(),
                    hidden: false,
                };

                if item.match_archetype(&["Hidden"]) {
                    component_entry.hidden = true;
                }

                if let Some(display) = DisplayName(&item).query() {
                    component_entry.display = display;
                }

                category.components.push(component_entry);

                let mut fields = vec![];

                let mut current_field = item
                    .iter()
                    .get_arrows_from()
                    .find(|t| t.iter().get_arrows_into().len() == 0);

                while current_field.is_some() {
                    let field = current_field.as_ref().unwrap();
                    let field_name = Label(field).query();
                    let field_datatype = Label(&field.target()).query();
                    println!("\t\t\t{:?}: {:?}", field_name, field_datatype);
                    fields.push((field_name, field_datatype));
                    current_field = field.iter().get_arrows_from().get_targets().next();
                }

                let formatted = if fields.is_empty() {
                    format!("{}: unit;", component_name)
                } else if fields.len() == 1 && fields.first().as_ref().unwrap().0.as_str() == "self"
                    || fields.first().as_ref().unwrap().0.as_str().is_empty()
                {
                    let (_, field_datatype) = fields.first().unwrap();
                    format!("{}: {};", component_name, field_datatype)
                } else {
                    let field_struct = fields
                        .iter()
                        .map(|(a, b)| format!("{}: {}", a, b))
                        .join(", ");
                    format!("{}: {{ {} }};", component_name, field_struct)
                };

                mosaic.new_type(&formatted).unwrap();
            }
            component_categories.push(category);
        });

        component_categories
    }

    pub fn prepare_mosaic(mosaic: Arc<Mosaic>) -> (Arc<Mosaic>, Vec<ComponentCategory>) {
        let components: Vec<ComponentCategory> = fs::read_dir("env\\components")
            .unwrap()
            .flat_map(|file_entry| {
                if let Ok(file) = file_entry {
                    println!("Loading {:?}", file);
                    Self::load_mosaic_components_from_file(&mosaic, file.path())
                } else {
                    vec![]
                }
            })
            .collect_vec();

        (mosaic, components)
    }

    pub fn new() -> Self {
        let editor_mosaic = Mosaic::new();
        let (_, components) = Self::prepare_mosaic(Arc::clone(&editor_mosaic));

        let editor_state_tile = editor_mosaic.new_object("EditorState", void());

        let new_window_request_queue = editor_mosaic.make_queue();
        new_window_request_queue.add_component("NewWindowRequestQueue", void());

        let refresh_quadtree_queue = editor_mosaic.make_queue();
        refresh_quadtree_queue.add_component("QuadtreeUpdateRequestQueue", void());

        let close_window_request_queue = editor_mosaic.make_queue();
        close_window_request_queue.add_component("CloseWindowRequestQueue", void());

        let named_focus_window_request_queue = editor_mosaic.make_queue();
        named_focus_window_request_queue.add_component("NamedFocusWindowRequestQueue", void());

        let toast_request_queue = editor_mosaic.make_queue();
        toast_request_queue.add_component("ToastRequestQueue", void());

        let new_editor_state = Self {
            editor_mosaic,
            component_renderers: HashMap::new(),
            editor_state_tile,
            new_tab_request_queue: new_window_request_queue,
            refresh_quadtree_queue,
            toast_request_queue,
            window_list: GraspEditorWindowList::default(),
            show_tabview: false,
            queued_component_delete: None,
            locked_components: vec![
                "Node".into(),
                "Arrow".into(),
                "Position".into(),
                "Offset".into(),
            ],
            loaded_categories: components,
        };

        new_editor_state
    }

    pub fn new_window(&mut self, name: Option<String>) {
        //new window tile that is at the same time "Queue" component
        let window_tile = self.editor_mosaic.make_queue();
        window_tile.add_component("EditorWindowQueue", void());

        //connecting all new windows with editor state tile
        self.editor_mosaic.new_arrow(
            &self.editor_state_tile,
            &window_tile,
            "DirectWindowRequest",
            void(),
        );

        let new_index = self.window_list.increment();
        let id = self.window_list.windows.len();

        let document_mosaic = Mosaic::new();
        assert!(self.editor_mosaic.id != document_mosaic.id);
        Self::prepare_mosaic(Arc::clone(&document_mosaic));
        assert!(self.editor_mosaic.id != document_mosaic.id);

        let filename = name.unwrap_or(format!("Untitled {}", new_index));
        let name = format!("[{}] {}", id, filename);

        let window = GraspEditorWindow {
            name: name.clone(),
            window_tile,
            quadtree: Mutex::new(Quadtree::new_with_anchor((-1000, -1000).into(), 16)),
            document_mosaic,
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
            grasp_editor_state: unsafe { Weak::from_raw(self) },
            window_list_index: id,
        };

        self.window_list.named_windows.push(name);
        self.window_list.windows.push_front(window);
    }

    pub fn show(&mut self, s: &GuiState) {
        self.show_hierarchy(s);
        self.show_properties(s);
        self.show_menu_bar(s);
        self.window_list.show(s);
    }

    fn show_hierarchy(&mut self, s: &GuiState) {
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
                    self.new_window(None);
                }

                let items = self
                    .window_list
                    .named_windows
                    .iter()
                    .map(|w| w.as_str())
                    .collect_vec();

                let mut i = if let Some(selected_window) = self.window_list.windows.front() {
                    self.window_list
                        .named_windows
                        .iter()
                        .position(|n| n == &selected_window.name)
                        .unwrap() as i32
                } else {
                    -1i32
                };
                s.ui.set_next_item_width(-1.0);
                let color =
                    s.ui.push_style_color(StyleColor::FrameBg, [0.1, 0.1, 0.15, 1.0]);

                if s.ui.list_box("##", &mut i, items.as_slice(), 20) {
                    let item: &str = items.get(i as usize).unwrap();
                    self.window_list.request_focus(item);
                }

                color.end();
            }

            w.end();
        }
    }

    fn show_properties(&mut self, s: &GuiState) {
        let viewport = GuiViewport::get_main_viewport();
        if let Some(w) =
            s.ui.window(ImString::new("Properties"))
                .position([viewport.size().x - 300.0, 18.0], Condition::FirstUseEver)
                .size([300.0, viewport.size().y - 18.0], Condition::FirstUseEver)
                .begin()
        {
            if let Some(focused_window) = self.window_list.windows.front_mut() {
                let mut selected = focused_window.editor_data.selected.clone();
                selected = selected.into_iter().unique().collect_vec();

                if !selected.is_empty() {
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
                                            format!("{} [ID: {}]", part, tile.id),
                                            TreeNodeFlags::DEFAULT_OPEN,
                                        ) {
                                            subheader_color.end();
                                            renderer(s, focused_window, tile.clone());
                                        }
                                    } else if s.ui.collapsing_header(
                                        format!("{} [ID: {}]", part, tile.id),
                                        TreeNodeFlags::DEFAULT_OPEN,
                                    ) {
                                        let is_locked =
                                            self.locked_components.contains(&tile.component);
                                        let is_header_covered = s.ui.is_item_hovered();
                                        let is_header_clicked =
                                            s.ui.is_item_clicked_with_button(MouseButton::Right);

                                        if !is_locked && is_header_covered && is_header_clicked {
                                            self.queued_component_delete = Some(tile.id);
                                            s.ui.open_popup(ImString::new("Component Menu"));
                                        }

                                        subheader_color.end();
                                        draw_default_property_renderer(
                                            s,
                                            focused_window,
                                            tile.clone(),
                                        );
                                    }
                                }
                            }
                            s.ui.unindent();
                        }

                        s.ui.spacing();
                        s.ui.spacing();
                        s.ui.separator();
                    }
                } else if s.ui.collapsing_header("Meta", TreeNodeFlags::empty()) {
                    for o in focused_window
                        .document_mosaic
                        .get_all()
                        .filter_objects()
                        .exclude_component("Node")
                    {
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
                                            format!("{} [ID: {}]", part, tile.id),
                                            TreeNodeFlags::DEFAULT_OPEN,
                                        ) {
                                            subheader_color.end();
                                            renderer(s, focused_window, tile.clone());
                                        }
                                    } else if s.ui.collapsing_header(
                                        format!("{} [ID: {}]", part, tile.id),
                                        TreeNodeFlags::DEFAULT_OPEN,
                                    ) {
                                        let is_locked =
                                            self.locked_components.contains(&tile.component);
                                        let is_header_covered = s.ui.is_item_hovered();
                                        let is_header_clicked =
                                            s.ui.is_item_clicked_with_button(MouseButton::Right);

                                        if !is_locked && is_header_covered && is_header_clicked {
                                            self.queued_component_delete = Some(tile.id);
                                            s.ui.open_popup(ImString::new("Component Menu"));
                                        }

                                        subheader_color.end();
                                        draw_default_property_renderer(
                                            s,
                                            focused_window,
                                            tile.clone(),
                                        );
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

                s.ui.popup(ImString::new("Component Menu"), || {
                    if s.ui.menu_item(ImString::new("Delete")) {
                        if let Some(tile) = self.queued_component_delete {
                            println!("DELETING TILE {}", tile);
                            if let Some(window) = self.window_list.get_focused() {
                                window.document_mosaic.delete_tile(tile);
                                self.queued_component_delete = None;
                            }
                        }
                    }
                });
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
                self.new_window(None);
            }

            if s.menu_item("Open") {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("Mosaic", &["mos"])
                    .set_directory(env::current_dir().unwrap())
                    .pick_file()
                {
                    self.new_window(Some(
                        file.file_name()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_string()
                            .clone(),
                    ));
                    let window = self.window_list.windows.front().unwrap();
                    let window_mosaic = &window.document_mosaic;

                    Self::prepare_mosaic(Arc::clone(window_mosaic));
                    window_mosaic.load(&fs::read(file).unwrap()).unwrap();
                    self.editor_mosaic.request_quadtree_update();
                }
            }

            if s.menu_item("Save") {
                if let Some(focused_window) = self.window_list.get_focused() {
                    assert!(focused_window.document_mosaic.id != self.editor_mosaic.id);
                    self.snapshot_all("SAVED");
                    let document = focused_window.document_mosaic.save();
                    if let Some(file) = rfd::FileDialog::new()
                        .add_filter("Mosaic", &["mos"])
                        .set_directory(env::current_dir().unwrap())
                        .save_file()
                    {
                        fs::write(file, document).unwrap();
                        self.editor_mosaic.send_toast("Document saved");
                    }
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

            window.request_quadtree_update();
        }
    }
}
