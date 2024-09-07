use salvation::{
    event_loop::CallbackContext,
    widgets::{padding_box::PaddingBox, text_input::TextInput, WidgetExt},
    window::create_window,
    winit::{error::EventLoopError, window::Window},
};

use crate::{context::Context, init_test_app};

struct State {}

impl State {
    fn new(_ctx: &mut CallbackContext<Self>) -> Self {
        let input = TextInput::new("Hello world");
        // TODO: use module_path!
        create_window(
            Window::default_attributes().with_title("salvation_tests::test_cases::text_input"),
            Some(PaddingBox::new(input.boxed()).boxed()),
        );
        State {}
    }
}

pub fn run() -> Result<(), EventLoopError> {
    init_test_app().run(State::new)
}

pub fn check(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.blinking_expected = true;
    let window = ctx.wait_for_window_by_pid()?;
    // Workaround for winit issue:
    // https://github.com/rust-windowing/winit/issues/2841
    window.minimize()?;
    window.activate()?;
    ctx.snapshot(&window, "window with text input - text Hello world")?;
    ctx.connection.key("Right")?;
    ctx.snapshot(&window, "cursor moved to the right of H")?;
    ctx.connection.key("Shift+Right")?;
    ctx.snapshot(&window, "selected e")?;
    ctx.connection.key("Right")?;
    ctx.snapshot(
        &window,
        "cleared selection and cursor moved to the right of He",
    )?;

    window.close()?;
    Ok(())
}
