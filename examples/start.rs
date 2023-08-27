use salvation::{
    event_loop::{self, CallbackContext},
    types::{Point, Rect, Size},
    widgets::{button::Button, image::Image, label::Label, stack::Stack},
};

struct State {}

impl State {
    fn new(ctx: &mut CallbackContext<Self>) -> Self {
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

        let mut btn1 = Button::new("btn1");
        btn1.on_clicked(ctx.callback_maker.add(|state, ctx, event| {
            state.button_clicked2(ctx, event, 1);
        }));
        root.add(
            Rect {
                top_left: Point { x: 20, y: 200 },
                size: Size { x: 200, y: 50 },
            },
            btn1,
        );

        let mut btn2 = Button::new("btn2");
        // btn2.on_clicked(ctx.callback_maker.add(Self::button_clicked));
        btn2.on_clicked(ctx.callback_maker.add(|state, ctx, event| {
            state.button_clicked2(ctx, event, 2);
        }));
        root.add(
            Rect {
                top_left: Point { x: 20, y: 260 },
                size: Size { x: 200, y: 50 },
            },
            btn2,
        );
        ctx.window.set_widget(Some(root));
        State {}
    }

    // fn button_clicked(&mut self, _ctx: &mut CallbackContext<Self>, data: String) {
    //     println!("callback! {:?}", data);
    // }

    fn button_clicked2(&mut self, _ctx: &mut CallbackContext<Self>, data: String, k: u32) {
        println!("callback! {:?}, {}", data, k);
    }
}

fn main() {
    event_loop::run(State::new);
}
