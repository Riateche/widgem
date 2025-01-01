use {
    crate::context::Context,
    salvation::{
        impl_widget_common,
        shortcut::{KeyCombinations, Shortcut, ShortcutScope},
        types::Axis,
        widgets::{
            column::Column, label::Label, scroll_bar::ScrollBar, Widget, WidgetCommon, WidgetExt,
            WidgetId,
        },
        WindowAttributes,
    },
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
            common: common.into(),
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

    window.close()?;
    Ok(())
}
