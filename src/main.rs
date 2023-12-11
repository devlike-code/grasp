use editor_state::GraspEditorState;
use grasp_common::create_native_options;

mod editor_queue_process;
mod editor_state;
mod editor_state_machine;
mod grasp_common;
mod grasp_context_menu;
mod grasp_render;
mod grasp_sense;
mod grasp_transitions;
mod grasp_update;

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
