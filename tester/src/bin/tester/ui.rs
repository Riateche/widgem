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
        widgets::{
            Button, Column, Image, Label, NewWidget, Row, ScrollArea, Widget, WidgetBaseOf,
            WidgetExt, WidgetId, Window,
        },
    },
};

pub struct TesterUi {
    base: WidgetBaseOf<Self>,
    reviewer: TesterLogic,
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
    pub fn set_reviewer(&mut self, reviewer: TesterLogic) -> anyhow::Result<()> {
        self.reviewer = reviewer;
        self.base.update();
        Ok(())
    }

    fn set_mode(&mut self, mode: Mode) -> anyhow::Result<()> {
        self.reviewer.set_mode(mode);
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
        self.reviewer.refresh()?;
        self.base.update();
        Ok(())
    }
}

impl NewWidget for TesterUi {
    type Arg = TesterLogic;

    #[allow(clippy::collapsible_if)]
    fn new(base: WidgetBaseOf<Self>, reviewer: Self::Arg) -> Self {
        Self {
            base,
            reviewer,
            coords: String::new(),
            image_scale: 1.0,
        }
    }

    fn handle_declared(&mut self, arg: Self::Arg) {
        self.set_reviewer(arg).or_report_err();
    }
}

impl Widget for TesterUi {
    impl_widget_base!();

    fn handle_declare_children_request(&mut self) -> anyhow::Result<()> {
        let callbacks = self.base.callback_creator();

        let window = self
            .base
            .declare_child::<Window>("widgem snapshot review".into());
        window.set_layout(Layout::ExplicitGrid);
        let mut current_row = 1;
        window
            .base_mut()
            .declare_child::<Label>("Test:".into())
            .set_grid_cell(1, current_row);
        let test_case_name = self
            .reviewer
            .current_test_case_name()
            .and_then(|name| {
                Some(format!(
                    "({}/{}) {:?}",
                    self.reviewer.current_test_case_index()? + 1,
                    self.reviewer.tests().num_tests(),
                    name
                ))
            })
            .unwrap_or_else(|| "none".into());
        window
            .base_mut()
            .declare_child::<Label>(test_case_name)
            .set_grid_cell(2, current_row);
        current_row += 1;

        let row = window
            .base_mut()
            .declare_child::<Row>(())
            .set_grid_cell(2, current_row)
            .set_padding_enabled(false);
        current_row += 1;

        row.base_mut()
            .declare_child::<Button>("First test".into())
            .on_triggered(callbacks.create(move |w, _e| {
                w.reviewer.go_to_first_test_case();
                w.base.update();
                Ok(())
            }));

        row.base_mut()
            .declare_child::<Button>("Previous test".into())
            .on_triggered(callbacks.create(move |w, _e| {
                w.reviewer.go_to_previous_test_case();
                w.base.update();
                Ok(())
            }));
        row.base_mut()
            .declare_child::<Button>("Next test".into())
            .on_triggered(callbacks.create(move |w, _e| {
                w.reviewer.go_to_next_test_case();
                w.base.update();
                Ok(())
            }));
        row.base_mut()
            .declare_child::<Button>("Last test".into())
            .on_triggered(callbacks.create(move |w, _e| {
                w.reviewer.go_to_last_test_case();
                w.base.update();
                Ok(())
            }));

        row.base_mut()
            .declare_child::<Button>("Refresh list".into())
            .on_triggered(callbacks.create(move |w, _e| {
                w.reviewer.refresh()?;
                w.base.update();
                Ok(())
            }));

        let row = window
            .base_mut()
            .declare_child::<Row>(())
            .set_grid_cell(2, current_row)
            .set_padding_enabled(false);
        current_row += 1;

        row.base_mut()
            .declare_child::<Button>("Run test subject".into())
            .set_enabled(self.reviewer.current_test_case_name().is_some())
            .on_triggered(callbacks.create(move |w, _e| w.reviewer.run_test_subject()));

        let test_finished = callbacks.create(move |w, _e: ()| w.test_finished());
        row.base_mut()
            .declare_child::<Button>("Run test".into())
            .set_enabled(self.reviewer.current_test_case_name().is_some())
            .on_triggered(callbacks.create(move |w, _e| {
                let mut child = w.reviewer.run_test()?;
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

        window
            .base_mut()
            .declare_child::<Label>("Snapshot:".into())
            .set_grid_cell(1, current_row);

        let snapshot_name = match self.reviewer.mode() {
            Mode::New | Mode::DiffWithConfirmed | Mode::DiffWithPreviousConfirmed => self
                .reviewer
                .unconfirmed_description()
                .map(|s| s.to_owned()),
            Mode::Confirmed => self.reviewer.confirmed_description().map(|s| s.to_owned()),
        }
        .and_then(|description| {
            let index = self.reviewer.current_snapshot_index()?;
            Some(format!(
                "({}/{}) {:?}",
                index,
                self.reviewer.num_current_snapshots(),
                description
            ))
        })
        .unwrap_or_else(|| "none".into());
        window
            .base_mut()
            .declare_child::<Label>(snapshot_name)
            .set_grid_cell(2, current_row);
        current_row += 1;

        let row = window
            .base_mut()
            .declare_child::<Row>(())
            .set_grid_cell(2, current_row)
            .set_padding_enabled(false);
        current_row += 1;

        row.base_mut()
            .declare_child::<Button>("Previous snapshot".into())
            .on_triggered(callbacks.create(move |w, _e| {
                w.reviewer.go_to_previous_snapshot();
                w.base.update();
                Ok(())
            }));
        row.base_mut()
            .declare_child::<Button>("Next snapshot".into())
            .on_triggered(callbacks.create(move |w, _e| {
                w.reviewer.go_to_next_snapshot();
                w.base.update();
                Ok(())
            }));

        window
            .base_mut()
            .declare_child::<Label>("Display mode:".into())
            .set_grid_cell(1, current_row);

        // TODO: radio buttons
        let modes_row = window
            .base_mut()
            .declare_child::<Row>(())
            .set_grid_cell(2, current_row)
            .set_padding_enabled(false);
        current_row += 1;

        for mode in Mode::iter() {
            // TODO: radio buttons
            let star = if self.reviewer.mode() == mode {
                "* "
            } else {
                ""
            };
            modes_row
                .base_mut()
                .declare_child::<Button>(format!("{}{}", star, mode_ui_name(mode)))
                .set_enabled(self.reviewer.is_mode_allowed(mode))
                .on_triggered(callbacks.create(move |w, _e| w.set_mode(mode)));
        }

        let pixmap = self.reviewer.pixmap().or_report_err().flatten();

        window
            .base_mut()
            .declare_child::<Label>("Snapshot size:".into())
            .set_grid_cell(1, current_row);
        window
            .base_mut()
            .declare_child::<Label>({
                if let Some(pixmap) = &pixmap {
                    format!(
                        "{} x {}",
                        pixmap.size_x().to_i32(),
                        pixmap.size_y().to_i32(),
                    )
                } else {
                    "".into()
                }
            })
            .set_grid_cell(2, current_row);
        current_row += 1;

        window
            .base_mut()
            .declare_child::<Label>("Zoom:".into())
            .set_grid_cell(1, current_row);

        let row = window
            .base_mut()
            .declare_child::<Row>(())
            .set_grid_cell(2, current_row)
            .set_padding_enabled(false);
        current_row += 1;

        row.base_mut()
            .declare_child::<Button>("100%".into())
            .on_triggered(callbacks.create(move |w, _e| {
                w.image_scale = 1.0;
                w.base.update();
                Ok(())
            }));
        row.base_mut()
            .declare_child::<Button>("200%".into())
            .on_triggered(callbacks.create(move |w, _e| {
                w.image_scale = 2.0;
                w.base.update();
                Ok(())
            }));
        row.base_mut()
            .declare_child::<Button>("400%".into())
            .on_triggered(callbacks.create(move |w, _e| {
                w.image_scale = 4.0;
                w.base.update();
                Ok(())
            }));
        row.base_mut()
            .declare_child::<Button>("800%".into())
            .on_triggered(callbacks.create(move |w, _e| {
                w.image_scale = 8.0;
                w.base.update();
                Ok(())
            }));
        row.base_mut().declare_child::<Label>(self.coords.clone());

        let image = window
            .base_mut()
            .declare_child::<ScrollArea>(())
            .set_grid_cell(2, current_row)
            .declare_content::<Column>(())
            .set_style("Column { background: #55c080; padding: 2px; }")
            .base_mut()
            .declare_child::<Image>(pixmap)
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

        window
            .base_mut()
            .declare_child::<Label>("Actions:".into())
            .set_grid_cell(1, current_row);

        let approve_and_skip = window
            .base_mut()
            .declare_child::<Row>(())
            .set_grid_cell(2, current_row)
            .set_padding_enabled(false);
        current_row += 1;

        approve_and_skip
            .base_mut()
            .declare_child::<Button>("Approve".into())
            .on_triggered(callbacks.create(move |w, _e| {
                w.reviewer.approve()?;
                w.base_mut().update();
                Ok(())
            }));
        approve_and_skip
            .base_mut()
            .declare_child::<Button>("Skip snapshot".into())
            .on_triggered(callbacks.create(move |w, _e| {
                if !w.reviewer.go_to_next_unconfirmed_snapshot() {
                    widgem::exit();
                }
                w.base.update();
                Ok(())
            }));
        #[allow(clippy::collapsible_if)]
        approve_and_skip
            .base_mut()
            .declare_child::<Button>("Skip test".into())
            .set_enabled(self.reviewer.has_unconfirmed())
            .on_triggered(callbacks.create(move |w, _e| {
                w.reviewer.go_to_next_test_case();
                if !w.reviewer.has_unconfirmed() {
                    if !w.reviewer.go_to_next_unconfirmed_snapshot() {
                        widgem::exit();
                    }
                }
                w.base.update();
                Ok(())
            }));

        let unconfirmed_count = self.reviewer.tests().unconfirmed_snapshot_count();
        window
            .base_mut()
            .declare_child::<Label>(if unconfirmed_count > 0 {
                format!("Unconfirmed snapshots remaining: {}", unconfirmed_count)
            } else {
                "No unconfirmed snapshots.".into()
            })
            .set_grid_cell(2, current_row);

        Ok(())
    }
}
