#![allow(dead_code)]

use std::time::Duration;

use anyhow::Result;
use salvation::{
    event_loop::{self, CallbackContext},
    system::add_interval,
    types::Axis,
    widgets::{
        button::Button, column::Column, label::Label, padding_box::PaddingBox,
        scroll_bar::ScrollBar, text_input::TextInput, Widget, WidgetExt, WidgetId,
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
                Some(Label::new(format!("counter: {}", state.counter)).boxed()),
            );
            Ok(())
        }));
        (another_state, btn.boxed())
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
    i: i32,
    label2_id: WidgetId<Label>,
}

impl State {
    fn new(ctx: &mut CallbackContext<Self>) -> Self {
        let mut root = Column::new();

        root.add_child(
            TextInput::new("Hello, Rust! 🦀\n").boxed(),
            Default::default(),
        );
        root.add_child(
            TextInput::new("Hebrew name Sarah: שרה.").boxed(),
            Default::default(),
        );

        /*
        let btn = Button::new("btn1")
            .with_icon(icon)
            .with_alignment(Al::Right)
            .with_on_clicked(slot)
            .split_id()
            .boxed();
        root.add(btn.widget);

        Self {
            btn_id: btn.id,
        }


        */

        let btn1 = Button::new("btn1")
            .with_on_clicked(ctx.callback(|state, ctx, event| state.button_clicked(ctx, event, 1)))
            .split_id();

        root.add_child(btn1.widget.boxed(), Default::default());

        root.add_child(
            Button::new("btn2")
                .with_on_clicked(
                    ctx.callback(|state, ctx, event| state.button_clicked(ctx, event, 2)),
                )
                .boxed(),
            Default::default(),
        );

        let button21 = Button::new("btn21")
            .with_on_clicked(ctx.callback(|_, _, _| {
                println!("click!");
                Ok(())
            }))
            .split_id();

        let button22 = Button::new("btn22").split_id();

        let column2 = Column::new()
            .with_child(button21.widget.boxed(), Default::default())
            .with_child(button22.widget.boxed(), Default::default())
            .split_id();
        root.add_child(column2.widget.boxed(), Default::default());

        let (another_state, btn3) =
            AnotherState::new(&mut ctx.map_state(|state| Some(&mut state.another_state)));
        root.add_child(btn3, Default::default());

        root.add_child(
            ScrollBar::new()
                .with_axis(Axis::Y)
                .with_on_value_changed(ctx.callback(|this, ctx, value| {
                    ctx.widget(this.label2_id)?
                        .set_text(format!("value={value}"));
                    Ok(())
                }))
                .boxed(),
            Default::default(),
        );
        let label2 = Label::new("ok").split_id();
        root.add_child(label2.widget.boxed(), Default::default());

        create_window(
            WindowBuilder::new().with_title("example"),
            Some(PaddingBox::new(root.boxed(), Default::default()).boxed()),
        );
        add_interval(
            Duration::from_secs(2),
            ctx.callback(|this, ctx, _| this.inc(ctx)),
        );
        State {
            another_state,
            button_id: btn1.id,
            column2_id: column2.id,
            button21_id: button21.id,
            button22_id: button22.id,
            flag_column: true,
            flag_button21: true,
            i: 0,
            label2_id: label2.id,
        }
    }

    fn inc(&mut self, ctx: &mut CallbackContext<Self>) -> Result<()> {
        self.i += 1;
        ctx.widget(self.button21_id)?
            .set_text(format!("i = {}", self.i));
        Ok(())
    }

    fn button_clicked(
        &mut self,
        ctx: &mut CallbackContext<Self>,
        data: String,
        k: u32,
    ) -> Result<()> {
        println!("callback! {:?}, {}", data, k);
        let button = ctx.widget(self.button_id)?;
        button.set_text(&format!("ok {}", if k == 1 { "1" } else { "22222" }));

        if k == 1 {
            self.flag_column = !self.flag_column;
            ctx.widget(self.column2_id)?.set_enabled(self.flag_column);
            println!("set enabled {:?} {:?}", self.column2_id, self.flag_column);
        } else {
            self.flag_button21 = !self.flag_button21;
            ctx.widget(self.button21_id)?
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
