use salvation::{
    event_loop,
    types::{Point, Rect, Size},
    widgets::{image::Image, label::Label, stack::Stack},
    WidgetContainer, WidgetInfo,
};

fn main() {
    let mut root = Stack::new();
    let w1 =
        Image::load_png("/home/ri/tmp/rusttype/dev/tests/reference_big_biohazard.png").unwrap();
    root.add(
        WidgetInfo {
            rect: Rect {
                top_left: Point { x: 20, y: 30 },
                size: Size {
                    width: 300,
                    height: 300,
                },
            },
        },
        w1,
    );

    let w2 = Label::new("Hello, Rust! 🦀\n");
    root.add(
        WidgetInfo {
            rect: Rect {
                top_left: Point { x: 100, y: 130 },
                size: Size {
                    width: 300,
                    height: 300,
                },
            },
        },
        w2,
    );

    let root = WidgetContainer {
        info: WidgetInfo {
            rect: Rect {
                top_left: Point { x: 0, y: 0 },
                size: Size {
                    width: 0,
                    height: 0,
                },
            },
        },
        widget: Box::new(root),
    };

    event_loop::run(root);
}
