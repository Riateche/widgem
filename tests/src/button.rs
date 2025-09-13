use {
    widgem::{
        impl_widget_base,
        widgets::{Button, Widget, WidgetBaseOf, WidgetInitializer, Window},
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
        let mut items = base.children_mut();
        let window = items.set_next_item(Window::init(module_path!().into()));

        window
            .items_mut()
            .set_next_item(Button::init("Test".into()));

        RootWidget { base }
    }

    fn reinit(self, _widget: &mut Self::Output) {}
}

impl Widget for RootWidget {
    impl_widget_base!();
}

#[widgem_tester::test]
pub fn button(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|root| {
        root.items_mut().set_next_item(RootWidget::init());
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.snapshot("button")?;
    window.close()?;
    Ok(())
}
