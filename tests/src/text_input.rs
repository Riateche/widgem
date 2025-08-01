use {
    widgem::{
        impl_widget_base,
        widgets::{NewWidget, TextInput, Widget, WidgetBaseOf, Window},
    },
    widgem_test_kit::context::Context,
};

pub struct RootWidget {
    base: WidgetBaseOf<Self>,
}

impl NewWidget for RootWidget {
    type Arg = ();

    fn new(mut base: WidgetBaseOf<Self>, (): Self::Arg) -> Self {
        let window = base.add_child::<Window>(module_path!().into());

        window
            .base_mut()
            .add_child::<TextInput>(())
            .set_text("Hello world");

        Self { base }
    }
    fn handle_declared(&mut self, (): Self::Arg) {}
}

impl Widget for RootWidget {
    impl_widget_base!();
}

#[widgem_test_kit::test]
pub fn keys(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().add_child::<RootWidget>(());
        Ok(())
    })?;
    ctx.set_blinking_expected(true);
    let mut window = ctx.wait_for_window_by_pid()?;
    ctx.snapshot(&mut window, "window with text input - text Hello world")?;
    ctx.connection().key("Right")?;
    ctx.snapshot(&mut window, "cursor moved to the right of H")?;
    ctx.connection().key("Shift+Right")?;
    ctx.set_blinking_expected(false);
    ctx.snapshot(&mut window, "selected e")?;
    ctx.connection().key("Right")?;
    ctx.set_blinking_expected(true);
    ctx.snapshot(
        &mut window,
        "cleared selection and cursor moved to the right of He",
    )?;
    ctx.connection().key("Left")?;
    ctx.snapshot(&mut window, "cursor moved to the right of H")?;
    ctx.connection().key("Ctrl+Right")?;
    ctx.snapshot(&mut window, "cursor moved to the right of Hello")?;
    ctx.connection().key("Ctrl+Right")?;
    ctx.snapshot(&mut window, "cursor moved to the end")?;
    ctx.connection().key("Ctrl+Left")?;
    ctx.snapshot(
        &mut window,
        "cursor moved to the right of Hello after space",
    )?;
    ctx.connection().key("Ctrl+Left")?;
    ctx.snapshot(&mut window, "cursor moved to the start")?;
    ctx.connection().key("End")?;
    ctx.snapshot(&mut window, "cursor moved to the end")?;
    ctx.connection().key("Shift+Left")?;
    ctx.set_blinking_expected(false);
    ctx.snapshot(&mut window, "selected d")?;
    ctx.connection().key("Left")?;
    ctx.set_blinking_expected(true);
    ctx.snapshot(
        &mut window,
        "cleared selection and cursor moved to the right of worl",
    )?;
    ctx.connection().key("Ctrl+Shift+Left")?;
    ctx.set_blinking_expected(false);
    ctx.snapshot(&mut window, "selected worl")?;
    ctx.connection().key("End")?;
    ctx.connection().type_text(" Lorem Ipsum")?;
    ctx.set_blinking_expected(true);
    ctx.snapshot(&mut window, "added space Lorem Ipsum to the end")?;
    // Checking horizontal scroll.
    ctx.connection().key("Ctrl+Left")?;
    ctx.connection().key("Ctrl+Left")?;
    ctx.connection().key("Ctrl+Left")?;
    ctx.snapshot(
        &mut window,
        "cursor moved to the right of Hello after space",
    )?;
    ctx.connection().key("Left")?;
    ctx.snapshot(
        &mut window,
        "cursor moved to the right of Hello and scrolled",
    )?;
    ctx.connection().key("Left")?;
    ctx.snapshot(
        &mut window,
        "cursor moved to the right of Hell and scrolled",
    )?;
    ctx.connection().key("Left")?;
    ctx.snapshot(&mut window, "cursor moved to the right of Hel and scrolled")?;

    window.close()?;
    Ok(())
}

#[widgem_test_kit::test]
pub fn mouse(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().add_child::<RootWidget>(());
        Ok(())
    })?;
    ctx.set_blinking_expected(true);
    let mut window = ctx.wait_for_window_by_pid()?;
    ctx.snapshot(&mut window, "text input")?;
    window.mouse_move(48, 27)?;
    ctx.connection().mouse_click(1)?;
    ctx.snapshot(&mut window, "cursor moved after hello")?;
    window.mouse_move(73, 29)?;
    ctx.connection().mouse_down(1)?;
    ctx.snapshot(&mut window, "cursor moved after wor")?;
    window.mouse_move(52, 17)?;
    ctx.connection().mouse_up(1)?;
    ctx.set_blinking_expected(false);
    ctx.snapshot(&mut window, "selected wor")?;
    // Click on the border/padding.
    window.mouse_move(48, 14)?;
    ctx.connection().mouse_click(1)?;
    ctx.set_blinking_expected(true);
    ctx.snapshot(&mut window, "cursor moved to beginning")?;

    window.close()?;
    Ok(())
}

#[widgem_test_kit::test]
pub fn resize(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().add_child::<RootWidget>(());
        Ok(())
    })?;
    ctx.set_blinking_expected(true);
    let mut window = ctx.wait_for_window_by_pid()?;
    ctx.snapshot(&mut window, "text input")?;

    window.resize(200, 50)?;
    ctx.snapshot(&mut window, "expand horizontally")?;

    window.resize(200, 10)?;
    ctx.snapshot(&mut window, "min vertical size")?;

    window.resize(100, 100)?;
    ctx.snapshot(&mut window, "normal horizontal and not expanding vertical")?;

    window.resize(10, 100)?;
    ctx.snapshot(&mut window, "min horizontal size")?;

    window.resize(10, 10)?;
    ctx.snapshot(&mut window, "min size")?;

    window.close()?;
    Ok(())
}
