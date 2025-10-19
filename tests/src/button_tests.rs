use {
    widgem::{
        impl_widget_base, widget_initializer,
        widgets::{Button, Window},
        Widget, WidgetBaseOf, WidgetInitializer,
    },
    widgem_tester::Context,
};

pub struct RootWidget {
    base: WidgetBaseOf<Self>,
}

impl RootWidget {
    pub fn init() -> impl WidgetInitializer<Output = Self> {
        widget_initializer::from_fallible_new(|mut base| {
            let window = base.set_main_child(Window::init(module_path!().into()))?;
            window.set_main_content(Button::init("Test".into()))?;

            Ok(RootWidget { base })
        })
    }
}

impl Widget for RootWidget {
    impl_widget_base!();
}

#[widgem_tester::test]
pub fn main(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|root| {
        root.set_main_content(RootWidget::init())?;
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.snapshot("button")?;
    window.close()?;
    Ok(())
}
