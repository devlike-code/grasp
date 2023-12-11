use egui::Pos2;
use mosaic::{
    capabilities::ArchetypeSubject,
    internals::{Tile, TileFieldEmptyQuery, TileFieldQuery, Value},
};

pub struct Pos(pub Tile);

impl TileFieldEmptyQuery for Pos {
    type Output = Pos2;

    fn query(&self) -> Self::Output {
        if let Some(pos_component) = self.0.get_component("Position") {
            if let (Value::F32(x), Value::F32(y)) = pos_component.get_by(("x", "y")) {
                return Pos2::new(x, y);
            }
        }

        Pos2::ZERO
    }
}

pub struct Label(pub Tile);

impl TileFieldEmptyQuery for Label {
    type Output = String;

    fn query(&self) -> Self::Output {
        if let Some(pos_component) = self.0.get_component("Label") {
            if let Value::S32(s) = pos_component.get("self") {
                return s.to_string();
            }
        }

        "".to_string()
    }
}
