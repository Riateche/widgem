use salvation::{
    event_loop,
    types::{Point, Rect, Size},
    widgets::{image::Image, label::Label, stack::Stack},
};

fn main() {
    let mut root = Stack::new();
    let w1 =
        Image::load_png("/home/ri/tmp/rusttype/dev/tests/reference_big_biohazard.png").unwrap();
    root.add(
        Rect {
            top_left: Point { x: 20, y: 30 },
            size: Size { x: 300, y: 300 },
        },
        w1,
    );

    let w2 = Label::new("Hello, Rust! 🦀\n");
    root.add(
        Rect {
            top_left: Point { x: 100, y: 130 },
            size: Size { x: 300, y: 300 },
        },
        w2,
    );

    event_loop::run(root);
}
