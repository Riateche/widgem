#[derive(Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Default)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

#[derive(Default)]
pub struct Rect {
    pub top_left: Point,
    pub size: Size,
}
