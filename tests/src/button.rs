use {
    widgem::{
        impl_widget_base,
        widgets::{Button, NewWidget, Widget, WidgetBaseOf, Window},
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

        window.base_mut().add_child::<Button>("Test".into());

        Self { base }
    }

    fn handle_declared(&mut self, (): Self::Arg) {}
}

impl Widget for RootWidget {
    impl_widget_base!();
}

#[widgem_test_kit::test]
pub fn button(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().add_child::<RootWidget>(());
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.snapshot("button")?;
    window.close()?;
    Ok(())
}
