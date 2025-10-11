use {
    crate::logic::{Mode, TesterLogic},
    anyhow::Context as _,
    itertools::Itertools,
    std::{
        collections::VecDeque,
        io::{BufRead, BufReader, Read},
        process::Child,
        sync::{Arc, Mutex},
        thread::{self, sleep},
        time::Duration,
    },
    strum::IntoEnumIterator,
    tracing::{info, warn},
    widgem::{
        event::Event,
        impl_widget_base,
        layout::Layout,
        system::OrWarn,
        types::Point,
        widget_initializer,
        widgets::{Button, Column, Image, Label, Row, ScrollArea, Window},
        Callback, Widget, WidgetBaseOf, WidgetExt, WidgetId, WidgetInitializer,
    },
};

pub struct TesterUi {
    base: WidgetBaseOf<Self>,
    tester_logic: TesterLogic,
    coords: String,
    image_scale: f32,
    run_test_process: Option<Arc<Mutex<Child>>>,
    test_output: VecDeque<String>,
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
            run_test_process: None,
            test_output: Default::default(),
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
        self.run_test_process = None;
        self.tester_logic.refresh()?;
        self.base.update();
        Ok(())
    }

    fn push_test_output(&mut self, line: String) -> anyhow::Result<()> {
        self.test_output.push_back(line);
        while self.test_output.len() > 20 {
            self.test_output.pop_front();
        }
        self.base.update();
        Ok(())
    }

    fn run_test(&mut self) -> anyhow::Result<()> {
        self.test_output.clear();
        let mut child = self.tester_logic.run_test()?;
        self.push_test_output(format!("test started (pid: {:?})", child.id()))?;
        let push_test_output = self.callback(Self::push_test_output);
        let stdout = child.stdout.take().context("missing stdout")?;
        let push_test_output2 = push_test_output.clone();
        thread::spawn(move || forward_output(stdout, push_test_output2));
        let stderr = child.stderr.take().context("missing stderr")?;
        let push_test_output2 = push_test_output.clone();
        thread::spawn(move || forward_output(stderr, push_test_output2));

        let child = Arc::new(Mutex::new(child));
        let child2 = Arc::clone(&child);
        self.run_test_process = Some(child);
        self.base.update();
        let test_finished = self.callback(|w, _e: ()| w.test_finished());
        thread::spawn(move || loop {
            match child2.lock().unwrap().try_wait() {
                Ok(Some(status)) => {
                    push_test_output.invoke(format!(
                        "process finished with status {}",
                        if let Some(code) = status.code() {
                            code.to_string()
                        } else {
                            format!("{:?}", status)
                        }
                    ));
                    test_finished.invoke(());
                    return;
                }
                Ok(None) => {}
                Err(err) => {
                    push_test_output
                        .invoke(format!("error while checking process status: {:?}", err));
                    test_finished.invoke(());
                    return;
                }
            }
            sleep(Duration::from_millis(100));
        });
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
            .set_enabled(self.tester_logic.tests().num_tests() > 0)
            .on_triggered(callbacks.create(move |w, _e| {
                w.tester_logic.go_to_first_test_case();
                w.base.update();
                Ok(())
            }));

        row_items
            .set_next_item(Button::init("Previous test".into()))?
            .set_enabled(self.tester_logic.has_previous_test_case())
            .on_triggered(callbacks.create(move |w, _e| {
                w.tester_logic.go_to_previous_test_case();
                w.base.update();
                Ok(())
            }));
        row_items
            .set_next_item(Button::init("Next test".into()))?
            .set_enabled(self.tester_logic.has_next_test_case())
            .on_triggered(callbacks.create(move |w, _e| {
                w.tester_logic.go_to_next_test_case();
                w.base.update();
                Ok(())
            }));
        row_items
            .set_next_item(Button::init("Last test".into()))?
            .set_enabled(self.tester_logic.tests().num_tests() > 0)
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

        row_items
            .set_next_item(Button::init("Run test".into()))?
            .set_enabled(
                self.tester_logic.current_test_case_name().is_some()
                    && self.run_test_process.is_none(),
            )
            .on_triggered(callbacks.create(move |this, _e| this.run_test()));

        row_items
            .set_next_item(Button::init("Stop".into()))?
            .set_enabled(self.run_test_process.is_some())
            .on_triggered(callbacks.create(move |this, _e| {
                let Some(child) = &this.run_test_process else {
                    return Ok(());
                };
                match child.lock().unwrap().kill() {
                    Ok(()) => info!("child kill succeeded"),
                    Err(err) => warn!(?err, "child kill failed"),
                }
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
            .set_enabled(self.tester_logic.has_previous_snapshot())
            .on_triggered(callbacks.create(move |w, _e| {
                w.tester_logic.go_to_previous_snapshot();
                w.base.update();
                Ok(())
            }));
        row_items
            .set_next_item(Button::init("Next snapshot".into()))?
            .set_enabled(self.tester_logic.has_next_snapshot())
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

        let pixmap = self.tester_logic.pixmap().or_warn().flatten();

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

        for scale in [1., 2., 4., 8.] {
            let star = if self.image_scale == scale { "* " } else { "" };
            row_items
                .set_next_item(Button::init(format!("{}{}%", star, scale * 100.)))?
                .on_triggered(callbacks.create(move |w, _e| {
                    w.image_scale = scale;
                    w.base.update();
                    Ok(())
                }));
        }

        row_items.set_next_item(Label::init(self.coords.clone()))?;

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
            .set_enabled(self.tester_logic.has_unconfirmed())
            .on_triggered(callbacks.create(move |w, _e| {
                w.tester_logic.approve()?;
                w.base_mut().update();
                Ok(())
            }));
        approve_and_skip_items
            .set_next_item(Button::init("Skip snapshot".into()))?
            .set_enabled(
                self.tester_logic.has_unconfirmed()
                    && self.tester_logic.has_next_unconfirmed_snapshot(),
            )
            .on_triggered(callbacks.create(move |w, _e| {
                w.tester_logic.go_to_next_unconfirmed_snapshot();
                w.base.update();
                Ok(())
            }));
        #[allow(clippy::collapsible_if)]
        approve_and_skip_items
            .set_next_item(Button::init("Skip test".into()))?
            .set_enabled(
                self.tester_logic.has_unconfirmed()
                    && self
                        .tester_logic
                        .has_next_unconfirmed_snapshot_in_next_tests(),
            )
            .on_triggered(callbacks.create(move |w, _e| {
                w.tester_logic
                    .go_to_next_unconfirmed_snapshot_in_next_tests();
                w.base.update();
                Ok(())
            }));
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

        let unconfirmed_count = self.tester_logic.tests().unconfirmed_snapshot_count();
        window_items
            .set_next_item(Label::init(if unconfirmed_count > 0 {
                format!("Unconfirmed snapshots remaining: {}", unconfirmed_count)
            } else {
                "No unconfirmed snapshots.".into()
            }))?
            .set_grid_cell(2, current_row);
        current_row += 1;

        window_items
            .set_next_item(Label::init("Test output:".into()))?
            .set_grid_cell(1, current_row);

        window_items
            .set_next_item(ScrollArea::init())?
            .set_grid_cell(2, current_row)
            .set_content(Label::init(self.test_output.iter().join("\n")))?;

        Ok(())
    }
}

fn forward_output(output: impl Read, callback: Callback<String>) {
    let reader = BufReader::new(output);
    for line in reader.lines() {
        match line {
            Ok(line) => callback.invoke(line),
            Err(err) => {
                warn!(?err, "error while reading child output");
            }
        }
    }
}
