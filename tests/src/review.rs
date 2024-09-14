use salvation::{
    impl_widget_common,
    layout::LayoutItemOptions,
    tiny_skia::Pixmap,
    widgets::{
        button::Button, image::Image, label::Label, row::Row, Widget, WidgetCommon, WidgetExt,
        WidgetId,
    },
    WindowAttributes,
};

pub struct ReviewWidget {
    common: WidgetCommon,
    test_name_id: WidgetId<Label>,
    snapshot_name_id: WidgetId<Label>,
    image_id: WidgetId<Image>,
}

impl ReviewWidget {
    pub fn new() -> Self {
        let mut common = WidgetCommon::new();
        common.add_child(
            Label::new("Test:").boxed(),
            LayoutItemOptions::from_pos_in_grid(1, 1),
        );
        let test_name = Label::new("").with_id();
        common.add_child(
            test_name.widget.boxed(),
            LayoutItemOptions::from_pos_in_grid(2, 1),
        );

        common.add_child(
            Label::new("Snapshot:").boxed(),
            LayoutItemOptions::from_pos_in_grid(1, 2),
        );
        let snapshot_name = Label::new("").with_id();
        common.add_child(
            snapshot_name.widget.boxed(),
            LayoutItemOptions::from_pos_in_grid(2, 2),
        );

        common.add_child(
            Label::new("Display mode:").boxed(),
            LayoutItemOptions::from_pos_in_grid(1, 3),
        );
        // TODO: radio buttons
        common.add_child(
            Row::new()
                .with_child(Button::new("New").boxed())
                .with_child(Button::new("Confirmed").boxed())
                .with_child(Button::new("Previous confirmed").boxed())
                .with_child(Button::new("Diff with confirmed").boxed())
                .with_child(Button::new("Diff with previous confirmed").boxed())
                .boxed(),
            LayoutItemOptions::from_pos_in_grid(2, 3),
        );

        common.add_child(
            Label::new("Snapshot:").boxed(),
            LayoutItemOptions::from_pos_in_grid(1, 4),
        );
        // TODO: allow no pixmap in image
        let image = Image::new(Pixmap::new(1, 1).unwrap()).with_id();
        common.add_child(
            image.widget.boxed(),
            LayoutItemOptions::from_pos_in_grid(2, 4),
        );

        common.add_child(
            Label::new("Actions:").boxed(),
            LayoutItemOptions::from_pos_in_grid(1, 5),
        );
        common.add_child(
            Row::new()
                .with_child(Button::new("Approve").boxed())
                .with_child(Button::new("Skip snapshot").boxed())
                .with_child(Button::new("Skip test").boxed())
                .boxed(),
            LayoutItemOptions::from_pos_in_grid(2, 5),
        );

        Self {
            common,
            test_name_id: test_name.id,
            snapshot_name_id: snapshot_name.id,
            image_id: image.id,
        }
        .with_window(WindowAttributes::default().with_title("salvation test review"))
    }
}

impl Widget for ReviewWidget {
    impl_widget_common!();
}
