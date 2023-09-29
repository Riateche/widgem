use derive_more::{From, Into};
use std::{
    cmp::{max, min},
    ops::{Add, Sub, SubAssign},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From, Into, Default)]
pub struct LogicalPixels(i32);

impl LogicalPixels {
    pub fn get(self) -> i32 {
        self.0
    }

    pub fn to_physical(self, scale: f32) -> PhysicalPixels {
        ((self.0 as f32 * scale).round() as i32).ppx()
    }
}

pub trait LpxSuffix {
    fn lpx(self) -> LogicalPixels;
}

impl LpxSuffix for i32 {
    fn lpx(self) -> LogicalPixels {
        LogicalPixels(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From, Into)]
pub struct PhysicalPixels(i32);

impl PhysicalPixels {
    pub fn get(self) -> i32 {
        self.0
    }
}

pub trait PpxSuffix {
    fn ppx(self) -> PhysicalPixels;
}

impl PpxSuffix for i32 {
    fn ppx(self) -> PhysicalPixels {
        PhysicalPixels(self)
    }
}

// TODO: use PhysicalPixels?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

impl Add for Point {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}
impl Sub for Point {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}
impl SubAssign for Point {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Size {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub top_left: Point,
    pub size: Size,
}

impl Rect {
    #[must_use]
    pub fn translate(&self, delta: Point) -> Self {
        Self {
            top_left: self.top_left + delta,
            size: self.size,
        }
    }

    /// Not inclusive.
    pub fn bottom_right(&self) -> Point {
        Point {
            x: self.top_left.x + self.size.x,
            y: self.top_left.y + self.size.y,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.size.x == 0 || self.size.y == 0
    }

    pub fn contains(&self, pos: Point) -> bool {
        let br = self.bottom_right();
        self.top_left.x <= pos.x && pos.x < br.x && self.top_left.y <= pos.y && pos.y < br.y
    }

    pub fn intersect(&self, other: Self) -> Self {
        let top_left = Point {
            x: max(self.top_left.x, other.top_left.x),
            y: max(self.top_left.y, other.top_left.y),
        };
        let br1 = self.bottom_right();
        let br2 = other.bottom_right();
        let bottom_right = Point {
            x: min(br1.x, br2.x),
            y: min(br1.y, br2.y),
        };
        let size = Size {
            x: bottom_right.x - top_left.x,
            y: bottom_right.y - top_left.y,
        };
        if size.x < 0 || size.y < 0 {
            return Rect::default();
        }
        Self { top_left, size }
    }
}
