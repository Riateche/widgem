use salvation::{
    widgets::{
        column::Column, label::Label, padding_box::PaddingBox, scroll_bar::ScrollBar, WidgetExt,
        WidgetId,
    },
    window::create_window,
    CallbackContext, WindowAttributes,
};

use crate::context::Context;

pub struct State {
    label_id: WidgetId<Label>,
    #[allow(dead_code)]
    scroll_bar_id: WidgetId<ScrollBar>,
}

impl State {
    pub fn new(ctx: &mut CallbackContext<Self>) -> Self {
        let value = 0;
        let label = Label::new(value.to_string()).with_id();
        let scroll_bar = ScrollBar::new()
            .with_on_value_changed(ctx.callback(State::on_scroll_bar_value_changed))
            .with_value(value)
            .with_id();
        let mut column = Column::new();
        column.add_child(scroll_bar.widget.boxed());
        column.add_child(label.widget.boxed());
        create_window(
            WindowAttributes::default().with_title(module_path!()),
            PaddingBox::new(column.boxed()).boxed(),
        );
        State {
            label_id: label.id,
            scroll_bar_id: scroll_bar.id,
        }
    }

    fn on_scroll_bar_value_changed(
        &mut self,
        ctx: &mut CallbackContext<Self>,
        value: i32,
    ) -> anyhow::Result<()> {
        ctx.widget(self.label_id)?.set_text(value.to_string());
        Ok(())
    }
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

    //std::thread::sleep(std::time::Duration::from_secs(1));
    window.mouse_move(40, 20)?;
    ctx.connection.mouse_down(1)?;
    ctx.snapshot(&mut window, "grabbed slider")?;
    window.mouse_move(50, 20)?;
    ctx.snapshot(&mut window, "moved slider by 10 px")?;
    ctx.connection.mouse_up(1)?;
    ctx.snapshot(&mut window, "released slider")?;

    window.close()?;
    Ok(())
}
