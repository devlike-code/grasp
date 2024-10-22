use std::{
    collections::{HashMap, HashSet},
    fs::{self},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use itertools::Itertools;
use mosaic::{
    capabilities::{ArchetypeSubject, QueueCapability, QueueTile},
    internals::{
        par, pars, void, ComponentValuesBuilderSetter, Mosaic, MosaicCRUD, MosaicIO,
        MosaicTypelevelCRUD, Tile, S32,
    },
    iterators::{component_selectors::ComponentSelectors, tile_deletion::TileDeletion},
};
use quadtree_rs::Quadtree;
use select::deselect;
use serde::{Deserialize, Serialize};

use crate::{
    core::{
        gui::components::{setup_component_renderers, setup_file_importers},
        math::Rect2,
    },
    editor_state::{helpers::RequireWindowFocus, windows::GraspEditorWindow},
    editor_state_machine::EditorState,
    grasp_editor_window_list::GraspEditorWindowList,
    grasp_render,
    transformers::{finite_state_transformer, pattern_match_tool, select},
    GuiState,
};

use super::{
    categories::ComponentCategory,
    network::Networked,
    view::{ComponentPropertyRenderer, ComponentRenderer, DeleteReaction, FileImporter},
};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum TransformerState {
    #[allow(dead_code)]
    Running,
    Cancelled,
    Done,
}

pub type TransformerFn =
    Box<dyn Fn(&mut GraspEditorWindow, &GuiState, &[Tile], &Tile) -> TransformerState + 'static>;

pub struct Transformer {
    pub transform_function: TransformerFn,
}

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, PartialOrd)]
pub struct EditorOptions {
    pub toggle_tabview: bool,
    pub toggle_hierarchy: bool,
    pub toggle_properties: bool,
    pub toggle_errors: bool,
}

impl Default for EditorOptions {
    fn default() -> Self {
        let mut def = Self {
            toggle_tabview: false,
            toggle_hierarchy: false,
            toggle_properties: false,
            toggle_errors: false,
        };

        if let Ok(config_content) = fs::read_to_string("env\\config.ron") {
            if let Ok(config) = ron::from_str::<EditorOptions>(&config_content) {
                def = config;
            }
        }

        def
    }
}

impl EditorOptions {
    pub fn save(&self) {
        let _ = fs::write("env\\config.ron", ron::to_string(&self).unwrap().as_bytes());
    }
}

#[allow(dead_code)]
pub struct GraspEditorState {
    pub editor_mosaic: Arc<Mosaic>,
    pub component_mosaic: Arc<Mosaic>,
    pub transformer_mosaic: Arc<Mosaic>,
    pub file_importers: HashMap<String, FileImporter>,
    pub component_delete_reactions: HashMap<String, DeleteReaction>,
    pub component_entity_renderers: HashMap<String, ComponentRenderer>,
    pub hidden_property_renderers: HashSet<String>,
    pub component_property_renderers: HashMap<String, ComponentPropertyRenderer>,
    pub window_list: GraspEditorWindowList,
    pub editor_state_tile: Tile,
    pub new_tab_request_queue: QueueTile,
    pub refresh_quadtree_queue: QueueTile,
    pub locked_components: Vec<S32>,
    pub editor_options: EditorOptions,
    pub properties_hovered: bool,
    pub queued_component_delete: Option<usize>,
    pub transformer_functions: HashMap<String, Transformer>,
    pub pending_close_window_request: Option<Tile>,
    pub pending_transform_window_request: Option<Tile>,
}

impl GraspEditorState {
    fn add_transformer(&mut self, name: &str, f: TransformerFn) {
        self.transformer_functions.insert(
            name.to_string(),
            Transformer {
                transform_function: f,
            },
        );

        self.transformer_mosaic.new_object("Transformer", par(name));
    }

    fn load_transformers(&mut self) {
        self.add_transformer("[Selection] Create", Box::new(select));
        self.add_transformer("[Selection] Delete", Box::new(deselect));
        self.add_transformer("[Pattern] Match", Box::new(pattern_match_tool));
        self.add_transformer("[FSM] Compile", Box::new(finite_state_transformer));
    }

    pub fn new() -> Self {
        let editor_mosaic = Mosaic::new();
        let mut component_mosaic = Mosaic::new();
        component_mosaic.initialize_networked();

        let mut transformer_mosaic = Mosaic::new();
        transformer_mosaic.initialize_networked();

        let _ = Self::prepare_mosaic(
            &component_mosaic,
            &editor_mosaic,
            Arc::clone(&editor_mosaic),
        );

        let _ = Self::prepare_mosaic(
            &component_mosaic,
            &editor_mosaic,
            Arc::clone(&transformer_mosaic),
        );

        let editor_state_tile = editor_mosaic.new_object("EditorState", void());

        let window_transformer_queue = editor_mosaic.make_queue();
        window_transformer_queue.add_component("WindowTransformerQueue", void());

        let new_window_request_queue = editor_mosaic.make_queue();
        new_window_request_queue.add_component("NewWindowRequestQueue", void());

        let new_window_request_queue = editor_mosaic.make_queue();
        new_window_request_queue.add_component("WindowRenameRequestQueue", void());

        let window_tile_delete_reaction_request_queue = editor_mosaic.make_queue();
        window_tile_delete_reaction_request_queue
            .add_component("WindowTileDeleteReactionRequestQueue", void());

        let refresh_quadtree_queue = editor_mosaic.make_queue();
        refresh_quadtree_queue.add_component("QuadtreeUpdateRequestQueue", void());

        let close_window_request_queue = editor_mosaic.make_queue();
        close_window_request_queue.add_component("CloseWindowRequestQueue", void());

        let named_focus_window_request_queue = editor_mosaic.make_queue();
        named_focus_window_request_queue.add_component("NamedFocusWindowRequestQueue", void());

        let mut instance = Self {
            component_entity_renderers: HashMap::new(),
            component_property_renderers: HashMap::new(),
            component_delete_reactions: HashMap::new(),
            editor_state_tile,
            new_tab_request_queue: new_window_request_queue,
            refresh_quadtree_queue,
            file_importers: HashMap::default(),
            window_list: GraspEditorWindowList::new(&editor_mosaic),
            editor_mosaic,
            component_mosaic,
            transformer_mosaic,
            editor_options: EditorOptions::default(),
            properties_hovered: false,
            queued_component_delete: None,
            locked_components: vec![
                "Node".into(),
                "Arrow".into(),
                "Position".into(),
                "Offset".into(),
            ],
            transformer_functions: HashMap::new(),
            hidden_property_renderers: HashSet::new(),
            pending_close_window_request: None,
            pending_transform_window_request: None,
        };

        setup_component_renderers(&mut instance);
        setup_file_importers(&mut instance);

        instance.initialize_networked();
        instance.load_transformers();
        instance
    }

    fn load_mosaic_components_from_file(
        component_mosaic: &Arc<Mosaic>,
        editor_mosaic: &Arc<Mosaic>,
        target_mosaic: &Arc<Mosaic>,
        categories: &Vec<ComponentCategory>,
    ) {
        for category in categories {
            component_mosaic
                .get_all()
                .include_component("ComponentCategory")
                .filter(|t| t.get("name").as_s32().to_string() == category.name)
                .delete();

            let cat_tile = component_mosaic.new_object(
                "ComponentCategory",
                pars()
                    .set("name", category.name.as_str())
                    .set("hidden", category.hidden)
                    .ok(),
            );

            for item in &category.components {
                let component_name = item.split(':').collect_vec()[0];

                target_mosaic.new_type(item).unwrap();
                editor_mosaic.new_type(item).unwrap();

                println!("{:?}", item);
                component_mosaic.new_extension(
                    &cat_tile,
                    "ComponentEntry",
                    pars()
                        .set("name", component_name)
                        .set("definition", item.clone())
                        .ok(),
                );
            }
        }
    }

    pub fn prepare_mosaic(
        component_mosaic: &Arc<Mosaic>,
        editor_mosaic: &Arc<Mosaic>,
        mosaic: Arc<Mosaic>,
    ) -> (Arc<Mosaic>, Vec<ComponentCategory>) {
        println!("Loading mosaic");
        assert_ne!(component_mosaic.id, editor_mosaic.id);
        component_mosaic
            .new_type("Error: { message: str, target: u64, window: u64 };")
            .unwrap();

        component_mosaic
            .new_type("ComponentCategory: { name: s32, hidden: bool };")
            .unwrap();

        component_mosaic
            .new_type("ComponentEntry: { name: s32, definition: str };")
            .unwrap();

        let components: Vec<ComponentCategory> = fs::read_dir("env\\components")
            .unwrap()
            .flat_map(|file_entry| {
                if let Ok(file) = file_entry {
                    if let Ok(contents) = fs::read_to_string(file.path()) {
                        let parsing = ron::from_str::<Vec<ComponentCategory>>(contents.as_str());
                        if let Ok(parsed) = parsing {
                            Self::load_mosaic_components_from_file(
                                component_mosaic,
                                editor_mosaic,
                                &mosaic,
                                &parsed,
                            );
                            vec![]
                        } else {
                            println!("{:?}", parsing.clone().unwrap_err().to_string());
                            editor_mosaic.new_object(
                                "Error",
                                pars()
                                    .set("message", parsing.unwrap_err().to_string())
                                    .set("target", 0)
                                    .set("window", 0)
                                    .ok(),
                            );

                            vec![]
                        }
                    } else {
                        editor_mosaic.new_object(
                            "Error",
                            pars()
                                .set(
                                    "message",
                                    format!(
                                        "Couldn't open {} components configuration file.",
                                        file.file_name().to_str().unwrap()
                                    ),
                                )
                                .set("target", 0)
                                .set("window", 0)
                                .ok(),
                        );
                        vec![]
                    }
                } else {
                    vec![]
                }
            })
            .collect_vec();

        (mosaic, components)
    }

    pub fn new_window(&mut self, path: Option<&PathBuf>) {
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

        let filename = path
            .and_then(|p| {
                p.file_name()
                    .and_then(|os| os.to_str().map(|s| s.to_string()))
            })
            .unwrap_or(format!("Untitled {}", new_index));
        let name = format!("[{}] {}", id, filename);

        let mut window = GraspEditorWindow {
            name: name.clone(),
            path: path.cloned(),
            window_tile,
            changed: false,
            quadtree: Mutex::new(Quadtree::new_with_anchor((-1000, -1000).into(), 16)),
            document_mosaic,
            component_mosaic: Arc::clone(&self.component_mosaic),
            editor_mosaic: Arc::clone(&self.editor_mosaic),
            transformer_mosaic: Arc::clone(&self.transformer_mosaic),
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
