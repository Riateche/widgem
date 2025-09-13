use {
    widgem::{
        impl_widget_base,
        widgets::{Label, Widget, WidgetBaseOf, WidgetInitializer, Window},
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
        let window = base.add_child(Window::init(module_path!().into()));

        window.base_mut().add_child(Label::init("Test".into()));

        RootWidget { base }
    }

    fn reinit(self, _widget: &mut Self::Output) {}
}

impl Widget for RootWidget {
    impl_widget_base!();
}

#[widgem_tester::test]
pub fn label(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().add_child(RootWidget::init());
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.snapshot("label")?;
    window.close()?;
    Ok(())
}
