#![allow(dead_code)]

use {
    anyhow::Result,
    salvation::{
        event::LayoutEvent,
        impl_widget_common,
        layout::SizeHintMode,
        system::add_interval,
        types::Rect,
        widgets::{
            button::Button, column::Column, label::Label, scroll_area::ScrollArea,
            text_input::TextInput, Widget, WidgetCommon, WidgetCommonTyped, WidgetExt, WidgetId,
        },
        App,
    },
    std::time::Duration,
    winit::window::Window,
};

struct AnotherWidget {
    common: WidgetCommon,
    counter: i32,
}

impl Widget for AnotherWidget {
    impl_widget_common!();

    fn new(common: WidgetCommonTyped<Self>) -> Self {
        let mut this = Self {
            counter: 0,
            common: common.into(),
        };
        let callback = this.callback(|this, _event| {
            this.counter += 1;
            println!("counter: {}", this.counter);
            this.common
                .add_child_window::<Label>(
                    this.counter as u64,
                    Window::default_attributes().with_title("example"),
                )
                .set_text(format!("counter: {}", this.counter));
            Ok(())
        });
        let button = this.common_mut().add_child::<Button>(0, Default::default());
        button.set_text("another button");
        button.on_triggered(callback);
        this
    }

    fn recalculate_size_hint_x(&mut self, mode: SizeHintMode) -> Result<i32> {
        Ok(self
            .common_mut()
            .children
            .get_mut(&0)
            .unwrap()
            .widget
            .size_hint_x(mode))
    }
    fn recalculate_size_x_fixed(&mut self) -> bool {
        self.common_mut()
            .children
            .get_mut(&0)
            .unwrap()
            .widget
            .size_x_fixed()
    }
    fn recalculate_size_hint_y(&mut self, size_x: i32, mode: SizeHintMode) -> Result<i32> {
        Ok(self
            .common_mut()
            .children
            .get_mut(&0)
            .unwrap()
            .widget
            .size_hint_y(size_x, mode))
    }
    fn recalculate_size_y_fixed(&mut self) -> bool {
        self.common_mut()
            .children
            .get_mut(&0)
            .unwrap()
            .widget
            .size_y_fixed()
    }
    fn handle_layout(&mut self, event: LayoutEvent) -> Result<()> {
        self.common.set_child_rect(
            0,
            event
                .new_rect_in_window
                .map(|r| Rect::from_pos_size(Default::default(), r.size)),
        )
    }
}

struct RootWidget {
    common: WidgetCommon,
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
    fn inc(&mut self) -> Result<()> {
        self.i += 1;
        if let Ok(widget) = self.common.widget(self.button21_id) {
            widget.set_text(format!("i = {}", self.i));
        }
        Ok(())
    }

    fn button_clicked(&mut self, data: (), k: u32) -> Result<()> {
        println!("callback! {:?}, {}", data, k);
        let button = self.common.widget(self.button_id)?;
        button.set_text(format!("ok {}", if k == 1 { "1" } else { "22222" }));

        if k == 1 {
            self.flag_column = !self.flag_column;
            self.common
                .widget(self.column2_id)?
                .set_enabled(self.flag_column);
            println!("set enabled {:?} {:?}", self.column2_id, self.flag_column);
        } else {
            self.flag_button21 = !self.flag_button21;
            self.common
                .widget(self.button21_id)?
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
    impl_widget_common!();

    fn new(mut common: WidgetCommonTyped<Self>) -> Self {
        let id = common.id();

        let root = common
            .add_child_window::<Column>(0, Window::default_attributes().with_title("example"));

        root.add_child::<TextInput>()
            .set_text("Hello, Rust! 🦀😂\n");
        root.add_child::<TextInput>()
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
            .add_child::<Button>()
            .set_text("btn1")
            .set_auto_repeat(true)
            .on_triggered(id.callback(|this, event| this.button_clicked(event, 1)))
            .id();

        root.add_child::<Button>()
            .set_text("btn2")
            .on_triggered(id.callback(|this, event| this.button_clicked(event, 2)));

        let column2 = root.add_child::<Column>();
        let button21_id = column2
            .add_child::<Button>()
            .set_text("btn21")
            .on_triggered(id.callback(|_, _| {
                println!("click!");
                Ok(())
            }))
            .id();

        let button22_id = column2.add_child::<Button>().set_text("btn22").id();
        let column2_id = column2.id();

        root.add_child::<AnotherWidget>();

        // root.add_child(
        //     ScrollBar::new()
        //         .with_axis(Axis::Y)
        //         .with_on_value_changed(ctx.callback(|this, ctx, value| {
        //             ctx.widget(this.label2_id)?
        //                 .set_text(format!("value={value}"));
        //             Ok(())
        //         }))
        //         .boxed(),
        // );
        let label2_id = root.add_child::<Label>().set_text("ok").id();

        let scroll_area = root.add_child::<ScrollArea>();
        let content = scroll_area.add_content::<Column>();
        for i in 1..=80 {
            content
                .add_child::<Button>()
                .set_text(format!("btn btn btn btn btn btn btn btn btn btn{i}"));
        }

        add_interval(Duration::from_secs(2), id.callback(|this, _| this.inc()));

        RootWidget {
            common: common.into(),
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

    fn recalculate_size_hint_x(&mut self, _mode: SizeHintMode) -> Result<i32> {
        Ok(0)
    }

    fn recalculate_size_hint_y(&mut self, _size_x: i32, _mode: SizeHintMode) -> Result<i32> {
        Ok(0)
    }
}

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    env_logger::init();
    App::new().run::<RootWidget>(|_| Ok(())).unwrap();
}
