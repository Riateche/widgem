use salvation::{
    impl_widget_common,
    widgets::{button::Button, menu::Menu, row::Row, Widget, WidgetCommon, WidgetCommonTyped},
    WindowAttributes,
};
use salvation_test_kit::context::Context;

pub struct RootWidget {
    common: WidgetCommon,
}

impl RootWidget {
    fn on_triggered(&mut self, _event: ()) -> anyhow::Result<()> {
        self.common.add_child::<Menu>(Default::default());
        Ok(())
    }
}

impl Widget for RootWidget {
    impl_widget_common!();

    fn new(mut common: WidgetCommonTyped<Self>) -> Self {
        let id = common.id();
        let content =
            common.add_child_window::<Row>(WindowAttributes::default().with_title(module_path!()));

        content
            .add_child::<Button>()
            .set_text("test")
            .on_triggered(id.callback(Self::on_triggered));

        Self {
            common: common.into(),
        }
    }
}

#[salvation_test_kit::test]
fn menu(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run::<RootWidget>(|_| Ok(()))?;
    let window = ctx.wait_for_window_by_pid()?;
    window.close()?;
    Ok(())
}
