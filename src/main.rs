use ::grasp::{
    internals::{
        self_val, EntityId, Mosaic, MosaicCRUD, MosaicIO, MosaicTypelevelCRUD, Tile,
        TileFieldGetter, TileFieldSetter, Value,
    },
    iterators::{
        component_selectors::ComponentSelectors, tile_filters::TileFilters,
        tile_getters::TileGetters,
    },
};
use editor_state_machine::{EditorState, EditorStateTrigger, StateMachine};
use egui::Pos2;
use egui::{ahash::HashMap, Align2, Color32, FontId, Sense, Ui, Vec2, WidgetText};
use egui_dock::{DockArea, DockState, Style, TabViewer};
use grasp::create_native_options;
use itertools::Itertools;
use quadtree_rs::{area::AreaBuilder, point::Point, Quadtree};
use std::{
    ops::{Add, Sub},
    sync::Arc,
};
mod editor_state_machine;
mod grasp;

#[derive(Default)]
pub struct GraspEditorData {
    pub pan: Vec2,
    pub selected: Vec<Tile>,
    pub cursor: Pos2,
    pub tab_offset: Pos2,
}

pub struct GraspEditorTab {
    pub name: String,
    pub state: EditorState,
    pub quadtree: Quadtree<i32, EntityId>,
    pub document_mosaic: Arc<Mosaic>,
    pub node_area: HashMap<EntityId, u64>,
    pub editor_data: GraspEditorData,
}

impl GraspEditorTab {
    fn offset(&self, v: Pos2) -> Pos2 {
        v.add(self.editor_data.pan)
            .add(self.editor_data.tab_offset.to_vec2())
    }

    pub fn render(&mut self, ui: &mut Ui) {
        let painter = ui.painter();

        // Rendering

        for node in self
            .document_mosaic
            .get_all()
            .filter_objects()
            .include_component("Position")
        {
            // Draw node
            let pos = Pos2::new(node.get("x").as_f32(), node.get("y").as_f32());
            painter.circle_filled(self.offset(pos), 5.0, Color32::WHITE);

            // Maybe draw label
            if let Some(label) = node
                .into_iter()
                .get_descriptors()
                .include_component("Label")
                .next()
            {
                painter.text(
                    self.offset(pos.add(Vec2::new(10.0, 10.0))),
                    Align2::LEFT_CENTER,
                    label.get("self").as_s32().to_string(),
                    FontId::default(),
                    Color32::WHITE,
                );
            }
        }

        // TODO: render arrows between nodes

        // painter.arrow(
        //     Pos2::new(200.0, 200.0),
        //     Vec2::new(100.0, 100.0),
        //     Stroke::new(1.0, Color32::WHITE),
        // );

        // for arrow in tab
        //     .document_mosaic
        //     .get_all()
        //     .filter_arrows()
        //     .include_component("Position")
        // {
        //     println!("{:?}", arrow);
        // }
    }
}

impl GraspEditorTab {
    pub fn create_new_object(&mut self, pos: Pos2) {
        let obj = self.document_mosaic.new_object(
            "Position",
            vec![
                ("x".into(), Value::F32(pos.x)),
                ("y".into(), Value::F32(pos.y)),
            ],
        );
        self.document_mosaic
            .new_descriptor(&obj, "Label", self_val(Value::S32("Label!".into())));

        let region_a = AreaBuilder::default()
            .anchor(Point {
                x: pos.x as i32 - 5,
                y: pos.y as i32 - 5,
            })
            .dimensions((10, 10))
            .build()
            .unwrap();

        if let Some(area_id) = self.quadtree.insert(region_a, obj.id) {
            self.node_area.insert(obj.id, area_id);
        }
    }
}

impl StateMachine for GraspEditorTab {
    type Trigger = EditorStateTrigger;
    type State = EditorState;

    fn trigger(&self, trigger: EditorStateTrigger) -> Option<EditorState> {
        match (self.state, trigger) {
            (EditorState::Idle, EditorStateTrigger::DblClickToCreate) => Some(EditorState::Idle),
            (EditorState::Idle, EditorStateTrigger::MouseDownOverNode) => Some(EditorState::Idle),
            (EditorState::Idle, EditorStateTrigger::ClickToSelect) => Some(EditorState::Idle),
            (EditorState::Idle, EditorStateTrigger::ClickToDeselect) => Some(EditorState::Idle),
            (EditorState::Idle, EditorStateTrigger::DragToPan) => Some(EditorState::Pan),
            (EditorState::Idle, EditorStateTrigger::DragToLink) => Some(EditorState::Link),
            (EditorState::Idle, EditorStateTrigger::DragToMove) => Some(EditorState::Move),
            (EditorState::Idle, EditorStateTrigger::DragToSelect) => Some(EditorState::Rect),
            (EditorState::Pan, EditorStateTrigger::EndDrag) => Some(EditorState::Idle),
            (EditorState::Link, EditorStateTrigger::EndDrag) => Some(EditorState::Idle),
            (EditorState::Move, EditorStateTrigger::EndDrag) => Some(EditorState::Idle),
            (EditorState::Rect, EditorStateTrigger::EndDrag) => Some(EditorState::Idle),

            _ => None,
        }
    }

    fn on_transition(&mut self, from: Self::State, trigger: Self::Trigger, next: Self::State) {
        match (from, trigger, next) {
            (_, EditorStateTrigger::DblClickToCreate, _) => {
                self.create_new_object(self.editor_data.cursor);
            }
            (_, EditorStateTrigger::MouseDownOverNode, _) => {}
            (_, EditorStateTrigger::ClickToSelect, _) => {}
            (_, EditorStateTrigger::ClickToDeselect, _) => {}
            (EditorState::Idle, EditorStateTrigger::DragToPan, EditorState::Pan) => {}
            (EditorState::Idle, EditorStateTrigger::DragToLink, EditorState::Link) => {}
            (EditorState::Idle, EditorStateTrigger::DragToMove, EditorState::Move) => {}
            (EditorState::Idle, EditorStateTrigger::DragToSelect, EditorState::Rect) => {}
            (EditorState::Pan, EditorStateTrigger::EndDrag, _) => {}
            (EditorState::Link, EditorStateTrigger::EndDrag, _) => {}
            (EditorState::Move, EditorStateTrigger::EndDrag, _) => {}
            (EditorState::Rect, EditorStateTrigger::EndDrag, _) => {}
            _ => {}
        }
    }

    fn get_current_state(&self) -> Self::State {
        self.state
    }
    fn move_to(&mut self, next: Self::State) {
        self.state = next;
    }
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
        tab.editor_data.tab_offset = xy;
        tab.render(ui);

        // Sense

        let (resp, _) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        // TODO: check against quadtree to see whether we're selecting or deselecting

        if let Some(pos) = resp.hover_pos() {
            tab.editor_data.cursor = pos.sub(tab.editor_data.pan).sub(xy.to_vec2());
        }

        let mouse_region = AreaBuilder::default()
            .anchor(
                (
                    tab.editor_data.cursor.x as i32,
                    tab.editor_data.cursor.y as i32,
                )
                    .into(),
            )
            .dimensions((1, 1))
            .build()
            .unwrap();

        let result = tab.quadtree.query(mouse_region);

        if resp.double_clicked() {
            if result.count() == 0 {
                tab.trigger(EditorStateTrigger::DblClickToCreate);
            }
        }

        ui.input(|i| {
            if resp.dragged_by(egui::PointerButton::Primary) {
                let mouse_pos = resp.interact_pointer_pos().unwrap();
                let region_c = AreaBuilder::default()
                    .anchor((mouse_pos.x as i32, mouse_pos.y as i32).into())
                    .dimensions((1, 1))
                    .build()
                    .unwrap();

                let mut to_remove = vec![];
                let result = tab.quadtree.query(region_c).collect_vec();

                if let Some(entry) = result.first() {
                    let entity_id = entry.value_ref();

                    println!("Mosaic: {:?}", tab.document_mosaic.get_all());
                    if let Some(mut tile) = tab.document_mosaic.get(*entity_id) {
                        println!("Selected tile: {:?}", tile);
                        let relative_pos = mouse_pos.sub(tab.editor_data.pan).sub(xy);
                        println!("Mouse position:{:?} ", mouse_pos);
                        tile.set("x", relative_pos.x);
                        tile.set("y", relative_pos.y);
                        println!("Relative Mouse position:{:?} ", relative_pos);

                        if let Some(area_id) = tab.node_area.get(&tile.id) {
                            if let Some(entry) = tab.quadtree.get(*area_id) {
                                println!("Area anchor: {:?}", entry.area().anchor());
                                let anchor = entry.area();
                                to_remove.push(anchor);
                            }
                        }
                    }
                    println!("{:?}", result);
                }

                for rem in to_remove {
                    tab.quadtree.delete(rem);
                }

                if i.modifiers.alt {}
            }
        })

        // TODO: create new sense painter to check for drag _if_ there were no clicks, to check for pan/move
    }
}

// Here is a simple example of how you can manage a `DockState` of your application.
struct GraspEditorState {
    document_mosaic: Arc<Mosaic>,
    tabs: GraspEditorTabs,
    dock_state: DockState<GraspEditorTab>,
}

impl GraspEditorState {
    pub fn new() -> Self {
        let document_mosaic = Mosaic::new();
        document_mosaic.new_type("Label: s32;").unwrap();
        document_mosaic
            .new_type("Position: { x: f32, y: f32 };")
            .unwrap();
        document_mosaic.new_type("Selection: unit;").unwrap();

        let dock_state = DockState::new(vec![]);
        let mut state = Self {
            document_mosaic,
            dock_state,
            tabs: GraspEditorTabs::default(),
        };

        let tab = state.new_tab();
        state.dock_state.main_surface_mut().push_to_first_leaf(tab);

        state
    }

    pub fn new_tab(&mut self) -> GraspEditorTab {
        GraspEditorTab {
            name: format!("Untitled {}", self.tabs.increment()),
            quadtree: Quadtree::new(16),
            document_mosaic: Arc::clone(&self.document_mosaic),
            node_area: Default::default(),
            editor_data: Default::default(),
            state: EditorState::Idle,
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
