use salvation::{
    widgets::{padding_box::PaddingBox, text_input::TextInput, WidgetExt},
    window::create_window,
    CallbackContext, WindowAttributes,
};

use crate::{context::Context, init_test_app};

struct State {}

impl State {
    fn new(_ctx: &mut CallbackContext<Self>) -> Self {
        let input = TextInput::new("Hello world");
        // TODO: use module_path!
        create_window(
            WindowAttributes::default().with_title("salvation_tests::test_cases::text_input"),
            Some(PaddingBox::new(input.boxed()).boxed()),
        );
        State {}
    }
}

pub fn run() -> anyhow::Result<()> {
    init_test_app().run(State::new)
}

pub fn check(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.set_blinking_expected(true);
    let window = ctx.wait_for_window_by_pid()?;
    // Workaround for winit issue:
    // https://github.com/rust-windowing/winit/issues/2841
    window.minimize()?;
    window.activate()?;
    ctx.snapshot(&window, "window with text input - text Hello world")?;
    ctx.connection.key("Right")?;
    ctx.snapshot(&window, "cursor moved to the right of H")?;
    ctx.connection.key("Shift+Right")?;
    ctx.set_blinking_expected(false);
    ctx.snapshot(&window, "selected e")?;
    ctx.connection.key("Right")?;
    ctx.set_blinking_expected(true);
    ctx.snapshot(
        &window,
        "cleared selection and cursor moved to the right of He",
    )?;
    ctx.connection.key("Left")?;
    ctx.snapshot(&window, "cursor moved to the right of H")?;
    ctx.connection.key("Ctrl+Right")?;
    ctx.snapshot(&window, "cursor moved to the right of Hello")?;
    ctx.connection.key("Ctrl+Right")?;
    ctx.snapshot(&window, "cursor moved to the end")?;
    ctx.connection.key("Ctrl+Left")?;
    ctx.snapshot(&window, "cursor moved to the right of Hello after space")?;
    ctx.connection.key("Ctrl+Left")?;
    ctx.snapshot(&window, "cursor moved to the start")?;
    ctx.connection.key("End")?;
    ctx.snapshot(&window, "cursor moved to the end")?;
    ctx.connection.key("Shift+Left")?;
    ctx.set_blinking_expected(false);
    ctx.snapshot(&window, "selected d")?;
    ctx.connection.key("Left")?;
    ctx.set_blinking_expected(true);
    ctx.snapshot(
        &window,
        "cleared selection and cursor moved to the right of worl",
    )?;
    ctx.connection.key("Ctrl+Shift+Left")?;
    ctx.set_blinking_expected(false);
    ctx.snapshot(&window, "selected worl")?;
    ctx.connection.key("End")?;
    ctx.connection.type_text(" Lorem Ipsum")?;
    ctx.set_blinking_expected(true);
    ctx.snapshot(&window, "added space Lorem Ipsum to the end")?;
    // Checking horizontal scroll.
    ctx.connection.key("Ctrl+Left")?;
    ctx.connection.key("Ctrl+Left")?;
    ctx.connection.key("Ctrl+Left")?;
    ctx.snapshot(&window, "cursor moved to the right of Hello after space")?;
    ctx.connection.key("Left")?;
    ctx.snapshot(&window, "cursor moved to the right of Hello and scrolled")?;
    ctx.connection.key("Left")?;
    ctx.snapshot(&window, "cursor moved to the right of Hell and scrolled")?;
    ctx.connection.key("Left")?;
    ctx.snapshot(&window, "cursor moved to the right of Hel and scrolled")?;

    window.close()?;
    Ok(())
}
