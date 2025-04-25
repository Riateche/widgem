use {
    salvation::{
        impl_widget_common,
        widgets::{
            label::Label, window::WindowWidget, Widget, WidgetCommon, WidgetCommonTyped, WidgetExt,
        },
    },
    salvation_test_kit::context::Context,
};

pub struct RootWidget {
    common: WidgetCommon,
}

impl Widget for RootWidget {
    impl_widget_common!();

    fn new(mut common: WidgetCommonTyped<Self>) -> Self {
        let window = common
            .add_child::<WindowWidget>(0)
            .set_title(module_path!());

        window
            .common_mut()
            .child::<Label>(0)
            .set_column(0)
            .set_row(0)
            .set_text("Test");

        Self {
            common: common.into(),
        }
    }
}

#[salvation_test_kit::test]
pub fn label(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.common_mut().child::<RootWidget>(0);
        Ok(())
    })?;
    let mut window = ctx.wait_for_window_by_pid()?;
    ctx.snapshot(&mut window, "label")?;
    window.close()?;
    Ok(())
}
