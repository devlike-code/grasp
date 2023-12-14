use crate::math::vec2::Vec2;

#[derive(Clone, Copy, Debug, Default)]
pub struct Rect2 {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect2 {
    pub fn from_two_pos(a: Vec2, b: Vec2) -> Rect2 {
        let min_x = a.x.min(b.x);
        let min_y = a.y.min(b.y);
        let max_x = a.x.max(b.x);
        let max_y = a.y.max(b.y);
        let width = max_x - min_x;
        let height = max_y - min_y;

        Rect2 { x: min_x, y: min_y, width, height }
    }

    pub fn min(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    pub fn max(&self) -> Vec2 {
        Vec2::new(self.x + self.width, self.y + self.height)
    }
}