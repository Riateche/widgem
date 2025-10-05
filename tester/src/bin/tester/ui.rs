use {
    crate::logic::{Mode, TesterLogic},
    std::thread,
    strum::IntoEnumIterator,
    tracing::{info, warn},
    widgem::{
        event::Event,
        impl_widget_base,
        layout::Layout,
        system::ReportError,
        types::Point,
        widget_initializer,
        widgets::{Button, Column, Image, Label, Row, ScrollArea, Window},
        Widget, WidgetBaseOf, WidgetExt, WidgetId, WidgetInitializer,
    },
};

pub struct TesterUi {
    base: WidgetBaseOf<Self>,
    tester_logic: TesterLogic,
    coords: String,
    image_scale: f32,
}

// TODO: translate
fn mode_ui_name(mode: Mode) -> &'static str {
    match mode {
        Mode::New => "New",
        Mode::Confirmed => "Confirmed",
        Mode::DiffWithConfirmed => "Diff with confirmed",
        Mode::DiffWithPreviousConfirmed => "Diff with previous confirmed",
    }
}

impl TesterUi {
    fn new(base: WidgetBaseOf<Self>, tester_logic: TesterLogic) -> Self {
        TesterUi {
            base,
            tester_logic,
            coords: String::new(),
            image_scale: 1.0,
        }
    }

    pub fn init(tester_logic: TesterLogic) -> impl WidgetInitializer<Output = Self> {
        widget_initializer::from_new_and_set(Self::new, Self::set_tester_logic, tester_logic)
    }

    pub fn set_tester_logic(&mut self, reviewer: TesterLogic) -> &mut Self {
        self.tester_logic = reviewer;
        self.base.update();
        self
    }

    fn set_mode(&mut self, mode: Mode) -> anyhow::Result<()> {
        self.tester_logic.set_mode(mode);
        self.base.update();
        Ok(())
    }

    fn image_mouse_move(
        &mut self,
        (image_id, pos_in_widget): (WidgetId<Image>, Option<Point>),
    ) -> anyhow::Result<()> {
        let Some(pos_in_widget) = pos_in_widget else {
            self.coords.clear();
            self.base.update();
            return Ok(());
        };
        let pos_in_content = self
            .base
            .find_child_mut(image_id)?
            .map_widget_pos_to_content_pos(pos_in_widget);
        self.coords = format!(
            "X: {}; Y: {}",
            pos_in_content.x().to_i32(),
            pos_in_content.y().to_i32()
        );
        self.base.update();
        Ok(())
    }

    fn test_finished(&mut self) -> anyhow::Result<()> {
        self.tester_logic.refresh()?;
        self.base.update();
        Ok(())
    }
}

impl Widget for TesterUi {
    impl_widget_base!();

    fn handle_declare_children_request(&mut self) -> anyhow::Result<()> {
        let callbacks = self.base.callback_creator();

        let mut window_items = self
            .base
            .set_child(0, Window::init("widgem snapshot review".into()))?
            .set_layout(Layout::ExplicitGrid)
            .contents_mut();
        let mut current_row = 1;
        window_items
            .set_next_item(Label::init("Test:".into()))?
            .set_grid_cell(1, current_row);
        let test_case_name = self
            .tester_logic
            .current_test_case_name()
            .and_then(|name| {
                Some(format!(
                    "({}/{}) {:?}",
                    self.tester_logic.current_test_case_index()? + 1,
                    self.tester_logic.tests().num_tests(),
                    name
                ))
            })
            .unwrap_or_else(|| "none".into());
        window_items
            .set_next_item(Label::init(test_case_name))?
            .set_grid_cell(2, current_row);
        current_row += 1;

        let mut row_items = window_items
            .set_next_item(Row::init())?
            .set_grid_cell(2, current_row)
            .set_padding_enabled(false)
            .contents_mut();
        current_row += 1;

        row_items
            .set_next_item(Button::init("First test".into()))?
            .on_triggered(callbacks.create(move |w, _e| {
                w.tester_logic.go_to_first_test_case();
                w.base.update();
                Ok(())
            }));

        row_items
            .set_next_item(Button::init("Previous test".into()))?
            .on_triggered(callbacks.create(move |w, _e| {
                w.tester_logic.go_to_previous_test_case();
                w.base.update();
                Ok(())
            }));
        row_items
            .set_next_item(Button::init("Next test".into()))?
            .on_triggered(callbacks.create(move |w, _e| {
                w.tester_logic.go_to_next_test_case();
                w.base.update();
                Ok(())
            }));
        row_items
            .set_next_item(Button::init("Last test".into()))?
            .on_triggered(callbacks.create(move |w, _e| {
                w.tester_logic.go_to_last_test_case();
                w.base.update();
                Ok(())
            }));

        row_items
            .set_next_item(Button::init("Refresh list".into()))?
            .on_triggered(callbacks.create(move |w, _e| {
                w.tester_logic.refresh()?;
                w.base.update();
                Ok(())
            }));

        let mut row_items = window_items
            .set_next_item(Row::init())?
            .set_grid_cell(2, current_row)
            .set_padding_enabled(false)
            .contents_mut();
        current_row += 1;

        row_items
            .set_next_item(Button::init("Run test subject".into()))?
            .set_enabled(self.tester_logic.current_test_case_name().is_some())
            .on_triggered(callbacks.create(move |w, _e| w.tester_logic.run_test_subject()));

        let test_finished = callbacks.create(move |w, _e: ()| w.test_finished());
        row_items
            .set_next_item(Button::init("Run test".into()))?
            .set_enabled(self.tester_logic.current_test_case_name().is_some())
            .on_triggered(callbacks.create(move |w, _e| {
                let mut child = w.tester_logic.run_test()?;
                info!("spawned process with pid: {:?}", child.id());
                let test_finished = test_finished.clone();
                thread::spawn(move || {
                    match child.wait() {
                        Ok(status) => {
                            info!("child {:?} finished with status {:?}", child.id(), status);
                        }
                        Err(err) => {
                            warn!("child {:?} wait error: {:?}", child.id(), err);
                        }
                    }
                    test_finished.invoke(());
                });
                Ok(())
            }));

        window_items
            .set_next_item(Label::init("Snapshot:".into()))?
            .set_grid_cell(1, current_row);

        let snapshot_name = match self.tester_logic.mode() {
            Mode::New | Mode::DiffWithConfirmed | Mode::DiffWithPreviousConfirmed => self
                .tester_logic
                .unconfirmed_description()
                .map(|s| s.to_owned()),
            Mode::Confirmed => self
                .tester_logic
                .confirmed_description()
                .map(|s| s.to_owned()),
        }
        .and_then(|description| {
            let index = self.tester_logic.current_snapshot_index()?;
            Some(format!(
                "({}/{}) {:?}",
                index,
                self.tester_logic.num_current_snapshots(),
                description
            ))
        })
        .unwrap_or_else(|| "none".into());
        window_items
            .set_next_item(Label::init(snapshot_name))?
            .set_grid_cell(2, current_row);
        current_row += 1;

        let mut row_items = window_items
            .set_next_item(Row::init())?
            .set_grid_cell(2, current_row)
            .set_padding_enabled(false)
            .contents_mut();
        current_row += 1;

        row_items
            .set_next_item(Button::init("Previous snapshot".into()))?
            .on_triggered(callbacks.create(move |w, _e| {
                w.tester_logic.go_to_previous_snapshot();
                w.base.update();
                Ok(())
            }));
        row_items
            .set_next_item(Button::init("Next snapshot".into()))?
            .on_triggered(callbacks.create(move |w, _e| {
                w.tester_logic.go_to_next_snapshot();
                w.base.update();
                Ok(())
            }));

        window_items
            .set_next_item(Label::init("Display mode:".into()))?
            .set_grid_cell(1, current_row);

        // TODO: radio buttons
        let mut modes_row_items = window_items
            .set_next_item(Row::init())?
            .set_grid_cell(2, current_row)
            .set_padding_enabled(false)
            .contents_mut();
        current_row += 1;

        for mode in Mode::iter() {
            // TODO: radio buttons
            let star = if self.tester_logic.mode() == mode {
                "* "
            } else {
                ""
            };
            modes_row_items
                .set_next_item(Button::init(format!("{}{}", star, mode_ui_name(mode))))?
                .set_enabled(self.tester_logic.is_mode_allowed(mode))
                .on_triggered(callbacks.create(move |w, _e| w.set_mode(mode)));
        }

        let pixmap = self.tester_logic.pixmap().or_report_err().flatten();

        window_items
            .set_next_item(Label::init("Snapshot size:".into()))?
            .set_grid_cell(1, current_row);
        window_items
            .set_next_item(Label::init({
                if let Some(pixmap) = &pixmap {
                    format!(
                        "{} x {}",
                        pixmap.size_x().to_i32(),
                        pixmap.size_y().to_i32(),
                    )
                } else {
                    "".into()
                }
            }))?
            .set_grid_cell(2, current_row);
        current_row += 1;

        window_items
            .set_next_item(Label::init("Zoom:".into()))?
            .set_grid_cell(1, current_row);

        let mut row_items = window_items
            .set_next_item(Row::init())?
            .set_grid_cell(2, current_row)
            .set_padding_enabled(false)
            .contents_mut();
        current_row += 1;

        row_items
            .set_next_item(Button::init("100%".into()))?
            .on_triggered(callbacks.create(move |w, _e| {
                w.image_scale = 1.0;
                w.base.update();
                Ok(())
            }));
        row_items
            .set_next_item(Button::init("200%".into()))?
            .on_triggered(callbacks.create(move |w, _e| {
                w.image_scale = 2.0;
                w.base.update();
                Ok(())
            }));
        row_items
            .set_next_item(Button::init("400%".into()))?
            .on_triggered(callbacks.create(move |w, _e| {
                w.image_scale = 4.0;
                w.base.update();
                Ok(())
            }));
        row_items
            .set_next_item(Button::init("800%".into()))?
            .on_triggered(callbacks.create(move |w, _e| {
                w.image_scale = 8.0;
                w.base.update();
                Ok(())
            }));
        row_items.set_next_item(Label::init(self.coords.clone()))?;

        let image = window_items
            .set_next_item(ScrollArea::init())?
            .set_grid_cell(2, current_row)
            .set_content(Column::init())?
            .set_style("Column { background: #55c080; padding: 2px; }")
            .base_mut()
            .set_child(0, Image::init(pixmap))?
            .set_scale(Some(self.image_scale));
        current_row += 1;

        let image_mouse_move = callbacks.create(Self::image_mouse_move);
        let image_id = image.id();
        image
            .base_mut()
            // TODO: special event filter object like `Callback`
            .install_event_filter(callbacks.id().raw(), move |event| {
                match event {
                    Event::MouseMove(event) => {
                        image_mouse_move.invoke((image_id, Some(event.pos())));
                    }
                    Event::MouseLeave(_) => {
                        image_mouse_move.invoke((image_id, None));
                    }
                    _ => (),
                }
                Ok(false)
            });

        window_items
            .set_next_item(Label::init("Actions:".into()))?
            .set_grid_cell(1, current_row);

        let mut approve_and_skip_items = window_items
            .set_next_item(Row::init())?
            .set_grid_cell(2, current_row)
            .set_padding_enabled(false)
            .contents_mut();
        current_row += 1;

        approve_and_skip_items
            .set_next_item(Button::init("Approve".into()))?
            .on_triggered(callbacks.create(move |w, _e| {
                w.tester_logic.approve()?;
                w.base_mut().update();
                Ok(())
            }));
        approve_and_skip_items
            .set_next_item(Button::init("Skip snapshot".into()))?
            .on_triggered(callbacks.create(move |w, _e| {
                if !w.tester_logic.go_to_next_unconfirmed_snapshot() {
                    w.base.app().exit();
                }
                w.base.update();
                Ok(())
            }));
        #[allow(clippy::collapsible_if)]
        approve_and_skip_items
            .set_next_item(Button::init("Skip test".into()))?
            .set_enabled(self.tester_logic.has_unconfirmed())
            .on_triggered(callbacks.create(move |w, _e| {
                w.tester_logic.go_to_next_test_case();
                if !w.tester_logic.has_unconfirmed() {
                    if !w.tester_logic.go_to_next_unconfirmed_snapshot() {
                        w.base.app().exit();
                    }
                }
                w.base.update();
                Ok(())
            }));

        let unconfirmed_count = self.tester_logic.tests().unconfirmed_snapshot_count();
        window_items
            .set_next_item(Label::init(if unconfirmed_count > 0 {
                format!("Unconfirmed snapshots remaining: {}", unconfirmed_count)
            } else {
                "No unconfirmed snapshots.".into()
            }))?
            .set_grid_cell(2, current_row);

        Ok(())
    }
}
