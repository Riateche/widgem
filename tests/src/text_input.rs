use {
    widgem::{
        impl_widget_base,
        widgets::{TextInput, Widget, WidgetBaseOf, WidgetInitializer, Window},
    },
    widgem_tester::context::Context,
};

pub struct RootWidget {
    base: WidgetBaseOf<Self>,
}

impl RootWidget {
    pub fn init() -> impl WidgetInitializer<Output = Self> {
        Initializer
    }
}

struct Initializer;

impl WidgetInitializer for Initializer {
    type Output = RootWidget;

    fn init(self, mut base: WidgetBaseOf<Self::Output>) -> Self::Output {
        let window = base.set_child(0, Window::init(module_path!().into()));

        window
            .base_mut()
            .set_child(0, TextInput::init())
            .set_text("Hello world");

        RootWidget { base }
    }

    fn reinit(self, _widget: &mut Self::Output) {}
}

impl Widget for RootWidget {
    impl_widget_base!();
}

#[widgem_tester::test]
pub fn keys(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().set_child(0, RootWidget::init());
        Ok(())
    })?;
    ctx.set_blinking_expected(true);
    let window = ctx.wait_for_window_by_pid()?;
    window.snapshot("window with text input - text Hello world")?;
    ctx.ui().key("Right")?;
    window.snapshot("cursor moved to the right of H")?;
    ctx.ui().key("Shift+Right")?;
    ctx.set_blinking_expected(false);
    window.snapshot("selected e")?;
    ctx.ui().key("Right")?;
    ctx.set_blinking_expected(true);
    window.snapshot("cleared selection and cursor moved to the right of He")?;
    ctx.ui().key("Left")?;
    window.snapshot("cursor moved to the right of H")?;
    ctx.ui().key("Ctrl+Right")?;
    window.snapshot("cursor moved to the right of Hello")?;
    ctx.ui().key("Ctrl+Right")?;
    window.snapshot("cursor moved to the end")?;
    ctx.ui().key("Ctrl+Left")?;
    window.snapshot("cursor moved to the right of Hello after space")?;
    ctx.ui().key("Ctrl+Left")?;
    window.snapshot("cursor moved to the start")?;
    ctx.ui().key("End")?;
    window.snapshot("cursor moved to the end")?;
    ctx.ui().key("Shift+Left")?;
    ctx.set_blinking_expected(false);
    window.snapshot("selected d")?;
    ctx.ui().key("Left")?;
    ctx.set_blinking_expected(true);
    window.snapshot("cleared selection and cursor moved to the right of worl")?;
    ctx.ui().key("Ctrl+Shift+Left")?;
    ctx.set_blinking_expected(false);
    window.snapshot("selected worl")?;
    ctx.ui().key("End")?;
    ctx.ui().type_text(" Lorem Ipsum")?;
    ctx.set_blinking_expected(true);
    window.snapshot("added space Lorem Ipsum to the end")?;
    // Checking horizontal scroll.
    ctx.ui().key("Ctrl+Left")?;
    ctx.ui().key("Ctrl+Left")?;
    ctx.ui().key("Ctrl+Left")?;
    window.snapshot("cursor moved to the right of Hello after space")?;
    ctx.ui().key("Left")?;
    window.snapshot("cursor moved to the right of Hello and scrolled")?;
    ctx.ui().key("Left")?;
    window.snapshot("cursor moved to the right of Hell and scrolled")?;
    ctx.ui().key("Left")?;
    window.snapshot("cursor moved to the right of Hel and scrolled")?;

    window.close()?;
    Ok(())
}

#[widgem_tester::test]
pub fn mouse(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().set_child(0, RootWidget::init());
        Ok(())
    })?;
    ctx.set_blinking_expected(true);
    let window = ctx.wait_for_window_by_pid()?;
    window.snapshot("text input")?;
    window.mouse_move(48, 27)?;
    ctx.ui().mouse_click(1)?;
    window.snapshot("cursor moved after hello")?;
    window.mouse_move(73, 29)?;
    ctx.ui().mouse_down(1)?;
    window.snapshot("cursor moved after wor")?;
    window.mouse_move(52, 17)?;
    ctx.ui().mouse_up(1)?;
    ctx.set_blinking_expected(false);
    window.snapshot("selected wor")?;
    // Click on the border/padding.
    window.mouse_move(48, 14)?;
    ctx.ui().mouse_click(1)?;
    ctx.set_blinking_expected(true);
    window.snapshot("cursor moved to beginning")?;

    window.close()?;
    Ok(())
}

#[widgem_tester::test]
pub fn resize(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().set_child(0, RootWidget::init());
        Ok(())
    })?;
    ctx.set_blinking_expected(true);
    let window = ctx.wait_for_window_by_pid()?;
    window.snapshot("text input")?;

    window.resize(200, 50)?;
    window.snapshot("expand horizontally")?;

    window.resize(200, 10)?;
    window.snapshot("min vertical size")?;

    window.resize(100, 100)?;
    window.snapshot("normal horizontal and not expanding vertical")?;

    window.resize(10, 100)?;
    window.snapshot("min horizontal size")?;

    window.resize(10, 10)?;
    window.snapshot("min size")?;

    window.close()?;
    Ok(())
}
