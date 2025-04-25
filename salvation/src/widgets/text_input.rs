use {
    super::{Widget, WidgetCommon, WidgetCommonTyped, WidgetExt},
    crate::{
        draw::DrawEvent,
        event::{
            FocusInEvent, FocusOutEvent, ImeEvent, KeyboardInputEvent, LayoutEvent,
            ScrollToRectEvent, StyleChangeEvent,
        },
        impl_widget_common,
        layout::{
            grid::{self, GridAxisOptions, GridOptions},
            Alignment, SizeHintMode,
        },
        style::text_input::{ComputedVariantStyle, TextInputState},
        system::ReportError,
        text_editor::Text,
        types::{Point, Rect},
    },
    anyhow::Result,
    cosmic_text::Attrs,
    log::warn,
    std::{cmp::max, fmt::Display},
    winit::window::CursorIcon,
};

struct Viewport {
    common: WidgetCommon,
}

impl Widget for Viewport {
    impl_widget_common!();

    fn new(common: WidgetCommonTyped<Self>) -> Self {
        Self {
            common: common.into(),
        }
    }

    fn recalculate_size_x_fixed(&mut self) -> bool {
        false
    }

    fn recalculate_size_y_fixed(&mut self) -> bool {
        true
    }

    fn recalculate_size_hint_x(&mut self, _mode: SizeHintMode) -> Result<i32> {
        Ok(0)
    }

    fn recalculate_size_hint_y(&mut self, _size_x: i32, _mode: SizeHintMode) -> Result<i32> {
        Ok(self
            .common
            .children
            .get(&0)
            .unwrap()
            .widget
            .downcast_ref::<Text>()
            .unwrap()
            .line_height() as i32)
    }
}

pub struct TextInput {
    common: WidgetCommon,
}

impl TextInput {
    fn text_widget(&self) -> &Text {
        self.common
            .children
            .get(&0)
            .unwrap()
            .widget
            .common()
            .children
            .get(&0)
            .unwrap()
            .widget
            .downcast_ref::<Text>()
            .expect("invalid child widget type")
    }

    fn text_widget_mut(&mut self) -> &mut Text {
        self.common
            .children
            .get_mut(&0)
            .unwrap()
            .widget
            .common_mut()
            .children
            .get_mut(&0)
            .unwrap()
            .widget
            .downcast_mut::<Text>()
            .expect("invalid child widget type")
    }

    pub fn set_text(&mut self, text: impl Display) {
        self.text_widget_mut().set_text(text, Attrs::new());
    }

    fn adjust_scroll(&mut self) {
        let Some(editor_viewport_rect) = self.common.children.get_mut(&0).unwrap().rect_in_parent
        else {
            return;
        };
        let text_size = self.text_widget().size();
        let cursor_position = self.text_widget().cursor_position();
        let mut scroll_x = self
            .common
            .children
            .get(&0)
            .unwrap()
            .widget
            .common()
            .children
            .get(&0)
            .unwrap()
            .rect_in_parent
            .map_or(0, |rect| -rect.left());
        let max_scroll = max(0, text_size.x - editor_viewport_rect.size.x);
        if let Some(cursor_position) = cursor_position {
            let cursor_x_in_viewport = cursor_position.x - scroll_x;
            if cursor_x_in_viewport < 0 {
                scroll_x -= -cursor_x_in_viewport;
            } else if cursor_x_in_viewport > editor_viewport_rect.size.x - 1 {
                scroll_x += cursor_x_in_viewport - (editor_viewport_rect.size.x - 1);
            }
        }
        scroll_x = scroll_x.clamp(0, max_scroll);
        let new_rect = Rect::from_pos_size(Point::new(-scroll_x, 0), text_size);
        if self
            .common
            .children
            .get(&0)
            .unwrap()
            .widget
            .common()
            .children
            .get(&0)
            .unwrap()
            .rect_in_parent
            != Some(new_rect)
        {
            self.common
                .children
                .get_mut(&0)
                .unwrap()
                .widget
                .common_mut()
                .set_child_rect(0, Some(new_rect))
                .or_report_err();
        }
    }

    fn current_variant_style(&self) -> &ComputedVariantStyle {
        let state = if self.common.is_enabled() {
            TextInputState::Enabled {
                focused: self.common.is_focused(),
                mouse_over: self.common.is_mouse_over,
            }
        } else {
            TextInputState::Disabled
        };
        self.common
            .style()
            .0
            .text_input
            .variants
            .get(&state)
            .unwrap()
    }

    fn refresh_style(&mut self) {
        let style = self.common.style().0.text_input.clone();
        let variant_style = self.current_variant_style().clone();
        self.common.set_grid_options(Some(GridOptions {
            x: GridAxisOptions {
                min_padding: style.min_padding_with_border.x,
                min_spacing: 0,
                preferred_padding: style.preferred_padding_with_border.x,
                preferred_spacing: 0,
                border_collapse: 0,
                alignment: Alignment::Start,
            },
            y: GridAxisOptions {
                min_padding: style.min_padding_with_border.y,
                min_spacing: 0,
                preferred_padding: style.preferred_padding_with_border.y,
                preferred_spacing: 0,
                border_collapse: 0,
                alignment: Alignment::Start,
            },
        }));
        let text_widget = self.text_widget_mut();
        text_widget.set_font_metrics(style.font_metrics);
        // TODO: support color changes based on state
        text_widget.set_text_color(variant_style.text_color);
        text_widget.set_selected_text_color(variant_style.selected_text_color);
        text_widget.set_selected_text_background(variant_style.selected_text_background);
    }
}

impl Widget for TextInput {
    impl_widget_common!();

    fn new(mut common: WidgetCommonTyped<Self>) -> Self {
        common.set_focusable(true);
        common.cursor_icon = CursorIcon::Text;
        let host_id = common.id;
        let viewport = common.add_child::<Viewport>(0).set_column(0).set_row(0);
        viewport.common_mut().receives_all_mouse_events = true;
        viewport.common_mut().cursor_icon = CursorIcon::Text;
        let editor = viewport
            .common_mut()
            .child::<Text>(0)
            .set_multiline(false)
            .set_editable(true)
            .set_host_id(host_id);
        editor.common_mut().receives_all_mouse_events = true;
        let mut t = Self {
            common: common.into(),
        };
        t.refresh_style();
        t
    }

    fn handle_focus_in(&mut self, event: FocusInEvent) -> Result<()> {
        self.text_widget_mut().handle_host_focus_in(event.reason)
    }

    fn handle_focus_out(&mut self, _event: FocusOutEvent) -> Result<()> {
        self.text_widget_mut().handle_host_focus_out()
    }

    fn handle_layout(&mut self, _event: LayoutEvent) -> Result<()> {
        let options = self.common().grid_options();
        let Some(size) = self.common().size() else {
            return Ok(());
        };
        let rects = grid::layout(&mut self.common_mut().children, &options, size)?;
        self.common_mut().set_child_rects(&rects)?;
        self.adjust_scroll();
        Ok(())
    }

    fn handle_style_change(&mut self, _event: StyleChangeEvent) -> Result<()> {
        self.refresh_style();

        Ok(())
    }

    fn handle_draw(&mut self, event: DrawEvent) -> Result<()> {
        let rect_in_window = self.common.rect_in_window_or_err()?;
        let style = self.current_variant_style();

        // TODO: stroke and fill instead
        event.stroke_rounded_rect(
            Rect {
                top_left: Point::default(),
                size: rect_in_window.size,
            },
            style.border.radius.get() as f32,
            style.border.color,
            style.border.width.get() as f32,
        );

        Ok(())
    }

    fn recalculate_size_hint_x(&mut self, mode: SizeHintMode) -> Result<i32> {
        let style = &self.common.style().0.text_input;
        let r = match mode {
            SizeHintMode::Min => style.min_width,
            SizeHintMode::Preferred => style.preferred_width,
        };
        Ok(r.get())
    }

    fn recalculate_size_hint_y(&mut self, _size_x: i32, mode: SizeHintMode) -> Result<i32> {
        let text_size = self.text_widget().size();
        let style = &self.common.style().0.text_input;
        let padding = match mode {
            SizeHintMode::Min => style.min_padding_with_border,
            SizeHintMode::Preferred => style.preferred_padding_with_border,
        };
        Ok(text_size.y + 2 * padding.y)
    }

    fn recalculate_size_x_fixed(&mut self) -> bool {
        false
    }

    fn recalculate_size_y_fixed(&mut self) -> bool {
        true
    }

    fn handle_keyboard_input(&mut self, event: KeyboardInputEvent) -> Result<bool> {
        self.text_widget_mut().handle_host_keyboard_input(event)
    }

    fn handle_ime(&mut self, event: ImeEvent) -> Result<bool> {
        self.text_widget_mut().handle_host_ime(event)
    }

    fn handle_scroll_to_rect(&mut self, event: ScrollToRectEvent) -> Result<bool> {
        if self.text_widget().common().id != event.address.widget_id() {
            warn!("TextInput received unexpected ScrollToRectEvent");
            return Ok(false);
        }

        self.adjust_scroll();

        Ok(true)
    }
}
