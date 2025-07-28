use {
    widgem::{
        impl_widget_base,
        widgets::{Button, Menu, MenuItem, NewWidget, Widget, WidgetBaseOf, Window},
    },
    widgem_test_kit::context::Context,
};

pub struct RootWidget {
    base: WidgetBaseOf<Self>,
}

impl RootWidget {
    fn on_triggered(&mut self, _event: ()) -> anyhow::Result<()> {
        let menu = self.base.add_child::<Menu>(());
        menu.base_mut().add_child::<MenuItem>("Item 1".into());
        menu.base_mut().add_child::<MenuItem>("Item 2".into());
        menu.base_mut().add_child::<MenuItem>("Item 3".into());
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
