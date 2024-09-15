use std::{thread::sleep, time::Duration};

use salvation::{
    impl_widget_common,
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
        Self {
            common,
            label_id: label.id,
        }
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
    sleep(Duration::from_millis(1200)); // auto repeat delay is 2 s
    ctx.connection.mouse_up(1)?;
    ctx.snapshot(&mut window, "released right arrow - no auto repeat")?;

    ctx.connection.mouse_down(1)?;
    ctx.snapshot(&mut window, "pressed right arrow - step right by 5")?;
    sleep(Duration::from_millis(2300));
    ctx.snapshot(&mut window, "first auto repeat")?;
    sleep(Duration::from_millis(1000));
    ctx.snapshot(&mut window, "second auto repeat")?;
    sleep(Duration::from_millis(1000));
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

    window.close()?;
    Ok(())
}
