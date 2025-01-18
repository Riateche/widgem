use {
    salvation::{
        impl_widget_common,
        layout::LayoutItemOptions,
        widgets::{label::Label, Widget, WidgetCommon, WidgetExt},
        WindowAttributes,
    },
    salvation_test_kit::context::Context,
};

pub struct RootWidget {
    common: WidgetCommon,
}

impl RootWidget {
    pub fn new() -> Self {
        let mut common = WidgetCommon::new::<Self>();
        let input = Label::new("Test");
        common.add_child(input.boxed(), LayoutItemOptions::from_pos_in_grid(0, 0));
        Self {
            common: common.into(),
        }
        .with_window(WindowAttributes::default().with_title(module_path!()))
    }
}

impl Widget for RootWidget {
    impl_widget_common!();
}

#[salvation_test_kit::test]
pub fn label(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|| RootWidget::new().boxed())?;
    let mut window = ctx.wait_for_window_by_pid()?;
    ctx.snapshot(&mut window, "label")?;
    window.close()?;
    Ok(())
}
