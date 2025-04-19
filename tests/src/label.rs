use {
    salvation::{
        impl_widget_common,
        widgets::{label::Label, row::Row, Widget, WidgetCommon, WidgetCommonTyped},
        WindowAttributes,
    },
    salvation_test_kit::context::Context,
};

pub struct RootWidget {
    common: WidgetCommon,
}

impl Widget for RootWidget {
    impl_widget_common!();

    fn new(mut common: WidgetCommonTyped<Self>) -> Self {
        let content = common
            .add_child_window::<Row>(0, WindowAttributes::default().with_title(module_path!()));

        content.add_child::<Label>().set_text("Test");

        Self {
            common: common.into(),
        }
    }
}

#[salvation_test_kit::test]
pub fn label(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.common_mut()
            .add_child::<RootWidget>(0, Default::default());
        Ok(())
    })?;
    let mut window = ctx.wait_for_window_by_pid()?;
    ctx.snapshot(&mut window, "label")?;
    window.close()?;
    Ok(())
}
