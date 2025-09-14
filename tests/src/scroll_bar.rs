use {
    std::ops::RangeInclusive,
    widgem::{
        impl_widget_base,
        shortcut::{KeyCombinations, Shortcut, ShortcutScope},
        types::Axis,
        widgets::{Label, ScrollBar, Widget, WidgetBaseOf, WidgetExt, WidgetInitializer, Window},
    },
    widgem_tester::context::Context,
};

pub struct RootWidget {
    base: WidgetBaseOf<Self>,
    range: RangeInclusive<i32>,
    axis: Axis,
    focusable: bool,
    value: i32,
}

impl RootWidget {
    fn on_scroll_bar_value_changed(&mut self, value: i32) -> anyhow::Result<()> {
        self.value = value;
        self.base.update();
        Ok(())
    }

    pub fn init() -> impl WidgetInitializer<Output = Self> {
        Initializer
    }
}

struct Initializer;

impl WidgetInitializer for Initializer {
    type Output = RootWidget;

    fn init(self, mut base: WidgetBaseOf<Self::Output>) -> Self::Output {
        let on_r = base.callback(|this, _| {
            this.axis = match this.axis {
                Axis::X => Axis::Y,
                Axis::Y => Axis::X,
            };
            this.base.update();
            Ok(())
        });
        let on_1 = base.callback(|this, _| {
            this.range = 0..=10000;
            this.base.update();
            Ok(())
        });
        let on_f = base.callback(|this, _| {
            this.focusable = !this.focusable;
            this.base.update();
            Ok(())
        });
        base.add_shortcut(Shortcut::new(
            KeyCombinations::from_str_portable("R").unwrap(),
            ShortcutScope::Application,
            on_r,
        ));
        base.add_shortcut(Shortcut::new(
            KeyCombinations::from_str_portable("1").unwrap(),
            ShortcutScope::Application,
            on_1,
        ));
        base.add_shortcut(Shortcut::new(
            KeyCombinations::from_str_portable("f").unwrap(),
            ShortcutScope::Application,
            on_f,
        ));

        RootWidget {
            base,
            range: 0..=100,
            axis: Axis::X,
            focusable: false,
            value: 0,
        }
    }

    fn reinit(self, _widget: &mut Self::Output) {}
}

impl Widget for RootWidget {
    impl_widget_base!();

    fn handle_declare_children_request(&mut self) -> anyhow::Result<()> {
        let callbacks = self.base.callback_creator();

        let mut window = self
            .base
            .set_child(0, Window::init(module_path!().into()))
            .items_mut();

        window
            .set_next_item(ScrollBar::init(self.axis))
            .set_value_range(self.range.clone())
            .set_focusable(self.focusable)
            .set_value(self.value)
            .on_value_changed(callbacks.create(Self::on_scroll_bar_value_changed));

        window.set_next_item(Label::init(self.value.to_string()));

        Ok(())
    }
}

#[widgem_tester::test]
pub fn basic(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().set_child(0, RootWidget::init());
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
        r.base_mut().set_child(0, RootWidget::init());
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    window.snapshot("scroll bar")?;
    ctx.ui().key("f")?;
    window.snapshot("focused")?;
    ctx.ui().key("1")?;
    window.snapshot("increased range")?;

    ctx.ui().key("Down")?;
    window.snapshot("step down")?;
    ctx.ui().key("Down")?;
    window.snapshot("step down")?;

    ctx.ui().key("Page_Down")?;
    window.snapshot("page down")?;
    ctx.ui().key("Page_Down")?;
    window.snapshot("page down")?;

    ctx.ui().key("Up")?;
    window.snapshot("step up")?;
    ctx.ui().key("Up")?;
    window.snapshot("step up")?;

    ctx.ui().key("Page_Up")?;
    window.snapshot("page up")?;
    ctx.ui().key("Page_Up")?;
    window.snapshot("page up")?;

    ctx.ui().key("End")?;
    window.snapshot("end")?;
    ctx.ui().key("Home")?;
    window.snapshot("home")?;

    window.close()?;
    Ok(())
}

#[widgem_tester::test]
pub fn mouse_scroll(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().set_child(0, RootWidget::init());
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    window.snapshot("scroll bar")?;

    window.mouse_move(100, 20)?;
    window.snapshot("highlighted pager")?;

    ctx.ui().mouse_scroll_down()?;
    window.snapshot("scrolled down")?;

    ctx.ui().mouse_scroll_down()?;
    window.snapshot("scrolled down")?;

    ctx.ui().mouse_scroll_up()?;
    window.snapshot("scrolled up")?;

    ctx.ui().mouse_scroll_up()?;
    window.snapshot("scrolled up")?;

    ctx.ui().mouse_scroll_right()?;
    window.snapshot("scrolled down")?;

    ctx.ui().mouse_scroll_right()?;
    window.snapshot("scrolled down")?;

    ctx.ui().mouse_scroll_left()?;
    window.snapshot("scrolled up")?;

    ctx.ui().mouse_scroll_left()?;
    window.snapshot("scrolled up")?;

    window.close()?;
    Ok(())
}

#[widgem_tester::test]
pub fn pager(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().set_child(0, RootWidget::init());
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    window.snapshot("scroll bar")?;

    window.mouse_move(100, 20)?;
    window.snapshot("highlighted pager")?;
    ctx.ui().mouse_click(1)?;
    window.snapshot("page right")?;
    window.mouse_move(0, 0)?;
    window.snapshot("no highlight")?;

    window.mouse_move(43, 20)?;
    window.snapshot("highlighted pager")?;
    ctx.ui().mouse_click(1)?;
    window.snapshot("page left")?;
    window.mouse_move(0, 0)?;
    window.snapshot("no highlight")?;

    ctx.ui().key("1")?;
    window.snapshot("increase range")?;
    window.mouse_move(100, 20)?;
    window.snapshot("highlighted pager")?;

    ctx.ui().mouse_down(1)?;
    window.snapshot("page right")?;
    ctx.ui().mouse_up(1)?;
    ctx.set_changing_expected(false);
    window.snapshot("released pager - no auto repeat")?;
    ctx.set_changing_expected(true);

    ctx.ui().mouse_down(1)?;
    window.snapshot("page right")?;
    window.snapshot("page right - first auto repeat")?;
    window.snapshot("page right - second auto repeat")?;
    ctx.ui().mouse_up(1)?;
    ctx.set_changing_expected(false);
    window.snapshot("released pager - no more auto repeats")?;
    ctx.set_changing_expected(true);

    window.close()?;
    Ok(())
}

#[widgem_tester::test]
pub fn resize(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().set_child(0, RootWidget::init());
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    window.snapshot("scroll bar")?;

    window.resize(1, 1)?;
    window.snapshot("min size")?;

    window.resize(200, 66)?;
    window.snapshot("resized")?;

    window.resize(300, 66)?;
    window.snapshot("resized")?;

    window.resize(300, 200)?;
    window.snapshot("no change - fixed y size")?;

    window.resize(300, 5)?;
    window.snapshot("min y size")?;

    ctx.ui().key("r")?;
    window.snapshot("changed to vertical scroll bar")?;

    window.resize(1, 1)?;
    window.snapshot("min size")?;

    window.resize(200, 200)?;
    window.snapshot("resized")?;

    window.resize(200, 300)?;
    window.snapshot("resized")?;

    window.resize(1, 300)?;
    window.snapshot("min x size")?;

    window.close()?;
    Ok(())
}

#[widgem_tester::test]
pub fn right_arrow(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().set_child(0, RootWidget::init());
        Ok(())
    })?;
    ctx.ui().mouse_move_global(0, 0)?;
    let window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    window.snapshot("scroll bar")?;
    window.mouse_move(142, 22)?;
    window.snapshot("highlighted right arrow")?;
    ctx.ui().mouse_down(1)?;
    window.snapshot("pressed right arrow - step right by 5")?;
    ctx.ui().mouse_up(1)?;
    window.snapshot("released right arrow - no auto repeat")?;

    ctx.ui().mouse_down(1)?;
    window.snapshot("pressed right arrow - step right by 5")?;
    window.snapshot("first auto repeat")?;
    window.snapshot("second auto repeat")?;
    window.snapshot("third auto repeat")?;
    ctx.ui().mouse_up(1)?;
    window.snapshot("released right arrow - no more auto repeats")?;

    window.close()?;
    Ok(())
}

#[widgem_tester::test]
pub fn slider_extremes(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().set_child(0, RootWidget::init());
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    window.snapshot("scroll bar")?;

    window.mouse_move(60, 20)?;
    window.snapshot("highlighted slider")?;
    ctx.ui().mouse_down(1)?;
    window.snapshot("grabbed slider")?;
    window.mouse_move(300, 24)?;
    window.snapshot("dragged all the way right")?;
    ctx.ui().mouse_up(1)?;
    window.snapshot("released slider - no highlight")?;

    window.mouse_move(90, 24)?;
    ctx.ui().mouse_down(1)?;
    window.snapshot("grabbed slider")?;
    window.mouse_move(0, 20)?;
    window.snapshot("dragged all the way left")?;
    window.mouse_move(20, 20)?;
    ctx.set_changing_expected(false);
    window.snapshot("still all the way left")?;
    ctx.set_changing_expected(true);
    window.mouse_move(58, 20)?;
    window.snapshot("no longer all the way left")?;
    ctx.ui().mouse_up(1)?;
    window.snapshot("released slider")?;

    window.close()?;
    Ok(())
}

#[widgem_tester::test]
pub fn slider(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().set_child(0, RootWidget::init());
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    window.snapshot("scroll bar")?;

    window.mouse_move(40, 20)?;
    window.snapshot("highlighted slider")?;
    ctx.ui().mouse_down(1)?;
    window.snapshot("grabbed slider")?;
    window.mouse_move(50, 20)?;
    window.snapshot("moved slider by 10 px")?;
    ctx.ui().mouse_up(1)?;
    window.snapshot("released slider")?;
    window.mouse_move(15, 15)?;
    window.snapshot("highlighted left arrow")?;
    ctx.ui().mouse_click(1)?;
    window.snapshot("step left by 5")?;

    window.mouse_move(60, 18)?;
    window.snapshot("highlighted slider")?;
    ctx.ui().mouse_down(1)?;
    window.snapshot("grabbed slider")?;
    window.mouse_move(60, 88)?;
    window.snapshot("dragged down and outside - no effect")?;
    window.mouse_move(50, 88)?;
    window.snapshot("dragged left")?;
    ctx.ui().mouse_up(1)?;
    window.snapshot("released slider - no highlight")?;

    window.close()?;
    Ok(())
}
