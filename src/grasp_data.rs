use std::sync::Arc;

use egui::{Vec2, vec2};
use grasp::internals::{Tile, EntityId, Mosaic, default_vals};

#[derive(Debug)]
pub struct GraspMetaData {
    pub mosaic: Arc<Mosaic>,
    pub graps_objects: Vec<GraspObject>
}

impl Default for GraspMetaData {
    fn default() -> Self {
        Self { mosaic: Mosaic::new(), graps_objects: Default::default()}
    }
}

impl GraspMetaData{
    pub fn add_tile_object(&mut self){
        let result: grasp::internals::Tile = self.mosaic.new_object("DEBUG", default_vals());
        let new_object: GraspObject = GraspObject::new(result);
        self.graps_objects.push(new_object);

       
    }
}

#[derive(Debug)]
pub struct GraspObject{
    pub tile_data : EntityId,
    pub position : Vec2,
    pub drag_start : Vec2,
    pub is_selected : bool,
}

impl GraspObject {
    pub fn new(tile : Tile) -> GraspObject{
        let position_default = vec2(100.0,100.0);
        let drag_start_default = vec2(0.0, 0.0);
        let is_selected_default = false;
        GraspObject { tile_data: tile.id, position: position_default, drag_start: drag_start_default, is_selected: is_selected_default }
    }
}