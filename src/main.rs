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
use quadtree_rs::{
    area::{Area, AreaBuilder},
    point::Point,
    Quadtree,
};
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
    fn pos_into_editor(&self, v: Pos2) -> Pos2 {
        v.add(self.editor_data.pan)
            .add(self.editor_data.tab_offset.to_vec2())
    }

    fn pos_from_editor(&self, v: Pos2) -> Pos2 {
        v.sub(self.editor_data.pan)
            .sub(self.editor_data.tab_offset.to_vec2())
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
            painter.circle_filled(self.pos_into_editor(pos), 5.0, Color32::WHITE);

            // Maybe draw label
            if let Some(label) = node
                .into_iter()
                .get_descriptors()
                .include_component("Label")
                .next()
            {
                painter.text(
                    self.pos_into_editor(pos.add(Vec2::new(10.0, 10.0))),
                    Align2::LEFT_CENTER,
                    label.get("self").as_s32().to_string(),
                    FontId::default(),
                    Color32::WHITE,
                );
            }
        }

        for arrow in self
            .document_mosaic
            .get_all()
            .filter_arrows()
            .include_component("Position")
        {
            // painter.arrow(
            //     Pos2::new(200.0, 200.0),
            //     Vec2::new(100.0, 100.0),
            //     Stroke::new(1.0, Color32::WHITE),
            // );
        }
    }

    pub fn sense(&mut self, ui: &mut Ui) {
        let (resp, _) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());

        if let Some(pos) = resp.hover_pos() {
            self.editor_data.cursor = self.pos_from_editor(pos);
        }

        let result = self
            .quadtree
            .query(build_area(self.editor_data.cursor))
            .collect_vec();

        if resp.double_clicked() && result.is_empty() {
            self.trigger(EditorStateTrigger::DblClickToCreate);
        } else if resp.drag_started_by(egui::PointerButton::Primary) && !result.is_empty() {
            let entity = self
                .document_mosaic
                .get(*result.first().unwrap().value_ref())
                .unwrap();
            self.editor_data.selected = vec![entity];
            self.trigger(EditorStateTrigger::DragToMove);
        } else if resp.drag_started_by(egui::PointerButton::Primary) && result.is_empty() {
            self.editor_data.selected = vec![];
            self.trigger(EditorStateTrigger::DragToSelect);
        } else if resp.drag_started_by(egui::PointerButton::Secondary) {
            self.trigger(EditorStateTrigger::DragToPan);
        }
    }

    pub fn update(&mut self, ui: &mut Ui) {
        match &self.state {
            EditorState::Idle => {}
            EditorState::Move => {
                let pos = self.editor_data.cursor;
                for tile in &mut self.editor_data.selected {
                    tile.set("x", pos.x);
                    tile.set("y", pos.y);
                }
            }
            EditorState::Pan => todo!(),
            EditorState::Link => todo!(),
            EditorState::Rect => todo!(),
        }
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

    fn on_transition(&mut self, from: Self::State, trigger: Self::Trigger) -> Option<EditorState> {
        println!("{:?} {:?}", from, trigger);
        match (from, trigger) {
            (_, EditorStateTrigger::DblClickToCreate) => {
                self.create_new_object(self.editor_data.cursor);
                Some(EditorState::Idle)
            }

            (_, EditorStateTrigger::MouseDownOverNode) => None,
            (_, EditorStateTrigger::ClickToSelect) => None,
            (_, EditorStateTrigger::ClickToDeselect) => None,
            (EditorState::Idle, EditorStateTrigger::DragToPan) => None,
            (EditorState::Idle, EditorStateTrigger::DragToLink) => None,
            (EditorState::Idle, EditorStateTrigger::DragToMove) => None,
            (EditorState::Idle, EditorStateTrigger::DragToSelect) => None,
            (EditorState::Pan, EditorStateTrigger::EndDrag) => None,
            (EditorState::Link, EditorStateTrigger::EndDrag) => None,
            (EditorState::Move, EditorStateTrigger::EndDrag) => None,
            (EditorState::Rect, EditorStateTrigger::EndDrag) => None,
            _ => None,
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

fn build_area(pos: Pos2) -> Area<i32> {
    AreaBuilder::default()
        .anchor((pos.x as i32, pos.y as i32).into())
        .dimensions((1, 1))
        .build()
        .unwrap()
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
        tab.sense(ui);
        tab.update(ui);
    }
}

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
