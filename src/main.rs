use editor_state::GraspEditorState;
use grasp::create_native_options;

mod editor_state;
mod editor_state_machine;
mod grasp;

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
