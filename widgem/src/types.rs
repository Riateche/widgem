use {
    anyhow::Context,
    derive_more::{Add, AddAssign, From, Into, Neg, Sub, SubAssign, Sum},
    ordered_float::OrderedFloat,
    serde::{Deserialize, Serialize},
    std::{
        cmp::{max, min},
        iter::Sum,
        ops::{Add, Div, Mul, Neg, Sub, SubAssign},
    },
};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, From, Into, Default, Serialize, Deserialize,
)]
pub struct LogicalPixels(OrderedFloat<f32>);

impl LogicalPixels {
    pub const fn from_f32(value: f32) -> Self {
        Self(OrderedFloat(value))
    }

    pub const fn to_f32(self) -> f32 {
        self.0 .0
    }

    pub fn to_physical(self, scale: f32) -> PhysicalPixels {
        ((self.0 * scale).round() as i32).ppx()
    }
}

impl From<LogicalPixels> for f32 {
    fn from(value: LogicalPixels) -> Self {
        value.to_f32()
    }
}

impl From<f32> for LogicalPixels {
    fn from(value: f32) -> Self {
        Self::from_f32(value)
    }
}

impl Mul<f32> for LogicalPixels {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self(self.0 * rhs)
    }
}

pub trait LpxSuffix {
    fn lpx(self) -> LogicalPixels;
}

impl LpxSuffix for f32 {
    fn lpx(self) -> LogicalPixels {
        LogicalPixels::from_f32(self)
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    From,
    Into,
    Add,
    AddAssign,
    Sub,
    SubAssign,
    Sum,
    Neg,
)]
pub struct PhysicalPixels(i32);

impl PhysicalPixels {
    pub const ZERO: Self = Self(0);

    pub const fn from_i32(value: i32) -> Self {
        Self(value)
    }

    pub const fn to_i32(self) -> i32 {
        self.0
    }

    pub fn mul_f32_round(self, scale: f32) -> Self {
        Self(((self.0 as f32) * scale).round() as i32)
    }

    pub fn mul_f32_ceil(self, scale: f32) -> Self {
        Self(((self.0 as f32) * scale).ceil() as i32)
    }

    pub fn div_f32_round(self, scale: f32) -> Self {
        Self(((self.0 as f32) / scale).round() as i32)
    }
}

impl<'a> Sum<&'a PhysicalPixels> for PhysicalPixels {
    fn sum<I: Iterator<Item = &'a PhysicalPixels>>(iter: I) -> Self {
        iter.copied().sum()
    }
}

impl Mul<i32> for PhysicalPixels {
    type Output = PhysicalPixels;

    fn mul(self, rhs: i32) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Mul<PhysicalPixels> for i32 {
    type Output = PhysicalPixels;

    fn mul(self, rhs: PhysicalPixels) -> Self::Output {
        PhysicalPixels(self * rhs.0)
    }
}

impl Div<i32> for PhysicalPixels {
    type Output = Self;

    fn div(self, rhs: i32) -> Self::Output {
        Self(self.0 / rhs)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Point {
    x: PhysicalPixels,
    y: PhysicalPixels,
}

impl Point {
    pub const fn new(x: PhysicalPixels, y: PhysicalPixels) -> Self {
        Self { x, y }
    }

    pub fn x(&self) -> PhysicalPixels {
        self.x
    }

    pub fn y(&self) -> PhysicalPixels {
        self.y
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

impl Neg for Point {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Point::new(-self.x, -self.y)
    }
}

impl From<Point> for tiny_skia::Point {
    fn from(value: Point) -> Self {
        tiny_skia::Point::from_xy(value.x.0 as f32, value.y.0 as f32)
    }
}

impl From<Point> for winit::dpi::PhysicalPosition<i32> {
    fn from(value: Point) -> Self {
        winit::dpi::PhysicalPosition::new(value.x.to_i32(), value.y.to_i32())
    }
}

impl From<winit::dpi::PhysicalPosition<i32>> for Point {
    fn from(value: winit::dpi::PhysicalPosition<i32>) -> Self {
        Point::new(value.x.ppx(), value.y.ppx())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Size {
    x: PhysicalPixels,
    y: PhysicalPixels,
}

impl Size {
    pub const fn new(x: PhysicalPixels, y: PhysicalPixels) -> Self {
        Self { x, y }
    }

    pub fn x(&self) -> PhysicalPixels {
        self.x
    }

    pub fn y(&self) -> PhysicalPixels {
        self.y
    }
}

impl From<Size> for winit::dpi::PhysicalSize<u32> {
    fn from(value: Size) -> Self {
        winit::dpi::PhysicalSize::new(value.x.to_i32() as u32, value.y.to_i32() as u32)
    }
}

impl From<winit::dpi::PhysicalSize<u32>> for Size {
    fn from(value: winit::dpi::PhysicalSize<u32>) -> Self {
        Size::new(
            PhysicalPixels::from_i32(value.width as i32),
            PhysicalPixels::from_i32(value.height as i32),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    top_left: Point,
    size: Size,
}

impl Rect {
    pub const fn from_xywh(
        x: PhysicalPixels,
        y: PhysicalPixels,
        w: PhysicalPixels,
        h: PhysicalPixels,
    ) -> Rect {
        Self::from_pos_size(Point::new(x, y), Size::new(w, h))
    }

    pub const fn from_pos_size(top_left: Point, size: Size) -> Self {
        Self { top_left, size }
    }

    #[must_use]
    pub fn translate(&self, delta: Point) -> Self {
        Self {
            top_left: self.top_left + delta,
            size: self.size,
        }
    }

    // TODO: naming with "x" and "y" for all methods?

    pub fn top_left(&self) -> Point {
        self.top_left
    }

    /// Not inclusive.
    pub fn bottom_right(&self) -> Point {
        Point {
            x: self.top_left.x + self.size.x,
            y: self.top_left.y + self.size.y,
        }
    }

    pub fn left(&self) -> PhysicalPixels {
        self.top_left.x
    }

    pub fn right(&self) -> PhysicalPixels {
        self.top_left.x + self.size.x
    }

    pub fn top(&self) -> PhysicalPixels {
        self.top_left.y
    }

    pub fn bottom(&self) -> PhysicalPixels {
        self.top_left.y + self.size.y
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn size_y(&self) -> PhysicalPixels {
        self.size.y
    }

    pub fn size_x(&self) -> PhysicalPixels {
        self.size.x
    }

    pub fn is_empty(&self) -> bool {
        self.size.x == 0.ppx() || self.size.y == 0.ppx()
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
        if size.x < 0.ppx() || size.y < 0.ppx() {
            return Rect::default();
        }
        Self { top_left, size }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    X,
    Y,
}

impl TryFrom<Rect> for tiny_skia::Rect {
    type Error = anyhow::Error;

    fn try_from(value: Rect) -> Result<Self, Self::Error> {
        tiny_skia::Rect::from_xywh(
            value.top_left.x.to_i32() as f32,
            value.top_left.y.to_i32() as f32,
            value.size.x.to_i32() as f32,
            value.size.y.to_i32() as f32,
        )
        .with_context(|| format!("invalid rect: {value:?}"))
    }
}

impl From<Rect> for accesskit::Rect {
    fn from(rect: Rect) -> Self {
        accesskit::Rect {
            x0: rect.top_left.x.to_i32() as f64,
            y0: rect.top_left.y.to_i32() as f64,
            x1: rect.bottom_right().x.to_i32() as f64,
            y1: rect.bottom_right().y.to_i32() as f64,
        }
    }
}
