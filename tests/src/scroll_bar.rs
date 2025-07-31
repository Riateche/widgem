use {
    std::ops::RangeInclusive,
    widgem::{
        impl_widget_base,
        shortcut::{KeyCombinations, Shortcut, ShortcutScope},
        types::Axis,
        widgets::{Label, NewWidget, ScrollBar, Widget, WidgetBaseOf, WidgetExt, Window},
    },
    widgem_test_kit::context::Context,
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
}

impl NewWidget for RootWidget {
    type Arg = ();

    fn new(mut base: WidgetBaseOf<Self>, (): Self::Arg) -> Self {
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

        Self {
            base,
            range: 0..=100,
            axis: Axis::X,
            focusable: false,
            value: 0,
        }
    }
    fn handle_declared(&mut self, (): Self::Arg) {}
}

impl Widget for RootWidget {
    impl_widget_base!();

    fn handle_declare_children_request(&mut self) -> anyhow::Result<()> {
        let id = self.base.id();

        let window = self.base.declare_child::<Window>(module_path!().into());

        window
            .base_mut()
            .declare_child::<ScrollBar>(self.axis)
            .set_value_range(self.range.clone())
            .set_focusable(self.focusable)
            .set_value(self.value)
            .on_value_changed(id.callback(Self::on_scroll_bar_value_changed));

        window
            .base_mut()
            .declare_child::<Label>(self.value.to_string());

        Ok(())
    }
}

#[widgem_test_kit::test]
pub fn basic(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().add_child::<RootWidget>(());
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.mouse_move(0, 0)?;
    ctx.snapshot(&window, "scroll bar and label")?;
    window.resize(160, 66)?;
    ctx.snapshot(&window, "resized")?;

    window.close()?;
    Ok(())
}

#[widgem_test_kit::test]
pub fn keyboard(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().add_child::<RootWidget>(());
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    ctx.snapshot(&window, "scroll bar")?;
    ctx.connection().key("f")?;
    ctx.snapshot(&window, "focused")?;
    ctx.connection().key("1")?;
    ctx.snapshot(&window, "increased range")?;

    ctx.connection().key("Down")?;
    ctx.snapshot(&window, "step down")?;
    ctx.connection().key("Down")?;
    ctx.snapshot(&window, "step down")?;

    ctx.connection().key("Page_Down")?;
    ctx.snapshot(&window, "page down")?;
    ctx.connection().key("Page_Down")?;
    ctx.snapshot(&window, "page down")?;

    ctx.connection().key("Up")?;
    ctx.snapshot(&window, "step up")?;
    ctx.connection().key("Up")?;
    ctx.snapshot(&window, "step up")?;

    ctx.connection().key("Page_Up")?;
    ctx.snapshot(&window, "page up")?;
    ctx.connection().key("Page_Up")?;
    ctx.snapshot(&window, "page up")?;

    ctx.connection().key("End")?;
    ctx.snapshot(&window, "end")?;
    ctx.connection().key("Home")?;
    ctx.snapshot(&window, "home")?;

    window.close()?;
    Ok(())
}

#[widgem_test_kit::test]
pub fn mouse_scroll(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().add_child::<RootWidget>(());
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    ctx.snapshot(&window, "scroll bar")?;

    window.mouse_move(100, 20)?;
    ctx.snapshot(&window, "highlighted pager")?;

    ctx.connection().mouse_scroll_down()?;
    ctx.snapshot(&window, "scrolled down")?;

    ctx.connection().mouse_scroll_down()?;
    ctx.snapshot(&window, "scrolled down")?;

    ctx.connection().mouse_scroll_up()?;
    ctx.snapshot(&window, "scrolled up")?;

    ctx.connection().mouse_scroll_up()?;
    ctx.snapshot(&window, "scrolled up")?;

    ctx.connection().mouse_scroll_right()?;
    ctx.snapshot(&window, "scrolled down")?;

    ctx.connection().mouse_scroll_right()?;
    ctx.snapshot(&window, "scrolled down")?;

    ctx.connection().mouse_scroll_left()?;
    ctx.snapshot(&window, "scrolled up")?;

    ctx.connection().mouse_scroll_left()?;
    ctx.snapshot(&window, "scrolled up")?;

    window.close()?;
    Ok(())
}

#[widgem_test_kit::test]
pub fn pager(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().add_child::<RootWidget>(());
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    ctx.snapshot(&window, "scroll bar")?;

    window.mouse_move(100, 20)?;
    ctx.snapshot(&window, "highlighted pager")?;
    ctx.connection().mouse_click(1)?;
    ctx.snapshot(&window, "page right")?;
    window.mouse_move(0, 0)?;
    ctx.snapshot(&window, "no highlight")?;

    window.mouse_move(43, 20)?;
    ctx.snapshot(&window, "highlighted pager")?;
    ctx.connection().mouse_click(1)?;
    ctx.snapshot(&window, "page left")?;
    window.mouse_move(0, 0)?;
    ctx.snapshot(&window, "no highlight")?;

    ctx.connection().key("1")?;
    ctx.snapshot(&window, "increase range")?;
    window.mouse_move(100, 20)?;
    ctx.snapshot(&window, "highlighted pager")?;

    ctx.connection().mouse_down(1)?;
    ctx.snapshot(&window, "page right")?;
    ctx.connection().mouse_up(1)?;
    ctx.set_changing_expected(false);
    ctx.snapshot(&window, "released pager - no auto repeat")?;
    ctx.set_changing_expected(true);

    ctx.connection().mouse_down(1)?;
    ctx.snapshot(&window, "page right")?;
    ctx.snapshot(&window, "page right - first auto repeat")?;
    ctx.snapshot(&window, "page right - second auto repeat")?;
    ctx.connection().mouse_up(1)?;
    ctx.set_changing_expected(false);
    ctx.snapshot(&window, "released pager - no more auto repeats")?;
    ctx.set_changing_expected(true);

    window.close()?;
    Ok(())
}

#[widgem_test_kit::test]
pub fn resize(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().add_child::<RootWidget>(());
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    ctx.snapshot(&window, "scroll bar")?;

    window.resize(1, 1)?;
    ctx.snapshot(&window, "min size")?;

    window.resize(200, 66)?;
    ctx.snapshot(&window, "resized")?;

    window.resize(300, 66)?;
    ctx.snapshot(&window, "resized")?;

    window.resize(300, 200)?;
    ctx.snapshot(&window, "no change - fixed y size")?;

    window.resize(300, 5)?;
    ctx.snapshot(&window, "min y size")?;

    ctx.connection().key("r")?;
    ctx.snapshot(&window, "changed to vertical scroll bar")?;

    window.resize(1, 1)?;
    ctx.snapshot(&window, "min size")?;

    window.resize(200, 200)?;
    ctx.snapshot(&window, "resized")?;

    window.resize(200, 300)?;
    ctx.snapshot(&window, "resized")?;

    window.resize(1, 300)?;
    ctx.snapshot(&window, "min x size")?;

    window.close()?;
    Ok(())
}

#[widgem_test_kit::test]
pub fn right_arrow(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().add_child::<RootWidget>(());
        Ok(())
    })?;
    ctx.connection().mouse_move_global(0, 0)?;
    let window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    ctx.snapshot(&window, "scroll bar")?;
    window.mouse_move(142, 22)?;
    ctx.snapshot(&window, "highlighted right arrow")?;
    ctx.connection().mouse_down(1)?;
    ctx.snapshot(&window, "pressed right arrow - step right by 5")?;
    ctx.connection().mouse_up(1)?;
    ctx.snapshot(&window, "released right arrow - no auto repeat")?;

    ctx.connection().mouse_down(1)?;
    ctx.snapshot(&window, "pressed right arrow - step right by 5")?;
    ctx.snapshot(&window, "first auto repeat")?;
    ctx.snapshot(&window, "second auto repeat")?;
    ctx.snapshot(&window, "third auto repeat")?;
    ctx.connection().mouse_up(1)?;
    ctx.snapshot(&window, "released right arrow - no more auto repeats")?;

    window.close()?;
    Ok(())
}

#[widgem_test_kit::test]
pub fn slider_extremes(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().add_child::<RootWidget>(());
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    ctx.snapshot(&window, "scroll bar")?;

    window.mouse_move(60, 20)?;
    ctx.snapshot(&window, "highlighted slider")?;
    ctx.connection().mouse_down(1)?;
    ctx.snapshot(&window, "grabbed slider")?;
    window.mouse_move(300, 24)?;
    ctx.snapshot(&window, "dragged all the way right")?;
    ctx.connection().mouse_up(1)?;
    ctx.snapshot(&window, "released slider - no highlight")?;

    window.mouse_move(90, 24)?;
    ctx.connection().mouse_down(1)?;
    ctx.snapshot(&window, "grabbed slider")?;
    window.mouse_move(0, 20)?;
    ctx.snapshot(&window, "dragged all the way left")?;
    window.mouse_move(20, 20)?;
    ctx.set_changing_expected(false);
    ctx.snapshot(&window, "still all the way left")?;
    ctx.set_changing_expected(true);
    window.mouse_move(58, 20)?;
    ctx.snapshot(&window, "no longer all the way left")?;
    ctx.connection().mouse_up(1)?;
    ctx.snapshot(&window, "released slider")?;

    window.close()?;
    Ok(())
}

#[widgem_test_kit::test]
pub fn slider(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        r.base_mut().add_child::<RootWidget>(());
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    ctx.snapshot(&window, "scroll bar")?;

    window.mouse_move(40, 20)?;
    ctx.snapshot(&window, "highlighted slider")?;
    ctx.connection().mouse_down(1)?;
    ctx.snapshot(&window, "grabbed slider")?;
    window.mouse_move(50, 20)?;
    ctx.snapshot(&window, "moved slider by 10 px")?;
    ctx.connection().mouse_up(1)?;
    ctx.snapshot(&window, "released slider")?;
    window.mouse_move(15, 15)?;
    ctx.snapshot(&window, "highlighted left arrow")?;
    ctx.connection().mouse_click(1)?;
    ctx.snapshot(&window, "step left by 5")?;

    window.mouse_move(60, 18)?;
    ctx.snapshot(&window, "highlighted slider")?;
    ctx.connection().mouse_down(1)?;
    ctx.snapshot(&window, "grabbed slider")?;
    window.mouse_move(60, 88)?;
    ctx.snapshot(&window, "dragged down and outside - no effect")?;
    window.mouse_move(50, 88)?;
    ctx.snapshot(&window, "dragged left")?;
    ctx.connection().mouse_up(1)?;
    ctx.snapshot(&window, "released slider - no highlight")?;

    window.close()?;
    Ok(())
}
