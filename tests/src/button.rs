use {
    salvation::{
        impl_widget_common,
        layout::LayoutItemOptions,
        widgets::{button::Button, window::WindowWidget, Widget, WidgetCommon, WidgetCommonTyped},
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
            .add_child::<WindowWidget>(0, Default::default())
            .set_title(module_path!());

        window
            .common_mut()
            .add_child::<Button>(0, LayoutItemOptions::from_pos_in_grid(0, 0))
            .set_text("Test");

        Self {
            common: common.into(),
        }
    }
}

#[salvation_test_kit::test]
pub fn button(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.common_mut()
            .add_child::<RootWidget>(0, Default::default());
        Ok(())
    })?;
    let mut window = ctx.wait_for_window_by_pid()?;
    ctx.snapshot(&mut window, "button")?;
    window.close()?;
    Ok(())
}
