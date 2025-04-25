use salvation::{
    impl_widget_common,
    widgets::{
        button::Button, menu::Menu, window::WindowWidget, Widget, WidgetCommon, WidgetCommonTyped,
        WidgetExt,
    },
};
use salvation_test_kit::context::Context;

pub struct RootWidget {
    common: WidgetCommon,
}

impl RootWidget {
    fn on_triggered(&mut self, _event: ()) -> anyhow::Result<()> {
        self.common.child::<Menu>(1);
        Ok(())
    }
}

impl Widget for RootWidget {
    impl_widget_common!();

    fn new(mut common: WidgetCommonTyped<Self>) -> Self {
        let id = common.id();
        let window = common
            .add_child::<WindowWidget>(0)
            .set_title(module_path!());

        window
            .common_mut()
            .child::<Button>(0)
            .set_column(0)
            .set_row(0)
            .set_text("test")
            .on_triggered(id.callback(Self::on_triggered));

        Self {
            common: common.into(),
        }
    }
}

#[salvation_test_kit::test]
fn menu(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.common_mut().child::<RootWidget>(0);
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.close()?;
    Ok(())
}
