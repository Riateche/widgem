use {
    widgem::{
        impl_widget_base,
        widgets::{Label, NewWidget, Widget, WidgetBaseOf, WidgetExt, Window},
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
            .add_child::<Label>("Test".into())
            .set_column(0)
            .set_row(0);

        Self { base }
    }
    fn handle_declared(&mut self, (): Self::Arg) {}
}

impl Widget for RootWidget {
    impl_widget_base!();
}

#[widgem_test_kit::test]
pub fn label(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().add_child::<RootWidget>(());
        Ok(())
    })?;
    let mut window = ctx.wait_for_window_by_pid()?;
    ctx.snapshot(&mut window, "label")?;
    window.close()?;
    Ok(())
}
