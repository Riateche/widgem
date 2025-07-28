use {
    widgem::{
        impl_widget_base,
        widgets::{Button, Menu, MenuItem, NewWidget, Widget, WidgetBaseOf, Window},
        WidgetExt, WidgetId,
    },
    widgem_test_kit::context::Context,
};

pub struct RootWidget {
    base: WidgetBaseOf<Self>,
    button_id: WidgetId<Button>,
}

impl RootWidget {
    fn on_triggered(&mut self, _event: ()) -> anyhow::Result<()> {
        let button = self.base.find_child(self.button_id)?;
        let rect = button.base().rect_in_window_or_err()?;
        let window = button.base().window_or_err()?;
        let pos_in_window = window
            .cursor_position()
            .unwrap_or_else(|| rect.bottom_right());
        let global_pos = window.inner_position()? + pos_in_window;

        let menu = self.base.add_child::<Menu>(global_pos);

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

        let button_id = window
            .base_mut()
            .add_child::<Button>("test".into())
            .on_triggered(id.callback(Self::on_triggered))
            .id();

        Self { base, button_id }
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
