use {
    salvation::{
        impl_widget_common,
        widgets::{label::Label, padding_box::PaddingBox, Widget, WidgetCommon, WidgetExt},
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
        common.add_child(
            PaddingBox::new(input.boxed())
                .with_window(WindowAttributes::default().with_title(module_path!()))
                .boxed(),
            Default::default(),
        );
        Self {
            common: common.into(),
        }
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
