use std::ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign};

use imgui::sys::ImVec2;

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl From<ImVec2> for Vec2 {
    fn from(value: ImVec2) -> Self {
        Vec2::new(value.x, value.y)
    }
}

impl From<Vec2> for ImVec2 {
    fn from(value: Vec2) -> Self {
        ImVec2::new(value.x, value.y)
    }
}

impl From<[f32; 2]> for Vec2 {
    fn from(value: [f32; 2]) -> Self {
        Vec2 {
            x: value[0],
            y: value[1],
        }
    }
}

impl From<Vec2> for [f32; 2] {
    fn from(value: Vec2) -> Self {
        [value.x, value.y]
    }
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };
    pub fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
    }

    pub fn len(&self) -> f32 {
        self.len_sqr().sqrt()
    }

    pub fn len_sqr(&self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    pub fn normalized(&self) -> Vec2 {
        let len = (self.x * self.x + self.y * self.y).sqrt();
        if len < f32::EPSILON {
            Default::default()
        } else {
            Vec2::new(self.x / len, self.y / len)
        }
    }

    pub fn lerp(&self, other: Vec2, t: f32) -> Vec2 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        Vec2::new(self.x + dx * t, self.y + dy * t)
    }

    pub fn distance(&self, other: Vec2) -> f32 {
        let diff = *self - other;
        diff.len()
    }
}

impl std::ops::Neg for Vec2 {
    type Output = Vec2;

    fn neg(self) -> Self::Output {
        -1.0 * self
    }
}
impl Add<Vec2> for Vec2 {
    type Output = Vec2;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl AddAssign<Vec2> for Vec2 {
    fn add_assign(&mut self, rhs: Vec2) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub<Vec2> for Vec2 {
    type Output = Vec2;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl SubAssign<Vec2> for Vec2 {
    fn sub_assign(&mut self, rhs: Vec2) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Mul<Vec2> for f32 {
    type Output = Vec2;

    fn mul(self, rhs: Vec2) -> Self::Output {
        Vec2 {
            x: self * rhs.x,
            y: self * rhs.y,
        }
    }
}

impl Mul<f32> for Vec2 {
    type Output = Vec2;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl MulAssign<f32> for Vec2 {
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl Mul<Vec2> for Vec2 {
    type Output = f32;

    fn mul(self, rhs: Vec2) -> Self::Output {
        self.x * rhs.x + self.y * rhs.y
    }
}
