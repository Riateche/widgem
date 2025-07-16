use {
    widgem::{
        impl_widget_base,
        widgets::{button::Button, menu::Menu, window::Window, Widget, WidgetBaseOf, WidgetExt},
    },
    widgem_test_kit::context::Context,
};

pub struct RootWidget {
    base: WidgetBaseOf<Self>,
}

impl RootWidget {
    fn on_triggered(&mut self, _event: ()) -> anyhow::Result<()> {
        self.base.add_child::<Menu>();
        Ok(())
    }
}

impl Widget for RootWidget {
    impl_widget_base!();

    fn new(mut base: WidgetBaseOf<Self>) -> Self {
        let id = base.id();
        let window = base.add_child::<Window>().set_title(module_path!());

        window
            .base_mut()
            .add_child::<Button>()
            .set_column(0)
            .set_row(0)
            .set_text("test")
            .on_triggered(id.callback(Self::on_triggered));

        Self { base }
    }
}

#[widgem_test_kit::test]
fn menu(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().add_child::<RootWidget>();
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.close()?;
    Ok(())
}
