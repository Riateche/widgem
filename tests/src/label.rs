use {
    widgem::{
        impl_widget_base,
        widgets::{Label, Widget, WidgetBaseOf, WidgetExt, Window},
    },
    widgem_test_kit::context::Context,
};

pub struct RootWidget {
    base: WidgetBaseOf<Self>,
}

impl Widget for RootWidget {
    impl_widget_base!();

    fn new(mut base: WidgetBaseOf<Self>) -> Self {
        let window = base.add_child::<Window>().set_title(module_path!());

        window
            .base_mut()
            .add_child::<Label>()
            .set_column(0)
            .set_row(0)
            .set_text("Test");

        Self { base }
    }
}

#[widgem_test_kit::test]
pub fn label(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().add_child::<RootWidget>();
        Ok(())
    })?;
    let mut window = ctx.wait_for_window_by_pid()?;
    ctx.snapshot(&mut window, "label")?;
    window.close()?;
    Ok(())
}
