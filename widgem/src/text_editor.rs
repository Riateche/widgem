use {
    crate::{
        accessibility::new_accessibility_node_id,
        draw::DrawEvent,
        event::{
            AccessibilityActionEvent, FocusReason, InputMethodEvent, KeyboardInputEvent,
            MouseInputEvent, MouseMoveEvent, WindowFocusChangeEvent,
        },
        impl_widget_base,
        layout::SizeHint,
        shortcut::standard_shortcuts,
        style::{
            common::ComputedElementStyle,
            css::{
                convert_background_color, convert_font, convert_main_color, is_selection,
                StyleSelector,
            },
            defaults, Styles,
        },
        system::OrWarn,
        text::{
            action::Action,
            edit::Edit,
            editor::{Editor, EditorDrawStyle},
            text_without_preedit, Metadata,
        },
        timer::TimerId,
        types::{PhysicalPixels, Point, PpxSuffix, Rect, Size},
        widget_initializer::{self, WidgetInitializer},
        RawWidgetId, Widget, WidgetBaseOf, WidgetExt,
    },
    accesskit::{ActionData, NodeId, Role, TextDirection, TextPosition, TextSelection},
    anyhow::Result,
    cosmic_text::{
        Affinity, Attrs, AttrsList, AttrsOwned, BorrowedWithFontSystem, Buffer, Cursor, Motion,
        Shaping, Wrap,
    },
    line_straddler::{GlyphStyle, LineGenerator, LineType},
    range_ext::intersect::Intersect,
    std::{
        cmp::{max, min},
        fmt::Display,
        ops::Range,
        rc::Rc,
        time::Duration,
    },
    strict_num::FiniteF32,
    tiny_skia::{Color, Paint, PathBuilder, Pixmap, Shader, Stroke, Transform},
    tracing::warn,
    unicode_segmentation::UnicodeSegmentation,
    widgem_macros::impl_with,
    winit::{
        event::{ElementState, Ime, MouseButton},
        keyboard::{Key, NamedKey},
        window::CursorIcon,
    },
};

const CURSOR_SIZE_X: i32 = 1; // TODO: configurable, scalable

#[derive(Debug, PartialEq, Clone)]
pub struct TextStyle {
    font_metrics: cosmic_text::Metrics,
    pub(crate) text_color: Color,
    selected_text_color: Color,
    selected_text_background: Color,
}

impl ComputedElementStyle for TextStyle {
    fn new(style: &Styles, element: &StyleSelector, scale: f32) -> Self {
        let rules = style.find_rules_for_element(element);

        // TODO: different selection styles depending on `element`
        let selection_properties = style.find_rules(is_selection);
        let selected_text_color = convert_main_color(&selection_properties).unwrap_or_else(|| {
            warn!("selected text color is unspecified");
            defaults::selected_text_color()
        });
        let selected_text_background = convert_background_color(&selection_properties)
            .unwrap_or_else(|| {
                warn!("selected text background is unspecified");
                defaults::selected_text_background()
            });
        Self {
            font_metrics: convert_font(&rules, Some(&style.root_font_style())).to_metrics(scale),
            text_color: convert_main_color(&rules).unwrap_or_else(|| style.root_color()),
            selected_text_color,
            selected_text_background,
        }
    }
}

pub struct Text {
    base: WidgetBaseOf<Self>,
    style: Rc<TextStyle>,
    editor: Editor<'static>,
    pixmap: Option<Pixmap>,
    size: Size,
    is_multiline: bool,
    is_editable: bool,
    is_cursor_hidden: bool,
    is_host_focused: bool,
    host_id: Option<RawWidgetId>,
    forbid_mouse_interaction: bool,
    blink_timer: Option<TimerId>,
    selected_text: String,
    line_accessibility_node_id: NodeId,
}

// TODO: get system setting
const CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug)]
pub struct AccessibilityLine {
    pub text: String,
    pub text_direction: TextDirection,
    pub character_lengths: Vec<u8>,
    pub character_positions: Vec<f32>,
    pub character_widths: Vec<f32>,
    pub word_lengths: Vec<u8>,
    // pub line_top: f32,
    // pub line_bottom: f32,
}

#[impl_with]
impl Text {
    fn new(base: WidgetBaseOf<Self>, text: String, style: Rc<TextStyle>) -> Self {
        let editor = base.app().with_font_system(|font_system| {
            Editor::new(Buffer::new(font_system, style.font_metrics))
        });
        let mut t = Text {
            editor,
            pixmap: None,
            style,
            size: Size::default(),
            is_multiline: true,
            is_editable: false,
            is_cursor_hidden: true,
            is_host_focused: false,
            host_id: None,
            forbid_mouse_interaction: false,
            blink_timer: None,
            selected_text: String::new(),
            line_accessibility_node_id: new_accessibility_node_id(),
            base,
        };
        if let Some(window) = t.base.window() {
            window.remove_accessibility_node(
                Some(t.base.id().into()),
                t.line_accessibility_node_id,
                0.into(),
            );
        }
        t.editor.set_cursor_hidden(true);
        t.set_text(text);
        t.adjust_size();
        t.reset_blink_timer();
        t.request_scroll();
        t
    }

    pub fn init(text: String, style: Rc<TextStyle>) -> impl WidgetInitializer<Output = Self> {
        widget_initializer::from_fallible_new_and_fallible_2_set(
            |base, text, style| Ok(Self::new(base, text, style)),
            |w, text| Ok(w.set_text(text)),
            |w, style| Ok(w.set_text_style(style)),
            text,
            style,
        )
    }

    pub fn set_editable(&mut self, editable: bool) -> &mut Self {
        self.is_editable = editable;
        self.base.set_input_method_enabled(editable);
        self.base.set_cursor_icon(if editable {
            CursorIcon::Text
        } else {
            CursorIcon::Default
        });
        if !editable {
            self.set_cursor_hidden(true);
        }
        self.adjust_size();
        self.reset_blink_timer();
        self.request_scroll();
        self
    }

    pub fn set_multiline(&mut self, multiline: bool) -> &mut Self {
        self.is_multiline = multiline;
        if !multiline {
            self.set_wrap(Wrap::None);
        }
        let text = self.text();
        let sanitized = self.sanitize(&text);
        if text != sanitized {
            self.set_text(sanitized);
        }
        self.request_scroll();
        self
    }

    pub fn set_host_id(&mut self, id: RawWidgetId) -> &mut Self {
        self.host_id = Some(id);
        self
    }

    fn sanitize(&self, text: &str) -> String {
        if self.is_multiline {
            text.into()
        } else {
            text.replace('\n', " ")
        }
    }

    pub fn set_font_metrics(&mut self, metrics: cosmic_text::Metrics) {
        if self
            .editor
            .with_buffer(|buffer| buffer.metrics() == metrics)
        {
            return;
        }
        self.base.app().with_font_system(|font_system| {
            self.editor.with_buffer_mut(|buffer| {
                let changed = buffer.metrics() != metrics;
                if changed {
                    buffer.set_metrics(font_system, metrics);
                }
                changed
            });
        });
        self.adjust_size();
        self.request_scroll();
    }

    pub fn set_wrap(&mut self, wrap: Wrap) {
        if self.editor.with_buffer(|buffer| buffer.wrap() == wrap) {
            return;
        }

        self.base.app().with_font_system(|font_system| {
            self.editor
                .with_buffer_mut(|buffer| buffer.set_wrap(font_system, wrap));
        });
        self.adjust_size();
        self.request_scroll();
    }

    pub fn handle_host_focus_in(&mut self, reason: FocusReason) -> Result<()> {
        self.is_host_focused = true;
        if reason == FocusReason::Tab {
            self.action(Action::SelectAll);
        }
        self.reset_blink_timer();
        self.request_scroll();
        Ok(())
    }

    pub fn handle_host_focus_out(&mut self) -> Result<()> {
        self.is_host_focused = false;
        self.interrupt_preedit();
        self.action(Action::ClearSelection);
        self.reset_blink_timer();
        Ok(())
    }

    fn request_scroll(&mut self) {
        if !self.is_host_focused {
            return;
        }
        let Some(cursor_position) = self.cursor_position() else {
            return;
        };
        let Some(visible_rect) = self.base.visible_rect_in_self() else {
            return;
        };
        // TODO: cursor width?
        let rect = Rect::from_pos_size(
            cursor_position,
            Size::new(
                PhysicalPixels::from_i32(1),
                PhysicalPixels::from_i32(self.line_height().ceil() as i32),
            ),
        );
        if visible_rect.intersect(rect).is_empty() {
            self.base.ensure_rect_visible(rect);
        }
    }

    #[allow(clippy::if_same_then_else)]
    pub fn handle_host_keyboard_input(&mut self, event: KeyboardInputEvent) -> Result<bool> {
        if !self.is_editable {
            return Ok(false);
        }
        if event.info.state == ElementState::Released {
            return Ok(true);
        }

        let shortcuts = standard_shortcuts();
        if shortcuts.move_to_next_char.matches(&event) {
            self.action(Action::Motion {
                motion: Motion::Next.into(),
                select: false,
            });
        } else if shortcuts.move_to_previous_char.matches(&event) {
            self.action(Action::Motion {
                motion: Motion::Previous.into(),
                select: false,
            });
        } else if shortcuts.delete.matches(&event) {
            self.action(Action::Delete);
        } else if shortcuts.backspace.matches(&event) {
            self.action(Action::Backspace);
        } else if shortcuts.cut.matches(&event) {
            self.copy_to_clipboard();
            self.action(Action::Delete);
        } else if shortcuts.copy.matches(&event) {
            self.copy_to_clipboard();
        } else if shortcuts.paste.matches(&event) {
            if let Some(text) = self.base.app().clipboard_text().or_warn() {
                let text = self.sanitize(&text);
                self.insert_string(&text, None);
            }
        } else if shortcuts.undo.matches(&event) {
            // TODO
        } else if shortcuts.redo.matches(&event) {
            // TODO
        } else if shortcuts.select_all.matches(&event) {
            self.action(Action::SelectAll);
        } else if shortcuts.deselect.matches(&event) {
            // TODO: why Escape?
            self.action(Action::ClearSelection);
        } else if shortcuts.move_to_next_word.matches(&event) {
            self.action(Action::Motion {
                motion: Motion::NextWord.into(),
                select: false,
            });
        } else if shortcuts.move_to_previous_word.matches(&event) {
            self.action(Action::Motion {
                motion: Motion::PreviousWord.into(),
                select: false,
            });
        } else if shortcuts.move_to_start_of_line.matches(&event) {
            self.action(Action::Motion {
                motion: Motion::Home.into(),
                select: false,
            });
        } else if shortcuts.move_to_end_of_line.matches(&event) {
            self.action(Action::Motion {
                motion: Motion::End.into(),
                select: false,
            });
        } else if shortcuts.select_next_char.matches(&event) {
            self.action(Action::Motion {
                motion: Motion::Next.into(),
                select: true,
            });
        } else if shortcuts.select_previous_char.matches(&event) {
            self.action(Action::Motion {
                motion: Motion::Previous.into(),
                select: true,
            });
        } else if shortcuts.select_next_word.matches(&event) {
            self.action(Action::Motion {
                motion: Motion::NextWord.into(),
                select: true,
            });
        } else if shortcuts.select_previous_word.matches(&event) {
            self.action(Action::Motion {
                motion: Motion::PreviousWord.into(),
                select: true,
            });
        } else if shortcuts.select_start_of_line.matches(&event) {
            self.action(Action::Motion {
                motion: Motion::Home.into(),
                select: true,
            });
        } else if shortcuts.select_end_of_line.matches(&event) {
            self.action(Action::Motion {
                motion: Motion::End.into(),
                select: true,
            });
        } else if shortcuts.delete_start_of_word.matches(&event) {
            self.action(Action::DeleteStartOfWord);
        } else if shortcuts.delete_end_of_word.matches(&event) {
            self.action(Action::DeleteEndOfWord);
        } else if let Some(text) = &event.info.text {
            if let Key::Named(key) = &event.info.logical_key {
                if [NamedKey::Tab, NamedKey::Enter, NamedKey::Escape].contains(key) {
                    return Ok(false);
                }
            }
            let text = self.sanitize(text);
            self.insert_string(&text, None);
        } else {
            return Ok(false);
        }
        // TODO: notify parent?
        //self.adjust_scroll();
        self.base.update();
        self.reset_blink_timer();
        self.request_scroll();
        Ok(true)
    }

    pub fn handle_host_ime(&mut self, event: InputMethodEvent) -> Result<bool> {
        if !self.is_editable {
            return Ok(false);
        }
        match event.info {
            Ime::Enabled => {}
            Ime::Preedit(preedit, cursor) => {
                // TODO: can pretext have line breaks?
                self.action(Action::SetPreedit {
                    preedit: self.sanitize(&preedit),
                    cursor,
                    attrs: None,
                });
            }
            Ime::Commit(string) => {
                self.editor.insert_string(&self.sanitize(&string), None);
            }
            Ime::Disabled => {}
        }
        // TODO: notify parent?
        //self.adjust_scroll();
        self.base.update();
        self.reset_blink_timer();
        self.request_scroll();
        Ok(true)
    }

    pub fn set_text_and_attrs(&mut self, text: impl Display, attrs: Attrs) -> &mut Self {
        let text = text.to_string();
        if self.text() == text {
            return self;
        }

        self.base.app().with_font_system(|font_system| {
            self.editor.with_buffer_mut(|buffer| {
                buffer.set_text(font_system, &text, &attrs, Shaping::Advanced)
            });
        });
        self.adjust_size();
        self.after_change();
        self.reset_blink_timer();
        self.base.update();
        self.request_scroll();
        self
    }

    pub fn set_text(&mut self, text: impl Display) -> &mut Self {
        self.set_text_and_attrs(text, Attrs::new())
    }

    pub fn text(&self) -> String {
        self.editor.with_buffer(text_without_preedit)
    }

    pub fn set_text_style(&mut self, style: Rc<TextStyle>) -> &mut Self {
        if self.style == style {
            return self;
        }
        let old_style = self.style.clone();
        self.style = style;
        self.set_font_metrics(self.style.font_metrics);
        if old_style.text_color != self.style.text_color
            || old_style.selected_text_color != self.style.selected_text_color
            || old_style.selected_text_background != self.style.selected_text_background
        {
            self.editor.with_buffer_mut(|b| b.set_redraw(true));
            self.base.update();
        }
        self
    }

    fn after_change(&mut self) {
        let new_selected_text = self.selected_text().unwrap_or_default();
        if new_selected_text != self.selected_text {
            self.selected_text = new_selected_text;
            #[cfg(all(
                unix,
                not(any(target_os = "macos", target_os = "android", target_os = "emscripten"))
            ))]
            self.base
                .app()
                .set_linux_primary_selection(&self.selected_text)
                .or_warn();
        }
        self.request_scroll();
    }

    #[cfg(all(
        unix,
        not(any(target_os = "macos", target_os = "android", target_os = "emscripten"))
    ))]
    fn paste_selection(&mut self) {
        if self.is_mouse_interaction_forbidden() {
            return;
        }
        let text = self.base.app().linux_primary_selection().or_warn();
        if let Some(text) = text {
            let text = self.sanitize(&text);
            self.insert_string(&text, None);
        }
    }

    fn copy_to_clipboard(&mut self) {
        if let Some(text) = self.selected_text() {
            self.base.app().set_clipboard_text(&text).or_warn();
        }
    }

    fn reset_blink_timer(&mut self) {
        if let Some(id) = self.blink_timer.take() {
            self.base.app().cancel_timer(id);
        }
        self.editor
            .set_cursor_hidden(!self.is_host_focused || !self.is_editable);
        if self.is_host_focused && self.is_editable {
            let id = self.base.app().add_interval(
                CURSOR_BLINK_INTERVAL,
                self.callback(|this, _| this.toggle_cursor_hidden()),
            );
            self.blink_timer = Some(id);
        }
        self.base.update();
    }

    fn toggle_cursor_hidden(&mut self) -> Result<()> {
        self.set_cursor_hidden(!self.is_cursor_hidden);
        if !self.editor.has_selection() {
            self.base.update();
        }
        Ok(())
    }

    fn accessibility_line(&mut self) -> AccessibilityLine {
        #[derive(Debug)]
        struct CharStats {
            bytes: Range<usize>,
            pixels: Option<Range<FiniteF32>>,
        }

        self.shape_as_needed();
        // TODO: extend for multiline
        // TODO: take ref
        let text = self
            .editor
            .with_buffer(|buffer| buffer.lines[0].text().to_owned());

        let mut character_lengths = Vec::new();
        let mut character_stats = Vec::new();
        for (i, c) in text.grapheme_indices(true) {
            character_lengths.push(c.len() as u8);
            character_stats.push(CharStats {
                bytes: i..i + c.len(),
                pixels: None,
            });
        }
        let mut word_lengths = Vec::new();
        // TODO: expose from cosmic-text
        let mut prev_index_in_chars = None;
        let mut total_chars_in_words = 0;
        for (i, word) in text.unicode_word_indices() {
            let end_i = i + word.len();
            let index_in_chars = character_stats
                .iter()
                .take_while(|s| s.bytes.start < end_i)
                .count();
            // TODO: checked_sub?
            let len_in_chars = index_in_chars - prev_index_in_chars.unwrap_or(0);
            word_lengths.push(len_in_chars as u8);
            prev_index_in_chars = Some(index_in_chars);
            total_chars_in_words += len_in_chars;
        }
        if total_chars_in_words < character_stats.len() {
            word_lengths.push((character_stats.len() - total_chars_in_words) as u8);
        }
        let text_direction = self.editor.with_buffer(|buffer| {
            let mut runs = buffer.layout_runs();
            let Some(run) = runs.next() else {
                // No runs is expected if the text is empty.
                return TextDirection::LeftToRight;
            };
            if runs.next().is_some() {
                // TODO: label can have multiple lines
                //warn!("multiple layout_runs in single line edit");
            }

            if run.line_i != 0 {
                warn!("invalid line_i in single line layout_runs");
            }
            for glyph in run.glyphs {
                if let Some(stats) = character_stats
                    .iter_mut()
                    .find(|s| s.bytes.does_intersect(&(glyph.start..glyph.end)))
                {
                    let new_start = FiniteF32::new(glyph.x).unwrap();
                    let new_end = FiniteF32::new(glyph.x + glyph.w).unwrap();
                    if let Some(pixels) = &mut stats.pixels {
                        pixels.start = min(pixels.start, new_start);
                        pixels.end = max(pixels.end, new_end);
                    } else {
                        stats.pixels = Some(new_start..new_end);
                    }
                } else {
                    warn!("no char found for glyph: {glyph:?}");
                }
            }
            if run.rtl {
                TextDirection::RightToLeft
            } else {
                TextDirection::LeftToRight
            }
        });

        AccessibilityLine {
            text_direction,
            // line_top: run.line_top,
            // line_bottom: run.line_top + self.editor.buffer().metrics().line_height,
            text,
            character_lengths,
            character_positions: character_stats
                .iter()
                .map(|s| {
                    s.pixels.as_ref().map_or_else(
                        || {
                            warn!("glyph for char not found");
                            0.0
                        },
                        |range| range.start.get(),
                    )
                })
                .collect(),
            character_widths: character_stats
                .iter()
                .map(|s| {
                    s.pixels.as_ref().map_or_else(
                        || {
                            warn!("glyph for char not found;");
                            0.0
                        },
                        |range| range.end.get() - range.start.get(),
                    )
                })
                .collect(),
            // TODO: real words
            word_lengths,
        }
    }

    pub fn handle_accessibility_set_selection_action(&mut self, data: TextSelection) {
        let text = self
            .editor
            .with_buffer(|buffer| buffer.lines[0].text().to_string());
        let char_to_byte_index =
            |char_index| text.grapheme_indices(true).nth(char_index).map(|(i, _)| i);
        if data.anchor == data.focus {
            self.set_select_opt(None);
        } else {
            let Some(index) = char_to_byte_index(data.anchor.character_index) else {
                warn!("char index is too large");
                return;
            };
            self.set_select_opt(Some(Cursor {
                line: 0,
                index,
                affinity: Affinity::Before,
            }));
        }
        let Some(index) = char_to_byte_index(data.focus.character_index) else {
            warn!("char index is too large");
            return;
        };
        self.set_cursor(Cursor {
            line: 0,
            index,
            affinity: Affinity::Before,
        });
    }

    fn selection_accessibility_info(&mut self, id: NodeId) -> TextSelection {
        let text = self
            .editor
            .with_buffer(|buffer| buffer.lines[0].text().to_string());
        let byte_to_char_index = |byte_index| {
            text.grapheme_indices(true)
                .take_while(|(i, _)| *i < byte_index)
                .count()
        };
        let focus = TextPosition {
            node: id,
            character_index: byte_to_char_index(self.cursor().index),
        };
        let anchor = if let Some(select) = self.select_opt() {
            TextPosition {
                node: id,
                character_index: byte_to_char_index(select.index),
            }
        } else {
            focus
        };
        TextSelection { anchor, focus }
    }

    pub fn insert_string(&mut self, text: &str, attrs_list: Option<AttrsList>) {
        if text.is_empty() {
            return;
        }
        self.editor.insert_string(text, attrs_list);
        self.adjust_size();
        self.base.update();
        self.request_scroll();
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn size_x(&self) -> PhysicalPixels {
        self.size.x()
    }

    pub fn size_y(&self) -> PhysicalPixels {
        self.size.y()
    }

    pub fn shape_as_needed(&mut self) {
        self.base
            .app()
            .with_font_system(|font_system| self.editor.shape_as_needed(font_system, false));
        self.request_scroll();
    }

    pub fn needs_redraw(&mut self) -> bool {
        self.shape_as_needed();
        self.editor.redraw()
    }

    pub fn is_mouse_interaction_forbidden(&self) -> bool {
        self.forbid_mouse_interaction
    }

    // TODO: private
    pub fn pixmap(&mut self) -> &Pixmap {
        if self.pixmap.is_none() || self.needs_redraw() {
            let (buffer_width, buffer_height) = self.editor.with_buffer(|buffer| buffer.size());
            let size_x = max(1, buffer_width.unwrap_or(0.).ceil() as u32);
            let size_y = max(1, buffer_height.unwrap_or(0.).ceil() as u32);

            let mut pixmap = Pixmap::new(size_x, size_y).expect("failed to create pixmap");
            self.base
                .app()
                .with_font_system_and_swash_cache(|font_system, swash_cache| {
                    self.editor.draw(
                        font_system,
                        swash_cache,
                        &EditorDrawStyle {
                            text_color: convert_color(self.style.text_color),
                            cursor_color: convert_color(self.style.text_color), // TODO: cursor color,
                            selection_color: convert_color(self.style.selected_text_background),
                            selected_text_color: convert_color(self.style.selected_text_color),
                        },
                        |x, y, w, h, c| {
                            // let color = PremultipliedColorU8::from_rgba(
                            //     min(c.a(), c.r()),
                            //     min(c.a(), c.g()),
                            //     min(c.a(), c.b()),
                            //     c.a(),
                            // )
                            // .expect("RGB components must be <= alpha");

                            // for iy in y..(y + h as i32) {
                            //     for ix in x..(x + w as i32) {
                            //         if ix >= 0 && ix < pixmap_width && iy >= 0 && iy < pixmap_height {
                            //             pixels[(ix + iy * pixmap_width) as usize] = color;
                            //         }
                            //     }
                            // }

                            let color = Color::from_rgba8(c.r(), c.g(), c.b(), c.a());
                            let paint = Paint {
                                shader: Shader::SolidColor(color),
                                anti_alias: false,
                                ..Paint::default()
                            };
                            pixmap.fill_rect(
                                tiny_skia::Rect::from_xywh(x as f32, y as f32, w as f32, h as f32)
                                    .unwrap(),
                                &paint,
                                Transform::default(),
                                None,
                            );
                        },
                    );
                });
            let mut alg = LineGenerator::new(LineType::Underline);
            let mut lines = Vec::new();
            let line_height = self
                .editor
                .with_buffer(|buffer| buffer.metrics().line_height);
            // TODO: determine from glyph width?
            let stroke_width = 1.0;
            self.editor.with_buffer(|buffer| {
                for run in buffer.layout_runs() {
                    let underline_space = line_height - run.line_y;
                    let line_y = run.line_top + underline_space / 2.0;
                    let line_y = (line_y + stroke_width / 2.0).round() - stroke_width / 2.0;
                    for glyph in run.glyphs {
                        if Metadata(glyph.metadata).is_preedit() {
                            let color = glyph
                                .color_opt
                                .unwrap_or(convert_color(self.style.text_color));
                            let glyph = line_straddler::Glyph {
                                line_y,
                                font_size: glyph.font_size,
                                width: glyph.w,
                                x: glyph.x,
                                style: GlyphStyle {
                                    boldness: 1,
                                    color: line_straddler::Color::rgba(
                                        color.r(),
                                        color.g(),
                                        color.b(),
                                        color.a(),
                                    ),
                                },
                            };
                            lines.extend(alg.add_glyph(glyph));
                        }
                    }
                }
            });
            lines.extend(alg.pop_line());
            for line in lines {
                let mut path = PathBuilder::new();
                path.move_to(line.start_x, line.y);
                path.line_to(line.end_x, line.y);
                pixmap.stroke_path(
                    &path.finish().unwrap(),
                    &Paint {
                        shader: Shader::SolidColor(tiny_skia::Color::from_rgba8(
                            line.style.color.red(),
                            line.style.color.green(),
                            line.style.color.blue(),
                            line.style.color.alpha(),
                        )),
                        ..Paint::default()
                    },
                    &Stroke {
                        width: stroke_width,
                        ..Stroke::default()
                    },
                    Transform::default(),
                    None,
                );
            }
            self.pixmap = Some(pixmap);
            self.editor.set_redraw(false);
        }
        self.pixmap.as_ref().expect("created above")
    }

    pub fn cursor_position(&self) -> Option<Point> {
        self.editor
            .cursor_position()
            .map(|(x, y)| Point::new(PhysicalPixels::from_i32(x), PhysicalPixels::from_i32(y)))
    }

    pub fn line_height(&self) -> f32 {
        self.editor
            .with_buffer(|buffer| buffer.metrics().line_height)
    }

    pub fn action(&mut self, mut action: Action) {
        if let Action::Drag { .. } = &mut action {
            if self.forbid_mouse_interaction {
                return;
            }
        }
        self.base
            .app()
            .with_font_system(|font_system| self.editor.action(font_system, action));
        self.adjust_size();
        self.base.update();
        self.request_scroll();
    }

    pub fn cursor(&self) -> Cursor {
        self.editor.cursor()
    }
    pub fn set_cursor(&mut self, cursor: Cursor) {
        self.editor.set_cursor(cursor);
        self.base.update();
        self.request_scroll();
    }

    pub fn has_selection(&self) -> bool {
        self.editor.has_selection()
    }

    // TODO: update API
    pub fn select_opt(&self) -> Option<Cursor> {
        if let cosmic_text::Selection::Normal(value) = self.editor.selection() {
            Some(value)
        } else {
            None
        }
    }

    pub fn set_select_opt(&mut self, select_opt: Option<Cursor>) {
        self.editor.set_selection(if let Some(cursor) = select_opt {
            cosmic_text::Selection::Normal(cursor)
        } else {
            cosmic_text::Selection::None
        });
        self.request_scroll();
    }

    fn interrupt_preedit(&mut self) {
        if let Some(text) = self.editor.preedit_text() {
            let text = text.to_owned();
            self.action(Action::SetPreedit {
                preedit: String::new(),
                cursor: None,
                attrs: None,
            });
            self.insert_string(&text, None);
            if let Some(window) = self.base.window() {
                window.cancel_ime_preedit();
            } else {
                warn!("no window id in text editor event handler");
            }
        }
    }

    pub fn attrs_at_cursor(&self) -> AttrsOwned {
        // TODO: use lines.get() everywhere to be safe
        self.editor.with_buffer(|buffer| {
            let line = &buffer.lines[self.editor.cursor().line];
            AttrsOwned::new(&line.attrs_list().get_span(self.editor.cursor().index))
        })
    }

    fn adjust_size(&mut self) {
        let size = self.base.app().with_font_system(|font_system| {
            self.editor.with_buffer_mut(|buffer| {
                let new_size = unrestricted_text_size(&mut buffer.borrow_with(font_system));
                let new_size = if self.is_editable {
                    Size::new(max(new_size.x(), CURSOR_SIZE_X.ppx()), new_size.y())
                } else {
                    new_size
                };
                buffer.set_size(
                    font_system,
                    Some(new_size.x().to_i32() as f32),
                    Some(new_size.y().to_i32() as f32),
                );
                new_size
            })
        });
        if size != self.size {
            self.size = size;
            self.base.size_hint_changed();
            self.request_scroll();
        }
    }

    pub fn set_cursor_hidden(&mut self, hidden: bool) {
        self.editor.set_cursor_hidden(hidden);
        self.is_cursor_hidden = hidden;
        self.base.update();
    }

    pub fn is_cursor_hidden(&self) -> bool {
        self.is_cursor_hidden
    }

    pub fn selection_bounds(&self) -> Option<(Cursor, Cursor)> {
        if self.editor.has_selection() {
            self.editor.selection_bounds()
        } else {
            None
        }
    }

    pub fn selected_text(&mut self) -> Option<String> {
        self.editor.copy_selection().filter(|s| !s.is_empty())
    }

    fn handle_main_click(&mut self, event: MouseInputEvent) -> Result<()> {
        if !self.is_editable {
            return Ok(());
        }
        let window = self.base.window_or_err()?;

        if !self.base.is_focused() {
            if let Some(host_id) = self.host_id {
                self.base
                    .app()
                    .set_focus(window.id(), host_id, FocusReason::Mouse);
            }
        }

        let old_cursor = self.editor.cursor();
        let preedit_range = self.editor.preedit_range();
        let click_cursor = self.editor.with_buffer(|buffer| {
            buffer.hit(event.pos.x().to_i32() as f32, event.pos.y().to_i32() as f32)
        });
        if let Some(click_cursor) = click_cursor {
            if click_cursor.line == old_cursor.line
                && preedit_range
                    .as_ref()
                    .is_some_and(|ime_range| ime_range.contains(&click_cursor.index))
            {
                // Click is inside IME preedit, so we ignore it.
                self.forbid_mouse_interaction = true;
            } else {
                // Click is outside IME preedit, so we insert the preedit text
                // as real text and cancel IME preedit.
                self.interrupt_preedit();
                self.shape_as_needed();
                let x = event.pos.x().to_i32();
                let y = event.pos.y().to_i32();
                let window = self.base.window_or_err()?;
                match ((event.num_clicks - 1) % 3) + 1 {
                    1 => self.action(Action::Click {
                        x,
                        y,
                        select: window.modifiers().shift_key(),
                    }),
                    2 => self.action(Action::DoubleClick { x, y }),
                    3 => self.action(Action::TripleClick { x, y }),
                    _ => {}
                }
            }
        }
        self.base.update();
        self.request_scroll();
        Ok(())
    }
}

impl Widget for Text {
    impl_widget_base!();

    fn handle_window_focus_change(&mut self, event: WindowFocusChangeEvent) -> Result<()> {
        if !event.is_window_focused {
            self.interrupt_preedit();
        }
        self.reset_blink_timer();
        Ok(())
    }

    fn handle_mouse_input(&mut self, event: MouseInputEvent) -> Result<bool> {
        if !self.is_editable {
            return Ok(false);
        }
        if event.state == ElementState::Pressed {
            match event.button {
                MouseButton::Left => {
                    self.handle_main_click(event)?;
                }
                MouseButton::Right => {
                    // TODO: context menu
                }
                MouseButton::Middle => {
                    #[cfg(all(
                        unix,
                        not(any(
                            target_os = "macos",
                            target_os = "android",
                            target_os = "emscripten"
                        ))
                    ))]
                    {
                        self.handle_main_click(event)?;
                        self.paste_selection();
                    }
                }
                _ => {}
            }
        }
        let is_released = self
            .base
            .window()
            .is_some_and(|window| !window.any_mouse_buttons_pressed());
        if is_released {
            self.forbid_mouse_interaction = false;
        }
        // TODO: notify parent?
        //self.adjust_scroll();
        self.reset_blink_timer();
        self.request_scroll();

        Ok(true)
    }

    fn handle_mouse_move(&mut self, event: MouseMoveEvent) -> Result<bool> {
        if !self.is_editable {
            return Ok(false);
        }
        let window = self.base.window_or_err()?;
        if window.is_mouse_button_pressed(MouseButton::Left) {
            let old_selection = (self.select_opt(), self.editor.cursor());
            self.action(Action::Drag {
                x: event.pos.x().to_i32(),
                y: event.pos.y().to_i32(),
            });
            let new_selection = (self.select_opt(), self.editor.cursor());
            if old_selection != new_selection {
                // TODO: notify parent?
                //self.adjust_scroll();
                self.base.update();
            }
        }
        self.request_scroll();
        Ok(true)
    }

    fn handle_draw(&mut self, event: DrawEvent) -> Result<()> {
        event.draw_pixmap(Point::default(), self.pixmap().as_ref(), Default::default());
        if self.is_editable && self.is_host_focused {
            if let Some(editor_cursor) = self.cursor_position() {
                // We specify an area below the input because on Windows
                // the IME window obscures the specified area.
                let rect_in_window = self.base.rect_in_window_or_err()?;
                let window = self.base.window_or_err()?;
                let top_left = rect_in_window.top_left()
                    + editor_cursor
                    + Point::new(
                        0.ppx(),
                        PhysicalPixels::from_i32(self.line_height().ceil() as i32),
                    );
                let size = rect_in_window.size(); // TODO: self_viewport_rect.size
                window.set_ime_cursor_area(Rect::from_pos_size(top_left, size));
            }
        }

        Ok(())
    }

    fn handle_accessibility_action(&mut self, event: AccessibilityActionEvent) -> Result<()> {
        match event.action {
            accesskit::Action::Click => {
                // TODO: separate reason?
                self.base.set_focus(FocusReason::Mouse);
            }
            accesskit::Action::SetTextSelection => {
                let Some(ActionData::SetTextSelection(data)) = event.data else {
                    warn!("expected SetTextSelection in data, got {:?}", event.data);
                    return Ok(());
                };
                self.handle_accessibility_set_selection_action(data);
                // TODO: notify parent
                //self.adjust_scroll();
                self.base.update();
                self.reset_blink_timer();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_accessibility_node_request(&mut self) -> Result<Option<accesskit::Node>> {
        let mut line_node = accesskit::Node::new(Role::TextInput);
        let line = self.accessibility_line();
        line_node.set_text_direction(line.text_direction);
        line_node.set_value(line.text);
        line_node.set_character_lengths(line.character_lengths);
        line_node.set_character_positions(line.character_positions);
        line_node.set_character_widths(line.character_widths);
        line_node.set_word_lengths(line.word_lengths);

        if let Some(rect_in_window) = self.base.rect_in_window() {
            line_node.set_bounds(rect_in_window.into());
        }

        let Some(window) = self.base.window() else {
            return Ok(None);
        };
        window.accessibility_node_updated(self.line_accessibility_node_id, Some(line_node));

        // TODO: configurable role
        let role = if self.is_multiline {
            Role::TextInput
        } else {
            Role::MultilineTextInput
        };
        let mut node = accesskit::Node::new(role);
        // TODO: use label widget and `Node::set_labeled_by`
        node.set_label("some input");
        node.add_action(accesskit::Action::Click);
        node.set_text_selection(self.selection_accessibility_info(self.line_accessibility_node_id));
        Ok(Some(node))
    }

    fn handle_size_hint_x_request(&self, _size_y: Option<PhysicalPixels>) -> Result<SizeHint> {
        Ok(SizeHint::new_fixed(self.size_x(), self.size_x()))
    }

    fn handle_size_hint_y_request(&self, _size_x: PhysicalPixels) -> Result<SizeHint> {
        // TODO: use size_x, handle multiple lines
        Ok(SizeHint::new_fixed(self.size_y(), self.size_y()))
    }
}

const MEASURE_MAX_SIZE: f32 = 10_000.;

fn unrestricted_text_size(buffer: &mut BorrowedWithFontSystem<'_, Buffer>) -> Size {
    buffer.set_size(Some(MEASURE_MAX_SIZE), Some(MEASURE_MAX_SIZE));
    buffer.shape_until_scroll(false);
    let height = (buffer.lines.len() as f32 * buffer.metrics().line_height).ceil() as i32;
    let width = buffer
        .layout_runs()
        .map(|run| run.line_w.ceil() as i32)
        .max()
        .unwrap_or(0);

    Size::new(
        PhysicalPixels::from_i32(width),
        PhysicalPixels::from_i32(height),
    )
}

fn convert_color(color: Color) -> cosmic_text::Color {
    let c = color.to_color_u8();
    cosmic_text::Color::rgba(c.red(), c.green(), c.blue(), c.alpha())
}

impl Drop for Text {
    fn drop(&mut self) {
        if let Some(window) = self.base.window() {
            window.update_accessibility_node(
                Some(self.base.id().into()),
                self.line_accessibility_node_id,
            );
        }
    }
}
