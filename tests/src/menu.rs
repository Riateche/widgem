use salvation::{
    impl_widget_common,
    layout::LayoutItemOptions,
    widgets::{button::Button, menu::Menu, Widget, WidgetCommon, WidgetExt},
    WindowAttributes,
};
use salvation_test_kit::context::Context;

pub struct RootWidget {
    common: WidgetCommon,
}

impl RootWidget {
    pub fn new() -> Self {
        let mut common = WidgetCommon::new::<Self>();
        let button = Button::new("test").with_on_triggered(common.callback(Self::on_triggered));
        common.add_child(button.boxed(), LayoutItemOptions::from_pos_in_grid(0, 0));
        Self {
            common: common.into(),
        }
        .with_window(WindowAttributes::default().with_title(module_path!()))
    }

    fn on_triggered(&mut self, _event: String) -> anyhow::Result<()> {
        let menu = Menu::new();
        self.common.add_child(menu.boxed(), Default::default());
        Ok(())
    }
}

impl Widget for RootWidget {
    impl_widget_common!();
}

#[salvation_test_kit::test]
fn menu(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|| RootWidget::new().boxed())?;
    let window = ctx.wait_for_window_by_pid()?;
    window.close()?;
    Ok(())
}
