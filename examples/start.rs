#![allow(dead_code)]

use anyhow::Result;
use salvation::{
    event_loop::{self, CallbackContext},
    widgets::{
        button::Button, column::Column, label::Label, padding_box::PaddingBox,
        text_input::TextInput, Widget, WidgetExt, WidgetId,
    },
    window::create_window,
};
use winit::window::WindowBuilder;

struct AnotherState {
    counter: i32,
}

impl AnotherState {
    fn new(ctx: &mut CallbackContext<Self>) -> (Self, Box<dyn Widget>) {
        let another_state = AnotherState { counter: 0 };
        let mut btn = Button::new("another button");
        btn.on_clicked(ctx.callback(|state, _ctx, _event| {
            state.counter += 1;
            println!("counter: {}", state.counter);
            create_window(
                WindowBuilder::new().with_title("example"),
                Some(Box::new(Label::new(format!("counter: {}", state.counter)))),
            );
            Ok(())
        }));
        (another_state, Box::new(btn))
    }
}

struct State {
    another_state: AnotherState,
    button_id: WidgetId<Button>,
    column2_id: WidgetId<Column>,
    button21_id: WidgetId<Button>,
    button22_id: WidgetId<Button>,
    flag_column: bool,
    flag_button21: bool,
}

impl State {
    fn new(ctx: &mut CallbackContext<Self>) -> Self {
        let mut root = Column::new();
        // let w1 =
        //     Image::load_png("1.png").unwrap();
        // root.add(
        //     Rect {
        //         top_left: Point { x: 20, y: 30 },
        //         size: Size { x: 300, y: 300 },
        //     },
        //     Box::new(w1),
        // );

        // let w2 = TextInput::new("Hello, Rust! 🦀 one two three four five\n");
        let w2 = TextInput::new("Hello, Rust! 🦀\n");
        root.add(Box::new(w2));
        let w3 = TextInput::new("Hebrew \nname Sarah: שרה, spelled");
        root.add(Box::new(w3));

        let mut btn1 = Button::new("btn1");
        let button_id = btn1.id();
        btn1.on_clicked(ctx.callback(|state, ctx, event| state.button_clicked2(ctx, event, 1)));
        root.add(Box::new(btn1));

        let mut btn2 = Button::new("btn2");
        // btn2.on_clicked(ctx.callback_maker.add(Self::button_clicked));
        btn2.on_clicked(ctx.callback(|state, ctx, event| state.button_clicked2(ctx, event, 2)));
        root.add(Box::new(btn2));

        let mut column2 = Column::new();
        let column2_id = column2.id();
        let mut button21 = Button::new("btn21");
        let button21_id = button21.id();
        button21.on_clicked(ctx.callback(|_, _, _| {
            println!("click!");
            Ok(())
        }));

        column2.add(Box::new(button21));
        let button22 = Button::new("btn22");
        let button22_id = button22.id();
        column2.add(Box::new(button22));

        root.add(Box::new(column2));

        let (another_state, btn3) =
            AnotherState::new(&mut ctx.map_state(|state| Some(&mut state.another_state)));
        root.add(btn3);

        create_window(
            WindowBuilder::new().with_title("example"),
            Some(Box::new(PaddingBox::new(Box::new(root)))),
            // Some(Box::new(root)),
        );
        State {
            another_state,
            button_id,
            column2_id,
            button21_id,
            button22_id,
            flag_column: true,
            flag_button21: true,
        }
    }

    // fn button_clicked(&mut self, _ctx: &mut CallbackContext<Self>, data: String) {
    //     println!("callback! {:?}", data);
    // }

    fn button_clicked2(
        &mut self,
        ctx: &mut CallbackContext<Self>,
        data: String,
        k: u32,
    ) -> Result<()> {
        println!("callback! {:?}, {}", data, k);
        let button = ctx.widget(self.button_id).unwrap();
        button.set_text(&format!("ok {}", if k == 1 { "1" } else { "22222" }));

        if k == 1 {
            self.flag_column = !self.flag_column;
            ctx.widget(self.column2_id)
                .unwrap()
                .set_enabled(self.flag_column);
            println!("set enabled {:?} {:?}", self.column2_id, self.flag_column);
        } else {
            self.flag_button21 = !self.flag_button21;
            ctx.widget(self.button21_id)
                .unwrap()
                .set_enabled(self.flag_button21);
            println!(
                "set enabled {:?} {:?}",
                self.button21_id, self.flag_button21
            );
        }
        Ok(())
    }
}

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    env_logger::init();
    event_loop::run(State::new).unwrap();
}
