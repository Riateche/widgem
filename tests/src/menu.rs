use {
    widgem::{
        impl_widget_base,
        widgets::{Button, Menu, NewWidget, Widget, WidgetBaseOf, WidgetExt, Window},
    },
    widgem_test_kit::context::Context,
};

pub struct RootWidget {
    base: WidgetBaseOf<Self>,
}

impl RootWidget {
    fn on_triggered(&mut self, _event: ()) -> anyhow::Result<()> {
        self.base.add_child::<Menu>(());
        Ok(())
    }
}

impl NewWidget for RootWidget {
    type Arg = ();

    fn new(mut base: WidgetBaseOf<Self>, (): Self::Arg) -> Self {
        let id = base.id();
        let window = base.add_child::<Window>(module_path!().into());

        window
            .base_mut()
            .add_child::<Button>("test".into())
            .set_column(0)
            .set_row(0)
            .on_triggered(id.callback(Self::on_triggered));

        Self { base }
    }
    fn handle_declared(&mut self, (): Self::Arg) {}
}

impl Widget for RootWidget {
    impl_widget_base!();
}

#[widgem_test_kit::test]
fn menu(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().add_child::<RootWidget>(());
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.close()?;
    Ok(())
}
