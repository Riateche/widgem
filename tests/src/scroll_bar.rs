use {
    salvation::{
        impl_widget_common,
        layout::LayoutItemOptions,
        shortcut::{KeyCombinations, Shortcut, ShortcutScope},
        types::Axis,
        widgets::{label::Label, scroll_bar::ScrollBar, Widget, WidgetCommon, WidgetExt, WidgetId},
        WindowAttributes,
    },
    salvation_test_kit::context::Context,
};

pub struct RootWidget {
    common: WidgetCommon,
    label_id: WidgetId<Label>,
    scroll_bar_id: WidgetId<ScrollBar>,
}

impl RootWidget {
    pub fn new() -> Self {
        let mut common = WidgetCommon::new::<Self>();

        let value = 0;
        let label = Label::new(value.to_string()).with_id();
        let scroll_bar = ScrollBar::new()
            .with_on_value_changed(common.callback(Self::on_scroll_bar_value_changed))
            .with_value(value)
            .with_id();
        common.add_child(
            scroll_bar.widget.boxed(),
            LayoutItemOptions::from_pos_in_grid(0, 0),
        );
        common.add_child(
            label.widget.boxed(),
            LayoutItemOptions::from_pos_in_grid(0, 1),
        );

        let mut this = Self {
            common: common.into(),
            label_id: label.id,
            scroll_bar_id: scroll_bar.id,
        }
        .with_window(WindowAttributes::default().with_title(module_path!()));

        let on_r = this.callback(|this, _| {
            let scroll_bar = this.common.widget(this.scroll_bar_id)?;
            match scroll_bar.axis() {
                Axis::X => scroll_bar.set_axis(Axis::Y),
                Axis::Y => scroll_bar.set_axis(Axis::X),
            }
            Ok(())
        });
        let on_1 = this.callback(|this, _| {
            let scroll_bar = this.common.widget(this.scroll_bar_id)?;
            scroll_bar.set_value_range(0..=10000);
            Ok(())
        });
        let on_f = this.callback(|this, _| {
            let scroll_bar = this.common.widget(this.scroll_bar_id)?;
            let focusable = scroll_bar.common().is_focusable();
            scroll_bar.common_mut().set_focusable(!focusable);
            Ok(())
        });
        this.common.add_shortcut(Shortcut::new(
            KeyCombinations::from_str_portable("R").unwrap(),
            ShortcutScope::Application,
            on_r,
        ));
        this.common.add_shortcut(Shortcut::new(
            KeyCombinations::from_str_portable("1").unwrap(),
            ShortcutScope::Application,
            on_1,
        ));
        this.common.add_shortcut(Shortcut::new(
            KeyCombinations::from_str_portable("f").unwrap(),
            ShortcutScope::Application,
            on_f,
        ));
        this
    }

    fn on_scroll_bar_value_changed(&mut self, value: i32) -> anyhow::Result<()> {
        self.common
            .widget(self.label_id)?
            .set_text(value.to_string());
        Ok(())
    }
}

impl Widget for RootWidget {
    impl_widget_common!();
}

#[salvation_test_kit::test]
pub fn basic(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|| RootWidget::new().boxed())?;
    let mut window = ctx.wait_for_window_by_pid()?;
    window.mouse_move(0, 0)?;
    ctx.snapshot(&mut window, "scroll bar and label")?;
    window.resize(160, 66)?;
    ctx.snapshot(&mut window, "resized")?;

    window.close()?;
    Ok(())
}

#[salvation_test_kit::test]
pub fn keyboard(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|| RootWidget::new().boxed())?;
    let mut window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    ctx.snapshot(&mut window, "scroll bar")?;
    ctx.connection().key("f")?;
    ctx.snapshot(&mut window, "focused")?;
    ctx.connection().key("1")?;
    ctx.snapshot(&mut window, "increased range")?;

    ctx.connection().key("Down")?;
    ctx.snapshot(&mut window, "step down")?;
    ctx.connection().key("Down")?;
    ctx.snapshot(&mut window, "step down")?;

    ctx.connection().key("Page_Down")?;
    ctx.snapshot(&mut window, "page down")?;
    ctx.connection().key("Page_Down")?;
    ctx.snapshot(&mut window, "page down")?;

    ctx.connection().key("Up")?;
    ctx.snapshot(&mut window, "step up")?;
    ctx.connection().key("Up")?;
    ctx.snapshot(&mut window, "step up")?;

    ctx.connection().key("Page_Up")?;
    ctx.snapshot(&mut window, "page up")?;
    ctx.connection().key("Page_Up")?;
    ctx.snapshot(&mut window, "page up")?;

    ctx.connection().key("End")?;
    ctx.snapshot(&mut window, "end")?;
    ctx.connection().key("Home")?;
    ctx.snapshot(&mut window, "home")?;

    window.close()?;
    Ok(())
}

#[salvation_test_kit::test]
pub fn mouse_scroll(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|| RootWidget::new().boxed())?;
    let mut window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    ctx.snapshot(&mut window, "scroll bar")?;

    window.mouse_move(100, 20)?;
    ctx.snapshot(&mut window, "highlighted pager")?;

    ctx.connection().mouse_scroll_down()?;
    ctx.snapshot(&mut window, "scrolled down")?;

    ctx.connection().mouse_scroll_down()?;
    ctx.snapshot(&mut window, "scrolled down")?;

    ctx.connection().mouse_scroll_up()?;
    ctx.snapshot(&mut window, "scrolled up")?;

    ctx.connection().mouse_scroll_up()?;
    ctx.snapshot(&mut window, "scrolled up")?;

    ctx.connection().mouse_scroll_right()?;
    ctx.snapshot(&mut window, "scrolled down")?;

    ctx.connection().mouse_scroll_right()?;
    ctx.snapshot(&mut window, "scrolled down")?;

    ctx.connection().mouse_scroll_left()?;
    ctx.snapshot(&mut window, "scrolled up")?;

    ctx.connection().mouse_scroll_left()?;
    ctx.snapshot(&mut window, "scrolled up")?;

    window.close()?;
    Ok(())
}

#[salvation_test_kit::test]
pub fn pager(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|| RootWidget::new().boxed())?;
    let mut window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    ctx.snapshot(&mut window, "scroll bar")?;

    window.mouse_move(100, 20)?;
    ctx.snapshot(&mut window, "highlighted pager")?;
    ctx.connection().mouse_click(1)?;
    ctx.snapshot(&mut window, "page right")?;
    window.mouse_move(0, 0)?;
    ctx.snapshot(&mut window, "no highlight")?;

    window.mouse_move(43, 20)?;
    ctx.snapshot(&mut window, "highlighted pager")?;
    ctx.connection().mouse_click(1)?;
    ctx.snapshot(&mut window, "page left")?;
    window.mouse_move(0, 0)?;
    ctx.snapshot(&mut window, "no highlight")?;

    ctx.connection().key("1")?;
    ctx.snapshot(&mut window, "increase range")?;
    window.mouse_move(100, 20)?;
    ctx.snapshot(&mut window, "highlighted pager")?;

    ctx.connection().mouse_down(1)?;
    ctx.snapshot(&mut window, "page right")?;
    ctx.connection().mouse_up(1)?;
    ctx.set_changing_expected(false);
    ctx.snapshot(&mut window, "released pager - no auto repeat")?;
    ctx.set_changing_expected(true);

    ctx.connection().mouse_down(1)?;
    ctx.snapshot(&mut window, "page right")?;
    ctx.snapshot(&mut window, "page right - first auto repeat")?;
    ctx.snapshot(&mut window, "page right - second auto repeat")?;
    ctx.connection().mouse_up(1)?;
    ctx.set_changing_expected(false);
    ctx.snapshot(&mut window, "released pager - no more auto repeats")?;
    ctx.set_changing_expected(true);

    window.close()?;
    Ok(())
}

#[salvation_test_kit::test]
pub fn resize(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|| RootWidget::new().boxed())?;
    let mut window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    ctx.snapshot(&mut window, "scroll bar")?;

    window.resize(1, 1)?;
    ctx.snapshot(&mut window, "min size")?;

    window.resize(200, 66)?;
    ctx.snapshot(&mut window, "resized")?;

    window.resize(300, 66)?;
    ctx.snapshot(&mut window, "resized")?;

    window.resize(300, 200)?;
    ctx.snapshot(&mut window, "no change - fixed y size")?;

    window.resize(300, 5)?;
    ctx.snapshot(&mut window, "min y size")?;

    ctx.connection().key("r")?;
    ctx.snapshot(&mut window, "changed to vertical scroll bar")?;

    window.resize(1, 1)?;
    ctx.snapshot(&mut window, "min size")?;

    window.resize(200, 200)?;
    ctx.snapshot(&mut window, "resized")?;

    window.resize(200, 300)?;
    ctx.snapshot(&mut window, "resized")?;

    window.resize(1, 300)?;
    ctx.snapshot(&mut window, "min x size")?;

    window.close()?;
    Ok(())
}

#[salvation_test_kit::test]
pub fn right_arrow(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|| RootWidget::new().boxed())?;
    ctx.connection().mouse_move_global(0, 0)?;
    let mut window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    ctx.snapshot(&mut window, "scroll bar")?;
    window.mouse_move(142, 22)?;
    ctx.snapshot(&mut window, "highlighted right arrow")?;
    ctx.connection().mouse_down(1)?;
    ctx.snapshot(&mut window, "pressed right arrow - step right by 5")?;
    ctx.connection().mouse_up(1)?;
    ctx.snapshot(&mut window, "released right arrow - no auto repeat")?;

    ctx.connection().mouse_down(1)?;
    ctx.snapshot(&mut window, "pressed right arrow - step right by 5")?;
    ctx.snapshot(&mut window, "first auto repeat")?;
    ctx.snapshot(&mut window, "second auto repeat")?;
    ctx.snapshot(&mut window, "third auto repeat")?;
    ctx.connection().mouse_up(1)?;
    ctx.snapshot(&mut window, "released right arrow - no more auto repeats")?;

    window.close()?;
    Ok(())
}

#[salvation_test_kit::test]
pub fn slider_extremes(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|| RootWidget::new().boxed())?;
    let mut window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    ctx.snapshot(&mut window, "scroll bar")?;

    window.mouse_move(60, 20)?;
    ctx.snapshot(&mut window, "highlighted slider")?;
    ctx.connection().mouse_down(1)?;
    ctx.snapshot(&mut window, "grabbed slider")?;
    window.mouse_move(300, 24)?;
    ctx.snapshot(&mut window, "dragged all the way right")?;
    ctx.connection().mouse_up(1)?;
    ctx.snapshot(&mut window, "released slider - no highlight")?;

    window.mouse_move(90, 24)?;
    ctx.connection().mouse_down(1)?;
    ctx.snapshot(&mut window, "grabbed slider")?;
    window.mouse_move(0, 20)?;
    ctx.snapshot(&mut window, "dragged all the way left")?;
    window.mouse_move(20, 20)?;
    ctx.set_changing_expected(false);
    ctx.snapshot(&mut window, "still all the way left")?;
    ctx.set_changing_expected(true);
    window.mouse_move(58, 20)?;
    ctx.snapshot(&mut window, "no longer all the way left")?;
    ctx.connection().mouse_up(1)?;
    ctx.snapshot(&mut window, "released slider")?;

    window.close()?;
    Ok(())
}

#[salvation_test_kit::test]
pub fn slider(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|| RootWidget::new().boxed())?;
    let mut window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    ctx.snapshot(&mut window, "scroll bar")?;

    window.mouse_move(40, 20)?;
    ctx.snapshot(&mut window, "highlighted slider")?;
    ctx.connection().mouse_down(1)?;
    ctx.snapshot(&mut window, "grabbed slider")?;
    window.mouse_move(50, 20)?;
    ctx.snapshot(&mut window, "moved slider by 10 px")?;
    ctx.connection().mouse_up(1)?;
    ctx.snapshot(&mut window, "released slider")?;
    window.mouse_move(15, 15)?;
    ctx.snapshot(&mut window, "highlighted left arrow")?;
    ctx.connection().mouse_click(1)?;
    ctx.snapshot(&mut window, "step left by 5")?;

    window.mouse_move(60, 18)?;
    ctx.snapshot(&mut window, "highlighted slider")?;
    ctx.connection().mouse_down(1)?;
    ctx.snapshot(&mut window, "grabbed slider")?;
    window.mouse_move(60, 88)?;
    ctx.snapshot(&mut window, "dragged down and outside - no effect")?;
    window.mouse_move(50, 88)?;
    ctx.snapshot(&mut window, "dragged left")?;
    ctx.connection().mouse_up(1)?;
    ctx.snapshot(&mut window, "released slider - no highlight")?;

    window.close()?;
    Ok(())
}
