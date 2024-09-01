use anyhow::ensure;
use salvation::{
    event_loop::CallbackContext,
    widgets::{padding_box::PaddingBox, text_input::TextInput, WidgetExt},
    window::create_window,
    winit::{error::EventLoopError, window::WindowBuilder},
};

use crate::{context::Context, init_test_app};

struct State {}

impl State {
    fn new(_ctx: &mut CallbackContext<Self>) -> Self {
        let input = TextInput::new("Hello world");
        create_window(
            WindowBuilder::new().with_title("salvation_tests::test_cases::text_input"),
            Some(PaddingBox::new(input.boxed()).boxed()),
        );
        State {}
    }
}

pub fn run() -> Result<(), EventLoopError> {
    init_test_app().run(State::new)
}

pub fn check(ctx: &mut Context) -> anyhow::Result<()> {
    let windows = ctx.connection.wait_for_windows_by_pid(ctx.pid)?;
    ensure!(windows.len() == 1);
    println!("found window!");
    ctx.snapshot(&windows[0], "window with text input - text Hello world")?;

    windows[0].close()?;
    Ok(())
}
