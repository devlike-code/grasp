use core::gui::imgui_keys::ExtraKeyEvents;
use core::gui::windowing::run_main_forever;
use editor_state::GraspEditorState;
use imgui::{Key, Ui};
use log::debug;

mod core;
mod editor_state;
mod editor_state_machine;
mod grasp_common;
mod grasp_context_menu;
mod grasp_editor_window;
mod grasp_editor_window_list;
mod grasp_queues;
mod grasp_render;
mod grasp_sense;
mod grasp_transitions;
mod grasp_update;
mod seq;
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

fn main() {
    let mut grasp_editor_state = GraspEditorState::new();

    run_main_forever(|ui, quit| {
        if ui.is_fkey_down(Key::F12) {
            grasp_editor_state.snapshot();
        }

        grasp_editor_state.process_requests();
        let gui = GuiState::new(ui);
        grasp_editor_state.show(&gui);

        //ui.show_demo_window(&mut true);

        *quit = *gui.quit.lock().unwrap();
    });
}
