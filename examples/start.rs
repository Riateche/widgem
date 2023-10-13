#![allow(dead_code)]

use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use itertools::Itertools;
use lightningcss::{
    properties::{
        custom::{CustomPropertyName, TokenOrValue},
        Property,
    },
    rules::CssRule,
    selector::{Component, Selector},
    stylesheet::StyleSheet,
};
use salvation::{
    event_loop::{self, CallbackContext},
    system::add_interval,
    widgets::{
        button::Button, column::Column, label::Label, padding_box::PaddingBox,
        scroll_area::ScrollArea, text_input::TextInput, Widget, WidgetExt, WidgetId,
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

        root.add_child(TextInput::new("Hello, Rust! 🦀\n").boxed());
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
            .with_on_clicked(ctx.callback(|state, ctx, event| state.button_clicked(ctx, event, 1)))
            .split_id();

        root.add_child(btn1.widget.boxed());

        root.add_child(
            Button::new("btn2")
                .with_on_clicked(
                    ctx.callback(|state, ctx, event| state.button_clicked(ctx, event, 2)),
                )
                .boxed(),
        );

        let button21 = Button::new("btn21")
            .with_on_clicked(ctx.callback(|_, _, _| {
                println!("click!");
                Ok(())
            }))
            .split_id();

        let button22 = Button::new("btn22").split_id();

        let column2 = Column::new()
            .with_child(button21.widget.boxed())
            .with_child(button22.widget.boxed())
            .split_id();
        root.add_child(column2.widget.boxed());

        let (another_state, btn3) =
            AnotherState::new(&mut ctx.map_state(|state| Some(&mut state.another_state)));
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
        let label2 = Label::new("ok").split_id();
        root.add_child(label2.widget.boxed());

        let mut content = Column::new();
        for i in 1..=20 {
            content.add_child(Button::new(format!("btn{i}")).boxed());
        }

        root.add_child(ScrollArea::new(content.boxed()).boxed());

        create_window(
            WindowBuilder::new().with_title("example"),
            Some(PaddingBox::new(root.boxed()).boxed()),
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
        if let Ok(widget) = ctx.widget(self.button21_id) {
            widget.set_text(format!("i = {}", self.i));
        }
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

fn replace_vars(style_sheet: &mut StyleSheet) {
    //let mut style_sheet: StyleSheet<'static, 'static> = style_sheet.into_owned();
    let mut vars = HashMap::new();
    for rule in &style_sheet.rules.0 {
        if let CssRule::Style(rule) = rule {
            // println!("selectors: {:?}", rule.selectors);
            for selector in &rule.selectors.0 {
                if is_root(selector) {
                    println!("found root!");
                    for (property, _) in rule.declarations.iter() {
                        //println!("root declaration: {declaration:?}");
                        if let Property::Custom(property) = property {
                            if let CustomPropertyName::Custom(name) = &property.name {
                                vars.insert(name.as_ref().to_string(), property.value.clone());
                            }
                        }
                    }
                }
                //print_selector(selector);
            }
        }
    }
    for rule in &mut style_sheet.rules.0 {
        if let CssRule::Style(rule) = rule {
            for property in rule.declarations.iter_mut() {
                if let Property::Unparsed(property) = property {
                    let mut new_tokens = Vec::new();
                    for token in &property.value.0 {
                        if let TokenOrValue::Var(variable) = token {
                            if let Some(value) = vars.get(variable.name.ident.as_ref()) {
                                println!("substitute!");
                                new_tokens.extend(value.0.clone());
                                continue;
                            }
                        }
                        new_tokens.push(token.clone());
                    }
                    property.value.0 = new_tokens;
                }
            }
        }
    }

    println!("vars: {vars:#?}");
}

fn print_selector(selector: &Selector) {
    println!("selector: {:?}", selector);
    let mut iter = selector.iter();
    loop {
        for x in &mut iter {
            println!("selector item: {:?}", x);
            if matches!(x, Component::Root) {
                println!("found root!");
            }
            if let Component::Negation(inner) = x {
                println!("found not! inner:");
                print_selector(&inner[0]);
                println!("inner end");
            }
        }
        if let Some(seq) = iter.next_sequence() {
            println!("seq: {seq:?}");
        } else {
            println!("no seq");
            break;
        }
    }
}

fn selector_items<'i, 'a>(selector: &'a Selector<'i>) -> Option<Vec<&'a Component<'i>>> {
    let mut iter = selector.iter();
    let components = (&mut iter).collect_vec();
    if iter.next_sequence().is_some() {
        // We don't support nesting in selectors.
        return None;
    }
    Some(components)
}

fn is_root(selector: &Selector) -> bool {
    selector_items(selector).map_or(false, |items| {
        items.len() == 1 && matches!(items[0], Component::Root)
    })
}

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    env_logger::init();

    let data = std::fs::read_to_string("/projects/salvation/1.css").unwrap();
    let mut style = StyleSheet::parse(&data, Default::default()).unwrap();
    replace_vars(&mut style);
    let code = style.to_css(Default::default()).unwrap().code;
    let style = StyleSheet::parse(&code, Default::default()).unwrap();
    println!("{style:#?}");

    //event_loop::run(State::new).unwrap();
}
