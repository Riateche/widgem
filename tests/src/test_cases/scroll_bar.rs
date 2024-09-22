use std::{thread::sleep, time::Duration};

use salvation::{
    impl_widget_common,
    shortcut::{KeyCombinations, Shortcut, ShortcutScope},
    types::Axis,
    widgets::{
        column::Column, label::Label, scroll_bar::ScrollBar, Widget, WidgetCommon, WidgetExt,
        WidgetId,
    },
    WindowAttributes,
};

use crate::context::Context;

pub struct RootWidget {
    common: WidgetCommon,
    label_id: WidgetId<Label>,
    scroll_bar_id: WidgetId<ScrollBar>,
}

impl RootWidget {
    pub fn new() -> Self {
        let mut common = WidgetCommon::new();

        let value = 0;
        let label = Label::new(value.to_string()).with_id();
        let scroll_bar = ScrollBar::new()
            .with_on_value_changed(common.id.callback(Self::on_scroll_bar_value_changed))
            .with_value(value)
            .with_id();
        let mut column = Column::new();
        column.add_child(scroll_bar.widget.boxed());
        column.add_child(label.widget.boxed());

        common.add_child(
            column
                .with_window(WindowAttributes::default().with_title(module_path!()))
                .boxed(),
            Default::default(),
        );

        let mut this = Self {
            common,
            label_id: label.id,
            scroll_bar_id: scroll_bar.id,
        };

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

pub fn check(ctx: &mut Context) -> anyhow::Result<()> {
    let mut window = ctx.wait_for_window_by_pid()?;
    // Workaround for winit issue:
    // https://github.com/rust-windowing/winit/issues/2841
    window.minimize()?;
    window.activate()?;
    window.mouse_move(0, 0)?;
    ctx.snapshot(&mut window, "scroll bar and label")?;
    window.resize(160, 66)?;
    ctx.snapshot(&mut window, "resized")?;

    window.mouse_move(40, 20)?;
    ctx.snapshot(&mut window, "highlighted slider")?;
    ctx.connection.mouse_down(1)?;
    ctx.snapshot(&mut window, "grabbed slider")?;
    window.mouse_move(50, 20)?;
    ctx.snapshot(&mut window, "moved slider by 10 px")?;
    ctx.connection.mouse_up(1)?;
    ctx.snapshot(&mut window, "released slider")?;
    window.mouse_move(15, 15)?;
    ctx.snapshot(&mut window, "highlighted left arrow")?;
    ctx.connection.mouse_click(1)?;
    ctx.snapshot(&mut window, "step left by 5")?;
    window.mouse_move(140, 20)?;
    ctx.snapshot(&mut window, "highlighted right arrow")?;

    ctx.connection.mouse_down(1)?;
    ctx.snapshot(&mut window, "pressed right arrow - step right by 5")?;
    sleep(Duration::from_millis(700)); // auto repeat delay is 2 s; snapshot delay is 0.5 s
    ctx.connection.mouse_up(1)?;
    ctx.snapshot(&mut window, "released right arrow - no auto repeat")?;

    ctx.connection.mouse_down(1)?;
    ctx.snapshot(&mut window, "pressed right arrow - step right by 5")?;
    sleep(Duration::from_millis(1300));
    ctx.snapshot(&mut window, "first auto repeat")?;
    sleep(Duration::from_millis(500));
    ctx.snapshot(&mut window, "second auto repeat")?;
    sleep(Duration::from_millis(500));
    ctx.snapshot(&mut window, "third auto repeat")?;
    ctx.connection.mouse_up(1)?;
    ctx.snapshot(&mut window, "released right arrow - no more auto repeats")?;

    window.mouse_move(103, 18)?;
    ctx.snapshot(&mut window, "highlighted slider")?;
    ctx.connection.mouse_down(1)?;
    ctx.snapshot(&mut window, "grabbed slider")?;
    window.mouse_move(103, 88)?;
    ctx.snapshot(&mut window, "dragged down and outside - no effect")?;
    window.mouse_move(90, 88)?;
    ctx.snapshot(&mut window, "dragged left")?;
    ctx.connection.mouse_up(1)?;
    ctx.snapshot(&mut window, "released slider - no highlight")?;

    window.mouse_move(60, 20)?;
    ctx.snapshot(&mut window, "highlighted slider")?;
    ctx.connection.mouse_down(1)?;
    ctx.snapshot(&mut window, "grabbed slider")?;
    window.mouse_move(0, 20)?;
    ctx.snapshot(&mut window, "dragged all the way left")?;
    window.mouse_move(20, 20)?;
    ctx.snapshot(&mut window, "still all the way left")?;
    window.mouse_move(58, 20)?;
    ctx.snapshot(&mut window, "no longer all the way left")?;
    ctx.connection.mouse_up(1)?;
    ctx.snapshot(&mut window, "released slider")?;

    window.mouse_move(90, 24)?;
    ctx.connection.mouse_down(1)?;
    ctx.snapshot(&mut window, "grabbed slider")?;
    window.mouse_move(300, 24)?;
    ctx.snapshot(&mut window, "dragged all the way right")?;
    ctx.connection.mouse_up(1)?;
    ctx.snapshot(&mut window, "released slider - no highlight")?;

    window.mouse_move(43, 20)?;
    ctx.snapshot(&mut window, "highlighted pager")?;
    ctx.connection.mouse_click(1)?;
    ctx.snapshot(&mut window, "page left")?;
    window.mouse_move(0, 0)?;
    ctx.snapshot(&mut window, "no highlight")?;
    window.mouse_move(100, 20)?;
    ctx.snapshot(&mut window, "highlighted pager")?;
    ctx.connection.mouse_click(1)?;
    ctx.snapshot(&mut window, "page right")?;

    window.close()?;
    Ok(())
}
