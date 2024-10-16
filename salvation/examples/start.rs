#![allow(dead_code)]

use std::time::Duration;

use anyhow::Result;

use salvation::{
    event::LayoutEvent,
    impl_widget_common,
    layout::SizeHintMode,
    system::add_interval,
    types::Rect,
    widgets::{
        button::Button, column::Column, label::Label, padding_box::PaddingBox,
        scroll_area::ScrollArea, text_input::TextInput, Widget, WidgetCommon, WidgetExt, WidgetId,
    },
    App,
};
use winit::window::Window;

struct AnotherWidget {
    common: WidgetCommon,
    counter: i32,
}

impl AnotherWidget {
    fn new() -> Self {
        let mut this = Self {
            counter: 0,
            common: WidgetCommon::new(),
        };
        let mut button = Button::new("another button");
        button.on_triggered(this.callback(|this, _event| {
            this.counter += 1;
            println!("counter: {}", this.counter);
            let label = Label::new(format!("counter: {}", this.counter))
                .with_window(Window::default_attributes().with_title("example"))
                .boxed();
            this.common_mut().add_child(label, Default::default());
            Ok(())
        }));
        this.common_mut()
            .add_child(button.boxed(), Default::default());
        this
    }
}

impl Widget for AnotherWidget {
    impl_widget_common!();

    fn recalculate_size_hint_x(&mut self, mode: SizeHintMode) -> Result<i32> {
        Ok(self.common_mut().children[0].widget.size_hint_x(mode))
    }
    fn recalculate_size_x_fixed(&mut self) -> bool {
        self.common_mut().children[0].widget.size_x_fixed()
    }
    fn recalculate_size_hint_y(&mut self, size_x: i32, mode: SizeHintMode) -> Result<i32> {
        Ok(self.common_mut().children[0]
            .widget
            .size_hint_y(size_x, mode))
    }
    fn recalculate_size_y_fixed(&mut self) -> bool {
        self.common_mut().children[0].widget.size_y_fixed()
    }
    fn handle_layout(&mut self, event: LayoutEvent) -> Result<()> {
        self.common.set_child_rect(
            0,
            event
                .new_rect_in_window()
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
    fn new() -> Self {
        let mut common = WidgetCommon::new();
        let mut root = Column::new();

        root.add_child(TextInput::new("Hello, Rust! 🦀😂\n").boxed());
        root.add_child(TextInput::new("Hebrew name Sarah: שרה.").boxed());

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
            .with_auto_repeat(true)
            .with_on_triggered(
                common
                    .id
                    .callback(|this: &mut Self, event| this.button_clicked(event, 1)),
            )
            .with_id();

        root.add_child(btn1.widget.boxed());

        root.add_child(
            Button::new("btn2")
                .with_on_triggered(
                    common
                        .id
                        .callback(|this: &mut Self, event| this.button_clicked(event, 2)),
                )
                .boxed(),
        );

        let button21 = Button::new("btn21")
            .with_on_triggered(common.id.callback(|_: &mut Self, _| {
                println!("click!");
                Ok(())
            }))
            .with_id();

        let button22 = Button::new("btn22").with_id();

        let column2 = Column::new()
            .with_child(button21.widget.boxed())
            .with_child(button22.widget.boxed())
            .with_id();
        root.add_child(column2.widget.boxed());

        let btn3 = AnotherWidget::new().boxed();
        root.add_child(btn3);

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
        let label2 = Label::new("ok").with_id();
        root.add_child(label2.widget.boxed());

        let mut content = Column::new();
        for i in 1..=80 {
            content.add_child(Button::new(format!("btn{i}")).boxed());
        }

        root.add_child(ScrollArea::new(content.boxed()).boxed());

        common.add_child(
            PaddingBox::new(root.boxed())
                .with_window(Window::default_attributes().with_title("example"))
                .boxed(),
            Default::default(),
        );
        add_interval(
            Duration::from_secs(2),
            common.id.callback(|this: &mut Self, _| this.inc()),
        );

        RootWidget {
            common,
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

    fn inc(&mut self) -> Result<()> {
        self.i += 1;
        if let Ok(widget) = self.common.widget(self.button21_id) {
            widget.set_text(format!("i = {}", self.i));
        }
        Ok(())
    }

    fn button_clicked(&mut self, data: String, k: u32) -> Result<()> {
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

    //salvation::style::Style::load("themes/default/theme.css").unwrap();

    // let data = std::fs::read_to_string("themes/default/theme.css").unwrap();
    // let mut style = StyleSheet::parse(&data, Default::default()).unwrap();
    // replace_vars(&mut style);
    // let code = style.to_css(Default::default()).unwrap().code;
    // let mut style = StyleSheet::parse(&code, Default::default()).unwrap();
    // println!("{style:#?}");

    // for rule in &mut style.rules.0 {
    //     if let CssRule::Style(rule) = rule {
    //         for property in rule.declarations.iter_mut() {
    //             if let Property::Custom(property) = property {
    //                 //println!("{}", property.name);
    //                 if property.name.as_ref() == "min-padding" {
    //                     println!("found min-padding: {:?}", property.value);
    //                     property.to_css();
    //                 }
    //                 // let mut new_tokens = Vec::new();
    //                 // for token in &property.value.0 {
    //                 //     if let TokenOrValue::Var(variable) = token {
    //                 //         if let Some(value) = vars.get(variable.name.ident.as_ref()) {
    //                 //             println!("substitute!");
    //                 //             new_tokens.extend(value.0.clone());
    //                 //             continue;
    //                 //         }
    //                 //     }
    //                 //     new_tokens.push(token.clone());
    //                 // }
    //                 // property.value.0 = new_tokens;
    //             }
    //         }
    //     }
    // }
    App::new().run(|| RootWidget::new().boxed()).unwrap();
}
