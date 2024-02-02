use std::{
    env,
    fmt::Display,
    fs::{self, File},
    io::Write,
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};

use imgui::{
    sys::igBegin, Condition, DrawListMut, ImString, MouseButton, StyleColor, TreeNodeToken,
    WindowFlags,
};
use itertools::Itertools;
use log::error;
use mosaic::{
    capabilities::{ArchetypeSubject, QueueCapability},
    internals::{void, Datatype, MosaicCRUD, MosaicIO, Tile, TileFieldSetter, ToByteArray, Value},
    iterators::{
        component_selectors::ComponentSelectors, tile_deletion::TileDeletion,
        tile_filters::TileFilters,
    },
};

use crate::{
    core::{
        gui::{
            docking::{gui_str, GuiViewport},
            windowing::gui_set_window_focus,
        },
        math::{Rect2, Vec2},
        structures::{grasp_queues, ErrorCapability},
    },
    editor_state_machine::EditorState,
    grasp_queues::CloseWindowRequestQueue,
    GuiState,
};

use super::{
    file_operations::SaveFileCapability,
    foundation::GraspEditorState,
    helpers::{QuadtreeUpdateCapability, RequireWindowFocus},
    sense::hash_input,
    windows::GraspEditorWindow,
};

pub type DeleteReaction = Box<dyn Fn(&mut GraspEditorWindow, String, &Tile) + Send + Sync>;

pub type ComponentRenderer =
    Box<dyn Fn(&GuiState, &mut GraspEditorWindow, Tile, &mut DrawListMut<'_>) + Send + Sync>;

pub type ComponentPropertyRenderer =
    Box<dyn Fn(&GuiState, &mut GraspEditorWindow, Tile) + Send + Sync>;

impl GraspEditorState {
    pub fn show(&mut self, s: &GuiState) {
        {
            let io = s.ui.io();
            if io.key_ctrl && io.keys_down[22] {
                // CTRL+S
                if let Some(w) = self.window_list.get_focused_mut() {
                    w.save_file()
                }
            }
        }

        if self.show_hierarchy {
            self.show_hierarchy(s);
        }
        
        self.show_properties(s);
        self.show_menu_bar(s);

        let mut caught_events = vec![];

        self.show_windows(s, &mut caught_events);

        self.show_errors(s);

        // TODO: FIX LATER
        // if self.pending_close_window_request.is_none()
        //     && !caught_events.contains(&hash_input("double click left"))
        //     && s.ui.is_mouse_double_clicked(imgui::MouseButton::Left)
        //     && s.ui.get
        // {
        //     self.open_files();
        // }

        caught_events.clear();
    }

    pub fn show_windows(&mut self, s: &GuiState, caught_events: &mut Vec<u64>) {
        let len = self.window_list.windows.len();
        let front_window_id = self.window_list.windows.front().map(|w| w.window_tile.id);

        for window_index in 0..len {
            let mut opened = true;
            let (window_name, window_id, changed) = {
                let window = self.window_list.windows.get(window_index).unwrap();
                (window.name.clone(), window.window_tile.id, window.changed)
            };
            let mut w = s.ui.window(window_name);
            if changed {
                w = w.flags(WindowFlags::UNSAVED_DOCUMENT.union(WindowFlags::NO_COLLAPSE));
            } else {
                w = w.flags(WindowFlags::NO_COLLAPSE);
            }
            w = w.opened(&mut opened);
            w.size_constraints([320.0, 240.0], [1024.0, 768.0])
                .scroll_bar(false)
                .size([700.0, 500.0], imgui::Condition::Appearing)
                .position(
                    [
                        200.0 + 50.0 * (window_id % 5) as f32,
                        200.0 - 20.0 * (window_id % 5) as f32,
                    ],
                    imgui::Condition::Appearing,
                )
                .build(|| {
                    let window = self.window_list.windows.get_mut(window_index).unwrap();
                    window.rect =
                        Rect2::from_pos_size(s.ui.window_pos().into(), s.ui.window_size().into());

                    let title_bar_rect =
                        Rect2::from_pos_size(window.rect.min(), Vec2::new(window.rect.width, 18.0));

                    if window.title_bar_drag && s.ui.is_mouse_released(imgui::MouseButton::Left) {
                        window.title_bar_drag = false;
                    } else if !window.title_bar_drag {
                        if title_bar_rect.contains(s.ui.io().mouse_pos.into())
                            && s.ui.is_mouse_clicked(imgui::MouseButton::Left)
                        {
                            window.title_bar_drag = true;
                        } else if self.pending_close_window_request.is_none() {
                            window.sense(
                                s,
                                front_window_id,
                                caught_events,
                                self.properties_hovered,
                            );
                        }
                    }

                    if title_bar_rect.contains(s.ui.io().mouse_pos.into())
                        && s.ui.is_mouse_double_clicked(imgui::MouseButton::Left)
                    {
                        caught_events.push(hash_input("double click left"));
                    }

                    let window_offset: Vec2 = s.ui.window_pos().into();

                    if window.editor_data.window_offset != window_offset {
                        window.editor_data.window_offset = window_offset;
                        window.update_quadtree(None);
                    } else {
                        window.editor_data.window_offset = window_offset;
                    }

                    let is_other_window_focused =
                        front_window_id.is_some_and(|w| w != window.window_tile.id);

                    if s.ui.is_window_focused() && is_other_window_focused {
                        window.require_named_window_focus(&window.name.clone());
                    }

                    if is_other_window_focused {
                        window.state = EditorState::Idle;
                    }

                    if let Some(request) = self.editor_mosaic.dequeue(&window.window_tile) {
                        match request.component.to_string().as_str() {
                            "QuadtreeUpdateRequest" => {
                                println!("UPDATING QUAD TREE {} FROM QUEUE", window.name);
                                window.update_quadtree(None);
                                request.iter().delete();
                            }
                            "FocusWindowRequest" => {
                                println!("FOCUSING WINDOW {} FROM QUEUE", window.name);
                                gui_set_window_focus(&window.name);
                                request.iter().delete();

                                window.require_named_window_focus(&window.name.clone());
                            }
                            other => {
                                error!("REQUEST UNFULFILLED: {:?}", other);
                            }
                        }
                    }

                    (window.renderer)(window, s, &self.component_entity_renderers);

                    window.draw_debug(s);

                    if self.pending_close_window_request.is_none() {
                        window.update_context_menu(front_window_id, s);
                        window.context_popup(s);
                    }

                    if self.pending_close_window_request.is_none()
                        && title_bar_rect.contains(s.ui.io().mouse_pos.into())
                        && s.ui.is_mouse_clicked(imgui::MouseButton::Middle)
                        && grasp_queues::is_empty(CloseWindowRequestQueue, &self.editor_mosaic)
                    {
                        let request = window
                            .editor_mosaic
                            .new_object("CloseWindowRequest", void());
                        grasp_queues::enqueue(CloseWindowRequestQueue, request);
                    }
                });

            if !opened {
                // close was clicked
                let window = self.window_list.windows.get_mut(window_index).unwrap();
                let request = window
                    .editor_mosaic
                    .new_object("CloseWindowRequest", void());
                grasp_queues::enqueue(CloseWindowRequestQueue, request);
            }

            self.window_list
                .windows
                .get_mut(window_index)
                .unwrap()
                .update(s);
        }
    }

    fn show_errors(&mut self, s: &GuiState) {
        if let Some(_w) =
            s.ui.window(ImString::new("Errors"))
                .position([100.0, 100.0], Condition::FirstUseEver)
                .size([300.0, 100.0], Condition::FirstUseEver)
                .begin()
        {
            s.ui.columns(3, "errors_columns", true);
            s.ui.set_column_width(0, 150.0);
            s.ui.set_column_width(1, 150.0);

            s.ui.text("Window ID");
            s.ui.next_column();

            s.ui.text("Entity ID");
            s.ui.next_column();

            s.ui.text("Message");
            s.ui.next_column();

            for error in self.editor_mosaic.get_all().include_component("Error") {
                let id = error.get("window").as_u64() as usize;
                let name = self
                    .window_list
                    .windows
                    .iter()
                    .find(|w| w.window_tile.id == id)
                    .unwrap()
                    .name
                    .clone();
                s.ui.text(&name);
                s.ui.next_column();

                s.ui.text(format!("{}", error.get("target").as_u64()));
                s.ui.next_column();

                s.ui.text(&error.get("message").as_str());
                s.ui.next_column();
            }
        }
    }

    fn show_hierarchy(&mut self, s: &GuiState) {
        let viewport = GuiViewport::get_main_viewport();
        if let Some(_w) =
            s.ui.window(ImString::new("Hierarchy"))
                .position([0.0, 18.0], Condition::FirstUseEver)
                .size(
                    [viewport.size().x, viewport.size().y - 18.0],
                    Condition::FirstUseEver,
                )
                .begin()
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

            if s.ui.list_box("##window-list", &mut i, items.as_slice(), 20) {
                let item: &str = items.get(i as usize).unwrap();
                self.require_named_window_focus(item);
                println!("Focus on {}", item);
            }

            color.end();
        }
    }

    fn show_properties(&mut self, s: &GuiState) {
        fn tree<'a, S: AsRef<str>>(
            s: &'a GuiState,
            t: &'a S,
            bullet: bool,
            open: bool,
        ) -> Option<TreeNodeToken<'a>> {
            s.ui.tree_node_config(t.as_ref())
                .default_open(open)
                .bullet(bullet)
                .push()
        }

        let viewport = GuiViewport::get_main_viewport();
        if let Some(w) =
            s.ui.window(ImString::new("Properties"))
                .position([viewport.size().x - 300.0, 18.0], Condition::FirstUseEver)
                .size([300.0, viewport.size().y - 18.0], Condition::FirstUseEver)
                .begin()
        {
            let props_contains_mouse = {
                let r1 = s.ui.window_pos();
                let r2 = s.ui.window_size();
                let r = Rect2::from_pos_size(r1.into(), r2.into());
                let m = s.ui.io().mouse_pos;
                r.contains(m.into())
            };

            self.properties_hovered = props_contains_mouse;

            if let Some(focused_window) = self.window_list.windows.front_mut() {
                let mut selected = focused_window.editor_data.selected.clone();
                selected = selected.into_iter().unique().collect_vec();

                if !selected.is_empty() {
                    for selected_tile in selected {
                        if let Some(_node_token) = tree(
                            s,
                            &format!("Entity {}##{}-header", selected_tile.id, selected_tile.id),
                            false,
                            true,
                        ) {
                            s.ui.separator();
                            for (part, tiles) in &selected_tile
                                .get_full_archetype()
                                .into_iter()
                                .sorted_by(|a, b| (a.1.first().cmp(&b.1.first())))
                                .collect_vec()
                            {
                                for part_tile in tiles.iter().sorted_by(|a, b| a.id.cmp(&b.id)) {
                                    if let Some(renderer) =
                                        self.component_property_renderers.get(part)
                                    {
                                        if let Some(_subnode_token) =
                                            tree(s, &part.to_string(), false, true)
                                        {
                                            renderer(s, focused_window, part_tile.clone());
                                        }
                                    } else {
                                        let is_bullet = {
                                            let comp = focused_window
                                                .document_mosaic
                                                .component_registry
                                                .get_component_type(part_tile.component)
                                                .unwrap();

                                            let fields = comp.get_fields();
                                            fields.len() == 1
                                                && fields.first().unwrap().datatype
                                                    == Datatype::UNIT
                                        };

                                        if let Some(_subnode_token) = tree(
                                            s,
                                            &part.to_string(),
                                            is_bullet,
                                            !self.hidden_property_renderers.contains(part),
                                        ) {
                                            draw_default_property_renderer(
                                                s,
                                                focused_window,
                                                part_tile.clone(),
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        s.ui.spacing();
                        s.ui.spacing();
                        s.ui.separator();
                    }
                } else {
                    let is_meta_present = {
                        focused_window
                            .document_mosaic
                            .get_all()
                            .filter_objects()
                            .exclude_component("Node")
                            .len()
                            > 0
                    };

                    if is_meta_present {
                        if let Some(_subnode_token) = tree(s, &"Meta", false, true) {
                            for o in focused_window
                                .document_mosaic
                                .get_all()
                                .filter_objects()
                                .exclude_component("Node")
                            {
                                if let Some(_subnode_token) = tree(
                                    s,
                                    &format!("[META] Entity {}##{}-header", o.id, o.id),
                                    false,
                                    true,
                                ) {
                                    for (component, tiles) in &o
                                        .get_full_archetype()
                                        .into_iter()
                                        .sorted_by(|a, b| (a.1.first().cmp(&b.1.first())))
                                        .collect_vec()
                                    {
                                        for tile in tiles.iter().sorted_by(|a, b| a.id.cmp(&b.id)) {
                                            if let Some(renderer) =
                                                self.component_property_renderers.get(component)
                                            {
                                                if let Some(_subnode_token) = tree(
                                                    s,
                                                    &component.to_string(),
                                                    false,
                                                    !self
                                                        .hidden_property_renderers
                                                        .contains(component),
                                                ) {
                                                    renderer(s, focused_window, tile.clone());
                                                }
                                            } else if let Some(_subnode_token) = tree(
                                                s,
                                                &component.to_string(),
                                                false,
                                                !self.hidden_property_renderers.contains(component),
                                            ) {
                                                let is_locked = self
                                                    .locked_components
                                                    .contains(&tile.component);
                                                let is_header_covered = s.ui.is_item_hovered();
                                                let is_header_clicked =
                                                    s.ui.is_item_clicked_with_button(
                                                        MouseButton::Right,
                                                    );

                                                if !is_locked
                                                    && is_header_covered
                                                    && is_header_clicked
                                                {
                                                    self.queued_component_delete = Some(tile.id);
                                                    s.ui.open_popup("component-menu");
                                                }

                                                draw_default_property_renderer(
                                                    s,
                                                    focused_window,
                                                    tile.clone(),
                                                );
                                            }
                                        }
                                    }
                                }

                                s.ui.spacing();
                                s.ui.spacing();
                                s.ui.separator();
                            }
                        }
                    }
                }
            }

            w.end();
        }
    }

    pub fn show_menu_bar(&mut self, s: &GuiState) {
        if let Some(m) = s.begin_main_menu_bar() {
            self.show_document_menu(s);

            if let Some(_f) = s.begin_menu("View") {
                self.show_view_menu(s);
            }

            if let Some(_f) = s.begin_menu("Windows") {
                self.show_windows_menu(s);
            }

            m.end();
        }
    }

    pub fn prepend_recent(file: PathBuf) {
        let path = file.to_str().unwrap().to_string();
        let mut recent = vec![];
        if let Ok(recent_list) = fs::read_to_string("env\\recent.txt") {
            recent = recent_list
                .lines()
                .map(|s| s.to_string())
                .filter(|p| p != &path)
                .collect_vec();
        }

        recent.insert(0, path.clone());

        let mut f = File::create("env\\recent.txt").unwrap();
        f.write_all(recent.join("\n").as_bytes()).unwrap();
    }

    fn open_file(&mut self, file: PathBuf) {
        if fs::metadata(file.clone()).is_ok() {
            self.new_window(Some(&file));
            let window = self.window_list.windows.front().unwrap();
            let window_mosaic = &window.document_mosaic;

            Self::prepare_mosaic(
                &window.component_mosaic,
                &self.editor_mosaic,
                Arc::clone(window_mosaic),
            );

            if window_mosaic.load(&fs::read(file.clone()).unwrap()).is_ok() {
                Self::prepend_recent(file);
                self.editor_mosaic.request_quadtree_update();
            } else {
                self.editor_mosaic.make_error(
                    &format!(
                        "Cannot open file path {}: invalid format",
                        file.as_path().to_str().unwrap_or_default()
                    ),
                    None,
                    None,
                );
                self.close_window(window.window_tile.clone());
            }
        }
    }

    fn open_files(&mut self) {
        if let Some(files) = rfd::FileDialog::new()
            .add_filter("Mosaic", &["mos"])
            .set_directory(env::current_dir().unwrap())
            .pick_files()
        {
            for file in files {
                self.open_file(file);
            }
        }
    }

    fn show_document_menu(&mut self, s: &GuiState) {
        if let Some(f) = s.begin_menu("Document") {
            if s.menu_item("New Window") {
                self.new_window(None);
            }

            if s.menu_item("Open") {
                self.open_files();
            }

            if let Some(_t) = s.begin_menu("Recent") {
                if let Ok(recent_list) = fs::read_to_string("env\\recent.txt") {
                    for entry in recent_list.lines().map(|s| s.to_string()) {
                        if s.menu_item(&entry) {
                            self.open_file(entry.into());
                        }
                    }
                }
            }

            s.separator();

            if s.menu_item("Save") {
                self.save_file();
            }

            if s.menu_item("Save As") {
                self.save_file_as();
            }

            s.separator();

            if s.menu_item("Exit") {
                s.exit();
            }

            f.end();
        }
    }

    fn xo(b: Option<bool>) -> String {
        if let Some(s) = b {
            (if s { "X" } else { " " }).to_string()
        } else {
            " ".to_string()
        }
    }

    fn show_view_menu(&mut self, s: &GuiState) {
        let tabview_on = if self.show_tabview { "X" } else { " " };
        let (grid_on, debug_on, ruler_on) = {
            let front = self.window_list.windows.front();
            let grid_on = Self::xo(front.map(|m| m.grid_visible));
            let debug_on = Self::xo(front.map(|m| m.editor_data.debug));
            let ruler_on = Self::xo(front.map(|m| m.ruler_visible));
            (grid_on, debug_on, ruler_on)
        };

        if s.menu_item(format!("[{}] Show Tab View", tabview_on)) {
            self.show_tabview = !self.show_tabview;
        }

        if s.menu_item(format!("[{}] Toggle Debug Draw", debug_on)) {
            if let Some(window) = self.window_list.windows.front_mut() {
                window.editor_data.debug = !window.editor_data.debug;
            }
        }

        if s.menu_item(format!("[{}] Toggle Grid", grid_on)) {
            if let Some(window) = self.window_list.windows.front_mut() {
                window.grid_visible = !window.grid_visible;
            }
        }
    }

    fn show_windows_menu(&mut self, s: &GuiState) {
        let hierarchy_on = if self.show_hierarchy { "X" } else { " " };

        if s.ui.button(format!("[{}] Hierarchy", hierarchy_on)) {
            self.show_hierarchy = !self.show_hierarchy;
            if self.show_hierarchy {
                self.show_hierarchy(s);
            }
        }

        s.ui.separator();
        s.ui.separator();

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

        if s.ui.list_box("##window-list", &mut i, items.as_slice(), 20) {
            let item: &str = items.get(i as usize).unwrap();
            self.require_named_window_focus(item);
            println!("Focus on {}", item);
        }

        color.end();
    }
}

pub fn two_float_property_xy_renderer(ui: &GuiState, window: &mut GraspEditorWindow, tile: Tile) {
    let mosaic = &window.document_mosaic;
    let _comp = mosaic
        .component_registry
        .get_component_type(tile.component)
        .unwrap();
    let x = tile.get("x").as_f32();
    let y = tile.get("y").as_f32();

    if ui
        .input_float2(format!("##{}-xy", tile.id).as_str(), &mut [x, y])
        .enter_returns_true(true)
        .build()
    {
        tile.clone().set("x", x);
        tile.clone().set("y", y);
        window.changed = true;
        window.request_quadtree_update();
    }
}

pub fn color_property_renderer(ui: &GuiState, _window: &mut GraspEditorWindow, tile: Tile) {
    let r = tile.get("r").as_f32();

    let g = tile.get("g").as_f32();
    let b = tile.get("b").as_f32();
    let a = tile.get("a").as_f32();

    let mut input = [r, g, b, a];
    ui.color_picker4("Color", &mut input);

    let mut tile = tile.clone();
    tile.set("r", input[0]);
    tile.set("g", input[1]);
    tile.set("b", input[2]);
    tile.set("a", input[3]);
}

#[allow(dead_code)]
pub fn selection_property_renderer(ui: &GuiState, window: &mut GraspEditorWindow, tile: Tile) {
    if let Some(color) = tile.get_component("Color") {
        assert!(tile.mosaic.is_tile_valid(&color));
        color_property_renderer(ui, window, color);
    } else {
        ui.text("No color");
    }
}

#[allow(dead_code)]
pub fn selected_property_renderer(
    ui: &GuiState,
    window: &mut GraspEditorWindow,
    selection_owner: Tile,
) {
    ui.text(format!("Selection owner: {:?}", selection_owner));
    selection_property_renderer(ui, window, selection_owner);
}

fn draw_default_property_renderer(ui: &GuiState, window: &mut GraspEditorWindow, d: Tile) {
    let mosaic = &window.document_mosaic;
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
            Value::I8(v) => draw_property_value(ui, window, &d, name.as_str(), v),
            Value::I16(v) => draw_property_value(ui, window, &d, name.as_str(), v),
            Value::I32(v) => draw_property_value(ui, window, &d, name.as_str(), v),
            Value::I64(v) => draw_property_value(ui, window, &d, name.as_str(), v),
            Value::U8(v) => draw_property_value(ui, window, &d, name.as_str(), v),
            Value::U16(v) => draw_property_value(ui, window, &d, name.as_str(), v),
            Value::U32(v) => draw_property_value(ui, window, &d, name.as_str(), v),
            Value::U64(v) => draw_property_value(ui, window, &d, name.as_str(), v),
            Value::F32(v) => draw_property_value(ui, window, &d, name.as_str(), v),
            Value::F64(v) => draw_property_value(ui, window, &d, name.as_str(), v),
            Value::S32(v) => draw_property_value(ui, window, &d, name.as_str(), v),
            Value::STR(v) => draw_property_value(ui, window, &d, name.as_str(), v),

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

    let committed = match datatype {
        Datatype::S32 => {
            let committed = state
                .ui
                .input_text(format!("{}##{}", name, id), &mut text)
                .auto_select_all(true)
                .enter_returns_true(true)
                .build();

            if text.len() >= 32 {
                text = text[0..32].to_string();
            }

            committed
        }

        Datatype::STR => {
            let rect = state.ui.content_region_avail();
            let committed = state
                .ui
                .input_text_multiline(
                    format!("{}##{}", name, id),
                    &mut text,
                    [rect[0], rect[1].min(150.0)],
                )
                .auto_select_all(true)
                .enter_returns_true(true)
                .build();

            committed
        }

        _ => state
            .ui
            .input_text(format!("{}##{}", name, id), &mut text)
            .auto_select_all(true)
            .enter_returns_true(true)
            .build(),
    };

    state
        .ui
        .columns(1, format!("##{}.{}-c1", tile.id, name), false);
    if let Ok(t) = text.parse::<T>() {
        if previous_text != text {
            window.state = EditorState::PropertyChanging;
            tile.clone().set(name, t);
            window.changed = true;
            window.request_quadtree_update();
        } else if window.state == EditorState::PropertyChanging && committed {
            window.state = EditorState::Idle;
            tile.clone().set(name, t);
            window.changed = true;
            window.request_quadtree_update();
        }
    }
}
