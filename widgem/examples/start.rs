#![allow(dead_code)]

use {
    anyhow::Result,
    std::time::Duration,
    tracing::level_filters::LevelFilter,
    tracing_subscriber::EnvFilter,
    widgem::{
        impl_widget_base,
        widgets::{
            Button, Column, Label, ScrollArea, TextInput, Widget, WidgetBaseOf, WidgetExt,
            WidgetId, WidgetInitializer, Window,
        },
    },
};

struct AnotherWidget {
    base: WidgetBaseOf<Self>,
    counter: i32,
}

impl AnotherWidget {
    fn init() -> impl WidgetInitializer<Output = Self> {
        struct Initializer;

        impl WidgetInitializer for Initializer {
            type Output = AnotherWidget;
            fn init(self, base: WidgetBaseOf<Self::Output>) -> Self::Output {
                let mut this = AnotherWidget { counter: 0, base };
                let callback = this.callback(|this, _event| {
                    this.counter += 1;
                    println!("counter: {}", this.counter);
                    let window = this.base.add_child_with_key(
                        ("window", this.counter),
                        Window::init("example".into()),
                    );
                    println!("window {:?}", window.id());
                    let label = window
                        .base_mut()
                        .add_child(Label::init(format!("counter: {}", this.counter)));
                    println!("label {:?}", label.id());
                    Ok(())
                });
                let button = this
                    .base_mut()
                    .add_child_with_key("button", Button::init("another button".into()));
                button.on_triggered(callback);
                this
            }
            fn reinit(self, _widget: &mut Self::Output) {}
        }

        Initializer
    }
}

// #[derive(Debug)]
// enum Abc {
//     Def,
// }

// impl FormatKey for Abc {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         writeln!(f, "{:?}", self)
//     }
// }

impl Widget for AnotherWidget {
    impl_widget_base!();
}

struct RootWidget {
    base: WidgetBaseOf<Self>,
    button_id: WidgetId<Button>,
    column2_id: WidgetId<Column>,
    button21_id: WidgetId<Button>,
    button22_id: WidgetId<Button>,
    flag_column: bool,
    flag_button21: bool,
    i: i32,
    label2_id: WidgetId<Label>,
}

impl RootWidget {
    fn init() -> impl WidgetInitializer<Output = Self> {
        struct Initializer;

        impl WidgetInitializer for Initializer {
            type Output = RootWidget;
            fn init(self, mut base: WidgetBaseOf<Self::Output>) -> Self::Output {
                let callbacks = base.callback_creator();

                let window = base.add_child(Window::init("example".into()));

                let root = window.base_mut().add_child(Column::init());

                root.base_mut()
                    .add_child(TextInput::init())
                    .set_text("Hello, Rust! 🦀😂\n");
                root.base_mut()
                    .add_child(TextInput::init())
                    .set_text("Hebrew name Sarah: שרה.");

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

                let button_id = root
                    .base_mut()
                    .add_child(Button::init("btn1".into()))
                    .set_auto_repeat(true)
                    .on_triggered(callbacks.create(|this, event| this.button_clicked(event, 1)))
                    .id();

                root.base_mut()
                    .add_child(Button::init("btn2".into()))
                    .on_triggered(callbacks.create(|this, event| this.button_clicked(event, 2)));

                let column2 = root.base_mut().add_child(Column::init());
                let button21_id = column2
                    .base_mut()
                    .add_child(Button::init("btn21".into()))
                    .on_triggered(callbacks.create(|_, _| {
                        println!("click!");
                        Ok(())
                    }))
                    .id();

                let button22_id = column2
                    .base_mut()
                    .add_child(Button::init("btn22".into()))
                    .id();
                let column2_id = column2.id();

                root.base_mut().add_child(AnotherWidget::init());

                let label2_id = root.base_mut().add_child(Label::init("ok".into())).id();

                let scroll_area = root.base_mut().add_child(ScrollArea::init());
                let content = scroll_area.set_content(Column::init());
                for i in 1..=80 {
                    content.base_mut().add_child(Button::init(format!(
                        "btn btn btn btn btn btn btn btn btn btn{i}"
                    )));
                }

                base.app().add_interval(
                    Duration::from_secs(2),
                    callbacks.create(|this, _| this.inc()),
                );

                RootWidget {
                    base,
                    button_id,
                    column2_id,
                    button21_id,
                    button22_id,
                    flag_column: true,
                    flag_button21: true,
                    i: 0,
                    label2_id,
                }
            }
            fn reinit(self, _widget: &mut Self::Output) {}
        }

        Initializer
    }

    fn inc(&mut self) -> Result<()> {
        self.i += 1;
        if let Ok(widget) = self.base.find_child_mut(self.button21_id) {
            widget.set_text(format!("i = {}", self.i));
        }
        Ok(())
    }

    fn button_clicked(&mut self, data: (), k: u32) -> Result<()> {
        println!("callback! {:?}, {}", data, k);
        let button = self.base.find_child_mut(self.button_id)?;
        button.set_text(format!("ok {}", if k == 1 { "1" } else { "22222" }));

        if k == 1 {
            self.flag_column = !self.flag_column;
            self.base
                .find_child_mut(self.column2_id)?
                .set_enabled(self.flag_column);
            println!("set enabled {:?} {:?}", self.column2_id, self.flag_column);
        } else {
            self.flag_button21 = !self.flag_button21;
            self.base
                .find_child_mut(self.button21_id)?
                .set_enabled(self.flag_button21);
            println!(
                "set enabled {:?} {:?}",
                self.button21_id, self.flag_button21
            );
        }
        Ok(())
    }
}

impl Widget for RootWidget {
    impl_widget_base!();
}

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }

    tracing_subscriber::fmt::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env()
                .unwrap(),
        )
        .init();

    widgem::run(|r| {
        r.base_mut().add_child(RootWidget::init());
        Ok(())
    })
    .unwrap();
}
