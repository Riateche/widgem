use std::{thread::sleep, time::Duration};

use salvation::winit::{event::WindowEvent, window::WindowBuilder};
use salvation::{
    event_loop::CallbackContext,
    widgets::{column::Column, padding_box::PaddingBox, text_input::TextInput, WidgetExt},
    window::create_window,
};

use anyhow::Result;

use crate::run::run;
use crate::run::TestContext;

struct State {}

impl State {
    fn new(_ctx: &mut CallbackContext<Self>) -> Self {
        let mut root = Column::new();
        root.add_child(TextInput::new("Hello, Testing Framework! 🦀\n").boxed());
        create_window(
            WindowBuilder::new().with_title("example"),
            Some(PaddingBox::new(root.boxed()).boxed()),
        );
        State {}
    }
}

pub fn first_test() -> Result<()> {
    let run_test = move |ctx: &mut TestContext| -> Result<()> {
        sleep(Duration::from_secs(10));
        ctx.snapshot("Just checks that something works, it is our first test.")?;
        ctx.event(0, WindowEvent::CloseRequested)?;
        Ok(())
    };
    run!(State::new, run_test)
}
