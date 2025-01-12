use {
    super::{viewport::Viewport, Widget, WidgetCommon, WidgetExt},
    crate::{
        draw::DrawEvent,
        event::{LayoutEvent, WidgetScopeChangeEvent},
        impl_widget_common,
        layout::{
            grid::{self, GridAxisOptions, GridOptions},
            LayoutItemOptions, SizeHintMode,
        },
        style::text_input::{ComputedVariantStyle, TextInputState},
        system::ReportError,
        text_editor::Text,
        types::{Point, Rect},
    },
    anyhow::Result,
    cosmic_text::Attrs,
    std::{cmp::max, fmt::Display},
};

pub struct TextInput {
    common: WidgetCommon,
}

impl TextInput {
    pub fn new(text: impl Display) -> Self {
        let mut common = WidgetCommon::new::<Self>();
        let editor = Text::new(text).with_multiline(false).with_editable(true);
        let mut viewport = Viewport::new();
        viewport
            .common_mut()
            .add_child(editor.boxed(), Default::default());
        common.add_child(viewport.boxed(), LayoutItemOptions::from_pos_in_grid(0, 0));
        Self {
            common: common.into(),
        }
    }

    fn text_widget(&self) -> &Text {
        self.common.children[0].widget.common().children[0]
            .widget
            .downcast_ref::<Text>()
            .expect("invalid child widget type")
    }

    fn text_widget_mut(&mut self) -> &mut Text {
        self.common.children[0].widget.common_mut().children[0]
            .widget
            .downcast_mut::<Text>()
            .expect("invalid child widget type")
    }

    pub fn set_text(&mut self, text: impl Display) {
        self.text_widget_mut().set_text(text, Attrs::new());
        self.adjust_scroll();
    }

    fn adjust_scroll(&mut self) {
        let Some(editor_viewport_rect) = self.common.children[0].rect_in_parent else {
            return;
        };
        let text_size = self.text_widget().size();
        let cursor_position = self.text_widget().cursor_position();
        let mut scroll_x = self.common.children[0].widget.common().children[0]
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
        if self.common.children[0].widget.common().children[0].rect_in_parent != Some(new_rect) {
            self.common.children[0]
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
}

impl Widget for TextInput {
    impl_widget_common!();

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

    fn handle_widget_scope_change(&mut self, _event: WidgetScopeChangeEvent) -> Result<()> {
        let style = self.common.style().0.text_input.clone();
        let variant_style = self.current_variant_style().clone();
        self.common.set_grid_options(Some(GridOptions {
            x: GridAxisOptions {
                min_padding: style.min_padding_with_border.x,
                min_spacing: 0,
                preferred_padding: style.preferred_padding_with_border.x,
                preferred_spacing: 0,
                border_collapse: 0,
            },
            y: GridAxisOptions {
                min_padding: style.min_padding_with_border.y,
                min_spacing: 0,
                preferred_padding: style.preferred_padding_with_border.y,
                preferred_spacing: 0,
                border_collapse: 0,
            },
        }));
        let text_widget = self.text_widget_mut();
        text_widget.set_font_metrics(style.font_metrics);
        // TODO: support color changes based on state
        text_widget.set_text_color(variant_style.text_color);
        text_widget.set_selected_text_color(variant_style.selected_text_color);
        text_widget.set_selected_text_background(variant_style.selected_text_background);

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
}
