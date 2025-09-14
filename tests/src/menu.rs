use {
    anyhow::{ensure, Context as _},
    widgem::{
        impl_widget_base,
        widgets::{Button, Menu, MenuAction, Widget, WidgetBaseOf, WidgetInitializer, Window},
        WidgetExt, WidgetId,
    },
    widgem_tester::context::Context,
};

pub struct RootWidget {
    base: WidgetBaseOf<Self>,
    button_id: WidgetId<Button>,
}

const KEY_BUTTON: u32 = 0;
const KEY_MENU: u32 = 1;

impl RootWidget {
    fn on_triggered(&mut self, _event: ()) -> anyhow::Result<()> {
        let button = self.base.find_child(self.button_id)?;
        let rect = button.base().rect_in_window_or_err()?;
        let window = button.base().window_or_err()?;
        let pos_in_window = window
            .cursor_position()
            .unwrap_or_else(|| rect.bottom_right());
        let global_pos = window.inner_position()? + pos_in_window;

        let mut menu = self
            .base
            .set_child(KEY_MENU, Menu::init(global_pos))
            .items_mut();
        menu.set_next_item(MenuAction::init("Item 1".into()));
        menu.set_next_item(MenuAction::init("Item 2".into()));
        menu.set_next_item(MenuAction::init("Long item 3".into()));
        // for i in 4..100 {
        //     menu.base_mut().add_child::<MenuItem>(format!("Item {i}"));
        // }
        Ok(())
    }
}

impl RootWidget {
    pub fn init() -> impl WidgetInitializer<Output = Self> {
        Initializer
    }
}

struct Initializer;

impl WidgetInitializer for Initializer {
    type Output = RootWidget;

    fn init(self, mut base: WidgetBaseOf<Self::Output>) -> Self::Output {
        let callbacks = base.callback_creator();

        let window = base.set_child(0, Window::init(module_path!().into()));

        let button_id = window
            .base_mut()
            .set_child(KEY_BUTTON, Button::init("Open menu".into()))
            .on_triggered(callbacks.create(RootWidget::on_triggered))
            .id();

        RootWidget { base, button_id }
    }

    fn reinit(self, _widget: &mut Self::Output) {}
}

impl Widget for RootWidget {
    impl_widget_base!();
}

#[widgem_tester::test]
fn menu(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.items_mut().set_next_item(RootWidget::init());
        Ok(())
    })?;
    let main_window = ctx.wait_for_window_by_pid()?;
    main_window.snapshot("main window")?;
    main_window.mouse_move(50, 30)?;
    ctx.ui().mouse_left_click()?;
    let windows = ctx.wait_for_windows_by_pid(2)?;
    ensure!(
        windows.iter().any(|w| w.id() == main_window.id()),
        "no main window"
    );
    let menu_window = windows
        .into_iter()
        .find(|w| w.id() != main_window.id())
        .context("no non-main window")?;
    main_window.snapshot("main window after opening menu")?;
    menu_window.snapshot("menu")?;
    menu_window.mouse_move(60, 50)?;
    menu_window.snapshot("select second item")?;
    main_window.mouse_move(1, 1)?;
    ctx.ui().mouse_left_click()?;
    let window2 = ctx.wait_for_window_by_pid()?;
    ensure!(window2.id() == main_window.id(), "no main window");
    main_window.snapshot("main window after closing menu")?;

    main_window.close()?;
    Ok(())
}
