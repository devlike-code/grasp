use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use itertools::Itertools;
use mosaic::{
    capabilities::{ArchetypeSubject, QueueCapability, QueueTile},
    internals::{
        pars, void, ComponentValuesBuilderSetter, Mosaic, MosaicCRUD, MosaicIO,
        MosaicTypelevelCRUD, Tile, TileFieldEmptyQuery, S32,
    },
    iterators::{component_selectors::ComponentSelectors, tile_getters::TileGetters},
};
use quadtree_rs::Quadtree;

use crate::{
    core::math::Rect2,
    editor_state::{
        categories::ComponentEntry,
        helpers::{DisplayName, RequireWindowFocus},
        windows::GraspEditorWindow,
    },
    editor_state_machine::EditorState,
    grasp_editor_window_list::GraspEditorWindowList,
    grasp_render,
    utilities::Label,
};

use super::{
    categories::{ComponentCategory, Transformer},
    network::Networked,
    view::ComponentRenderer,
};

#[allow(dead_code)]
pub struct GraspEditorState {
    pub editor_mosaic: Arc<Mosaic>,
    pub component_mosaic: Arc<Mosaic>,
    pub component_renderers: HashMap<S32, ComponentRenderer>,
    pub window_list: GraspEditorWindowList,
    pub editor_state_tile: Tile,
    pub new_tab_request_queue: QueueTile,
    pub refresh_quadtree_queue: QueueTile,
    pub locked_components: Vec<S32>,
    pub show_tabview: bool,
    pub queued_component_delete: Option<usize>,
}

impl GraspEditorState {
    pub fn new() -> Self {
        let editor_mosaic = Mosaic::new();
        let mut component_mosaic = Mosaic::new();
        component_mosaic.initialize_networked();

        let (_, _) = Self::prepare_mosaic(
            &component_mosaic,
            &editor_mosaic,
            Arc::clone(&editor_mosaic),
        );

        let editor_state_tile = editor_mosaic.new_object("EditorState", void());

        let new_window_request_queue = editor_mosaic.make_queue();
        new_window_request_queue.add_component("NewWindowRequestQueue", void());

        let refresh_quadtree_queue = editor_mosaic.make_queue();
        refresh_quadtree_queue.add_component("QuadtreeUpdateRequestQueue", void());

        let close_window_request_queue = editor_mosaic.make_queue();
        close_window_request_queue.add_component("CloseWindowRequestQueue", void());

        let named_focus_window_request_queue = editor_mosaic.make_queue();
        named_focus_window_request_queue.add_component("NamedFocusWindowRequestQueue", void());

        // let types = editor_mosaic.component_registry.component_definitions.lock().unwrap();
        // dbg!(types);

        let mut instance = Self {
            component_renderers: HashMap::new(),
            editor_state_tile,
            new_tab_request_queue: new_window_request_queue,
            refresh_quadtree_queue,
            window_list: GraspEditorWindowList::new(&editor_mosaic),
            editor_mosaic,
            component_mosaic,
            show_tabview: false,
            queued_component_delete: None,
            locked_components: vec![
                "Node".into(),
                "Arrow".into(),
                "Position".into(),
                "Offset".into(),
            ],
        };

        instance.initialize_networked();
        instance
    }

    fn load_mosaic_transformers_from_file(
        component_mosaic: &Arc<Mosaic>,
        editor_mosaic: &Arc<Mosaic>,
        target_mosaic: &Arc<Mosaic>,
        file: PathBuf,
    ) -> Vec<Transformer> {
        // let mut transformers = vec![];
        let loader_mosaic = Mosaic::new();
        loader_mosaic.new_type("Node: unit;").unwrap();
        loader_mosaic.new_type("Arrow: unit;").unwrap();
        loader_mosaic.new_type("Label: s32;").unwrap();

        loader_mosaic.new_type("Hidden: unit;").unwrap();
        loader_mosaic.new_type("DisplayName: s32;").unwrap();

        loader_mosaic.load(&fs::read(file).unwrap()).unwrap();

        let trans_vec = loader_mosaic
            .get_all()
            .filter_map(|t| {
                if t.is_object()
                    && t.iter().get_arrows_into().len() == 0
                    && t.match_archetype(&["Label"])
                    && t.match_archetype(&["DisplayName"])
                {
                    Some(t)
                } else {
                    None
                }
            })
            .map(|t| Transformer {
                fn_name: Label(&t).query(),
                display: DisplayName(&t).query().unwrap(),
            })
            .collect_vec();

        let trans_tile_iter = loader_mosaic
            .get_all()
            .include_component("Transformers")
            .next();

        if let Some(trans_tile) = trans_tile_iter {
            trans_vec.iter().for_each(|entry| {
                target_mosaic.new_extension(
                    &trans_tile,
                    "Transformer",
                    pars()
                        .set("fn_name", entry.fn_name.as_str())
                        .set("display", entry.display.as_str())
                        // .set("hidden", entry.hidden)
                        .ok(),
                );
            });
        }

        trans_vec
    }

    fn load_mosaic_components_from_file(
        component_mosaic: &Arc<Mosaic>,
        editor_mosaic: &Arc<Mosaic>,
        target_mosaic: &Arc<Mosaic>,
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

            if menu.match_archetype(&["Hidden"]) {
                category.hidden = true;
            }

            let items = menu.iter().get_arrows_from().get_targets();

            for item in items {
                let component_name = Label(&item).query();
                assert_eq!(item.mosaic, loader_mosaic);

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

                target_mosaic.new_type(&formatted).unwrap();
                editor_mosaic.new_type(&formatted).unwrap();
            }

            component_categories.push(category);
        });

        component_categories.iter().for_each(|cat| {
            let cat_tile = component_mosaic.new_object(
                "ComponentCategory",
                pars()
                    .set("name", cat.name.as_str())
                    .set("hidden", cat.hidden)
                    .ok(),
            );

            cat.components.iter().for_each(|entry| {
                component_mosaic.new_extension(
                    &cat_tile,
                    "ComponentEntry",
                    pars()
                        .set("name", entry.name.as_str())
                        .set("display", entry.display.as_str())
                        .set("hidden", entry.hidden)
                        .ok(),
                );
            });
        });

        component_categories
    }

    pub fn prepare_mosaic(
        component_mosaic: &Arc<Mosaic>,
        editor_mosaic: &Arc<Mosaic>,
        mosaic: Arc<Mosaic>,
    ) -> (Arc<Mosaic>, Vec<ComponentCategory>) {
        assert_ne!(component_mosaic.id, editor_mosaic.id);
        component_mosaic
            .new_type("ComponentEntry: { name: s32, display: s32, hidden: bool };")
            .unwrap();
        component_mosaic
            .new_type("ComponentCategory: { name: s32, hidden: bool };")
            .unwrap();
        component_mosaic.new_type("Transformers: unit;").unwrap();
        component_mosaic.new_type("Transformer: unit;").unwrap();

        if let Some(trans) = component_mosaic
            .get_all()
            .include_component("Transformers")
            .next()
        {
            component_mosaic.delete_tile(trans);
        }

        component_mosaic.new_object("Transformers", void());

        let components: Vec<ComponentCategory> = fs::read_dir("env\\components")
            .unwrap()
            .flat_map(|file_entry| {
                if let Ok(file) = file_entry {
                    println!("Loading ComponentCategories{:?}", file);
                    Self::load_mosaic_components_from_file(
                        component_mosaic,
                        editor_mosaic,
                        &mosaic,
                        file.path(),
                    )
                } else {
                    vec![]
                }
            })
            .collect_vec();

        let transformers: Vec<Transformer> = fs::read_dir("env\\transformers")
            .unwrap()
            .flat_map(|file_entry| {
                if let Ok(file) = file_entry {
                    println!("Loading Transformers {:?}", file);
                    Self::load_mosaic_transformers_from_file(
                        component_mosaic,
                        editor_mosaic,
                        &mosaic,
                        file.path(),
                    )
                } else {
                    vec![]
                }
            })
            .collect_vec();

        println!("TRANSFORMERS {:?}", transformers);

        (mosaic, components)
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
        Self::prepare_mosaic(
            &self.component_mosaic,
            &self.editor_mosaic,
            Arc::clone(&document_mosaic),
        );
        assert!(self.editor_mosaic.id != document_mosaic.id);

        let filename = name.unwrap_or(format!("Untitled {}", new_index));
        let name = format!("[{}] {}", id, filename);

        let mut window = GraspEditorWindow {
            name: name.clone(),
            window_tile,
            quadtree: Mutex::new(Quadtree::new_with_anchor((-1000, -1000).into(), 16)),
            document_mosaic,
            component_mosaic: Arc::clone(&self.component_mosaic),
            editor_mosaic: Arc::clone(&self.editor_mosaic),
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
            window_list_index: id,
        };

        window.document_mosaic.initialize_networked();

        self.window_list.named_windows.push(name);
        self.window_list.windows.push_front(window);
    }

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
                self.require_named_window_focus(first);
            }
        }
    }
}
