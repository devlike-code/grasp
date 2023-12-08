use egui::Pos2;
use mosaic::{
    capabilities::ArchetypeSubject,
    internals::{Tile, TileFieldQuery, Value},
};
pub struct Pos(pub Tile);

impl TileFieldQuery<()> for Pos {
    type Output = Pos2;

    fn get_by(&self, index: ()) -> Self::Output {
        if let Some(pos_component) = self.0.get_component("Position") {
            if let (Value::F32(x), Value::F32(y)) = pos_component.get_by(("x", "y")) {
                return Pos2::new(x, y);
            }
        }

        Pos2::ZERO
    }
}
