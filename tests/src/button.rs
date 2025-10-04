use {
    widgem::{
        impl_widget_base,
        widgets::{Button, Widget, WidgetBaseOf, Window},
        WidgetInitializer,
    },
    widgem_tester::context::Context,
};

pub struct RootWidget {
    base: WidgetBaseOf<Self>,
}

impl RootWidget {
    pub fn init() -> impl WidgetInitializer<Output = Self> {
        struct Initializer;

        impl WidgetInitializer for Initializer {
            type Output = RootWidget;

            fn init(self, mut base: WidgetBaseOf<Self::Output>) -> Self::Output {
                let window = base.set_main_child(Window::init(module_path!().into()));
                window.set_main_content(Button::init("Test".into()));

                RootWidget { base }
            }

            fn reinit(self, _widget: &mut Self::Output) {}
        }

        Initializer
    }
}

impl Widget for RootWidget {
    impl_widget_base!();
}

#[widgem_tester::test]
pub fn button(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|root| {
        root.set_main_content(RootWidget::init());
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.snapshot("button")?;
    window.close()?;
    Ok(())
}
