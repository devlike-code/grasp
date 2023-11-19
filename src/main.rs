mod pane;
mod editor;
mod grasp;
mod grasp_data;
mod tile_manager;

use grasp::create_grasp_editor;

fn main() -> Result<(), eframe::Error> {
    env_logger::init();
    create_grasp_editor()
}