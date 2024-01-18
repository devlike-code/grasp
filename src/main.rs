use core::gui::imgui_keys::ExtraKeyEvents;
use core::gui::windowing::run_main_forever;
use editor_state::foundation::GraspEditorState;
use imgui::{Key, Ui};

mod core;
mod editor_state;
mod editor_state_machine;
mod grasp_common;
mod grasp_context_menu;
mod grasp_editor_window_list;
mod grasp_queues;
mod grasp_render;
mod grasp_transitions;
mod grasp_update;
mod querying;
mod seq;
mod transformers;
mod utilities;

use std::{ops::Deref, sync::Mutex};

pub struct GuiState<'a> {
    ui: &'a Ui,
    quit: Mutex<bool>,
}

impl<'a> GuiState<'a> {
    pub fn new(ui: &'a Ui) -> Self {
        GuiState {
            ui,
            quit: Mutex::new(false),
        }
    }

    pub fn exit(&self) {
        *self.quit.lock().unwrap() = true;
    }
}

impl<'a> Deref for GuiState<'a> {
    type Target = Ui;

    fn deref(&self) -> &Self::Target {
        self.ui
    }
}

#[tokio::main]
async fn main() {
    let mut grasp_editor_state = GraspEditorState::new();

    run_main_forever(|ui, quit| {
        if ui.is_fkey_down(Key::F11) {
            grasp_editor_state.snapshot_all("SNAPSHOT");
        }

        if ui.is_fkey_down(Key::F12) {
            grasp_editor_state.update_snapshot_all("SNAPSHOT");
        }

        grasp_editor_state.process_requests();
        let gui = GuiState::new(ui);
        grasp_editor_state.show(&gui);

        //        ui.show_demo_window(&mut true);

        *quit = *gui.quit.lock().unwrap();
    });
}
