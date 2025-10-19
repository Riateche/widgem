use {
    std::ops::RangeInclusive,
    widgem::{
        impl_widget_base,
        shortcut::{KeyCombinations, Shortcut, ShortcutScope},
        types::Axis,
        widget_initializer,
        widgets::{Column, Label, Row, ScrollBar, Window},
        Widget, WidgetBaseOf, WidgetExt, WidgetInitializer,
    },
    widgem_tester::{Context, Key},
};

pub struct RootWidget {
    base: WidgetBaseOf<Self>,
    range: RangeInclusive<i32>,
    axis: Axis,
    focusable: bool,
    value: i32,
    placeholder_visible: bool,
}

impl RootWidget {
    fn on_scroll_bar_value_changed(&mut self, value: i32) -> anyhow::Result<()> {
        self.value = value;
        self.base.update();
        Ok(())
    }

    fn new(mut base: WidgetBaseOf<Self>) -> Self {
        let callbacks = base.callback_creator();
        base.add_shortcut(Shortcut::new(
            KeyCombinations::from_str_portable("R").unwrap(),
            ShortcutScope::Application,
            callbacks.create(|this, _| {
                this.axis = match this.axis {
                    Axis::X => Axis::Y,
                    Axis::Y => Axis::X,
                };
                this.base.update();
                Ok(())
            }),
        ));
        base.add_shortcut(Shortcut::new(
            KeyCombinations::from_str_portable("1; numpad1").unwrap(),
            ShortcutScope::Application,
            callbacks.create(|this, _| {
                this.range = 0..=10000;
                this.base.update();
                Ok(())
            }),
        ));
        base.add_shortcut(Shortcut::new(
            KeyCombinations::from_str_portable("f").unwrap(),
            ShortcutScope::Application,
            callbacks.create(|this, _| {
                this.focusable = !this.focusable;
                this.base.update();
                Ok(())
            }),
        ));
        base.add_shortcut(Shortcut::new(
            KeyCombinations::from_str_portable("t").unwrap(),
            ShortcutScope::Application,
            callbacks.create(|this, _| {
                this.placeholder_visible = !this.placeholder_visible;
                this.base.update();
                Ok(())
            }),
        ));

        RootWidget {
            base,
            range: 0..=100,
            axis: Axis::X,
            focusable: false,
            value: 0,
            placeholder_visible: false,
        }
    }

    pub fn init() -> impl WidgetInitializer<Output = Self> {
        widget_initializer::from_new(Self::new)
    }
}

impl Widget for RootWidget {
    impl_widget_base!();

    fn handle_declare_children_request(&mut self) -> anyhow::Result<()> {
        let callbacks = self.base.callback_creator();

        let mut window = self
            .base
            .set_child(0, Window::init(module_path!().into()))?
            .contents_mut();

        let mut row_items = window
            .set_next_item(Row::init())?
            .set_padding_enabled(false)
            .contents_mut();
        row_items
            .set_next_item(Label::init("placeholder".into()))?
            .set_visible(self.placeholder_visible);
        let mut column_items = row_items
            .set_next_item(Column::init())?
            .set_padding_enabled(false)
            .contents_mut();
        column_items
            .set_next_item(ScrollBar::init(self.axis))?
            .set_value_range(self.range.clone())
            .set_focusable(self.focusable)
            .set_value(self.value)
            .on_value_changed(callbacks.create(Self::on_scroll_bar_value_changed));

        column_items.set_next_item(Label::init(self.value.to_string()))?;

        Ok(())
    }
}

#[widgem_tester::test]
pub fn main(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().set_child(0, RootWidget::init())?;
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.mouse_move(0, 0)?;
    window.snapshot("scroll bar and label")?;
    window.resize(160, 66)?;
    window.snapshot("resized")?;

    window.close()?;
    Ok(())
}

#[widgem_tester::test]
pub fn keyboard(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().set_child(0, RootWidget::init())?;
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    window.snapshot("scroll bar")?;
    ctx.ui_context().key(Key::Unicode('f'))?;
    window.snapshot("focused")?;
    ctx.ui_context().key(Key::Unicode('1'))?;
    window.snapshot("increased range")?;

    ctx.ui_context().key(Key::DownArrow)?;
    window.snapshot("step down")?;
    ctx.ui_context().key(Key::DownArrow)?;
    window.snapshot("step down")?;

    ctx.ui_context().key(Key::PageDown)?;
    window.snapshot("page down")?;
    ctx.ui_context().key(Key::PageDown)?;
    window.snapshot("page down")?;

    ctx.ui_context().key(Key::UpArrow)?;
    window.snapshot("step up")?;
    ctx.ui_context().key(Key::UpArrow)?;
    window.snapshot("step up")?;

    ctx.ui_context().key(Key::PageUp)?;
    window.snapshot("page up")?;
    ctx.ui_context().key(Key::PageUp)?;
    window.snapshot("page up")?;

    ctx.ui_context().key(Key::End)?;
    window.snapshot("end")?;
    ctx.ui_context().key(Key::Home)?;
    window.snapshot("home")?;

    window.close()?;
    Ok(())
}

#[widgem_tester::test]
pub fn mouse_scroll(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().set_child(0, RootWidget::init())?;
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    window.snapshot("scroll bar")?;

    window.mouse_move(100, 20)?;
    window.snapshot("highlighted pager")?;

    //println!("scroll down 1");
    ctx.ui_context().mouse_scroll_down()?;
    window.snapshot("scrolled down")?;

    //println!("scroll down 2");
    ctx.ui_context().mouse_scroll_down()?;
    window.snapshot("scrolled down")?;

    //println!("scroll up 3");
    ctx.ui_context().mouse_scroll_up()?;
    window.snapshot("scrolled up")?;

    //println!("scroll up 4");
    ctx.ui_context().mouse_scroll_up()?;
    window.snapshot("scrolled up")?;

    //println!("scroll right 5");
    ctx.ui_context().mouse_scroll_right()?;
    window.snapshot("scrolled down")?;

    //println!("scroll right 6");
    ctx.ui_context().mouse_scroll_right()?;
    window.snapshot("scrolled down")?;

    //println!("scroll left 7");
    ctx.ui_context().mouse_scroll_left()?;
    window.snapshot("scrolled up")?;

    //println!("scroll left 8");
    ctx.ui_context().mouse_scroll_left()?;
    window.snapshot("scrolled up")?;

    window.close()?;
    Ok(())
}

#[widgem_tester::test]
pub fn pager(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().set_child(0, RootWidget::init())?;
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    window.snapshot("scroll bar")?;

    window.mouse_move(100, 20)?;
    window.snapshot("highlighted pager")?;
    ctx.ui_context().mouse_left_click()?;
    window.snapshot("page right")?;
    window.mouse_move(0, 0)?;
    window.snapshot("no highlight")?;

    window.mouse_move(43, 20)?;
    window.snapshot("highlighted pager")?;
    ctx.ui_context().mouse_left_click()?;
    window.snapshot("page left")?;
    window.mouse_move(0, 0)?;
    window.snapshot("no highlight")?;

    ctx.ui_context().key(Key::Unicode('1'))?;
    window.snapshot("increase range")?;
    window.mouse_move(100, 20)?;
    window.snapshot("highlighted pager")?;

    ctx.ui_context().mouse_left_press()?;
    window.snapshot("page right")?;
    ctx.ui_context().mouse_left_release()?;
    ctx.set_changing_expected(false);
    window.snapshot("released pager - no auto repeat")?;
    ctx.set_changing_expected(true);

    ctx.ui_context().mouse_left_press()?;
    window.snapshot("page right")?;
    window.snapshot("page right - first auto repeat")?;
    window.snapshot("page right - second auto repeat")?;
    ctx.ui_context().mouse_left_release()?;
    ctx.set_changing_expected(false);
    window.snapshot("released pager - no more auto repeats")?;
    ctx.set_changing_expected(true);

    window.close()?;
    Ok(())
}

#[widgem_tester::test]
pub fn resize(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().set_child(0, RootWidget::init())?;
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    ctx.ui_context().key(Key::Unicode('t'))?;
    window.resize(234, 66)?;
    window.snapshot("scroll bar")?;

    window.resize(1, 1)?;
    window.snapshot("min size")?;

    window.resize(274, 66)?;
    window.snapshot("resized")?;

    window.resize(374, 66)?;
    window.snapshot("resized")?;

    window.resize(374, 200)?;
    window.snapshot("no change - fixed y size")?;

    window.resize(374, 5)?;
    window.snapshot("min y size")?;

    ctx.ui_context().key(Key::Unicode('r'))?;
    window.snapshot("changed to vertical scroll bar")?;

    window.resize(1, 1)?;
    window.snapshot("min size")?;

    window.resize(274, 200)?;
    window.snapshot("resized")?;

    window.resize(274, 300)?;
    window.snapshot("resized")?;

    window.resize(1, 300)?;
    window.snapshot("min x size")?;

    window.close()?;
    Ok(())
}

#[widgem_tester::test]
pub fn right_arrow(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().set_child(0, RootWidget::init())?;
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    window.snapshot("scroll bar")?;
    window.mouse_move(142, 22)?;
    window.snapshot("highlighted right arrow")?;
    ctx.ui_context().mouse_left_press()?;
    window.snapshot("pressed right arrow - step right by 5")?;
    ctx.ui_context().mouse_left_release()?;
    window.snapshot("released right arrow - no auto repeat")?;

    ctx.ui_context().mouse_left_press()?;
    window.snapshot("pressed right arrow - step right by 5")?;
    window.snapshot("first auto repeat")?;
    window.snapshot("second auto repeat")?;
    window.snapshot("third auto repeat")?;
    ctx.ui_context().mouse_left_release()?;
    window.snapshot("released right arrow - no more auto repeats")?;

    window.close()?;
    Ok(())
}

#[widgem_tester::test]
pub fn slider_extremes(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().set_child(0, RootWidget::init())?;
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    window.snapshot("scroll bar")?;

    window.mouse_move(60, 20)?;
    window.snapshot("highlighted slider")?;
    ctx.ui_context().mouse_left_press()?;
    window.snapshot("grabbed slider")?;
    window.mouse_move(300, 24)?;
    window.snapshot("dragged all the way right")?;
    ctx.ui_context().mouse_left_release()?;
    window.snapshot("released slider - no highlight")?;

    window.mouse_move(90, 24)?;
    ctx.ui_context().mouse_left_press()?;
    window.snapshot("grabbed slider")?;
    window.mouse_move(0, 20)?;
    window.snapshot("dragged all the way left")?;
    window.mouse_move(20, 20)?;
    ctx.set_changing_expected(false);
    window.snapshot("still all the way left")?;
    ctx.set_changing_expected(true);
    window.mouse_move(58, 20)?;
    window.snapshot("no longer all the way left")?;
    ctx.ui_context().mouse_left_release()?;
    window.snapshot("released slider")?;

    window.close()?;
    Ok(())
}

#[widgem_tester::test]
pub fn slider(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().set_child(0, RootWidget::init())?;
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    window.snapshot("scroll bar")?;

    window.mouse_move(40, 20)?;
    window.snapshot("highlighted slider")?;
    ctx.ui_context().mouse_left_press()?;
    window.snapshot("grabbed slider")?;
    window.mouse_move(50, 20)?;
    window.snapshot("moved slider by 10 px")?;
    ctx.ui_context().mouse_left_release()?;
    window.snapshot("released slider")?;
    window.mouse_move(15, 15)?;
    window.snapshot("highlighted left arrow")?;
    ctx.ui_context().mouse_left_click()?;
    window.snapshot("step left by 5")?;

    window.mouse_move(60, 18)?;
    window.snapshot("highlighted slider")?;
    ctx.ui_context().mouse_left_press()?;
    window.snapshot("grabbed slider")?;
    window.mouse_move(60, 88)?;
    window.snapshot("dragged down and outside - no effect")?;
    window.mouse_move(50, 88)?;
    window.snapshot("dragged left")?;
    ctx.ui_context().mouse_left_release()?;
    window.snapshot("released slider - no highlight")?;

    window.close()?;
    Ok(())
}
