use {
    widgem::{
        layout::Layout,
        types::Axis,
        widgets::{Button, Label, ScrollBar, TextInput},
        WidgetExt, Window,
    },
    widgem_tester::Context,
};

#[widgem_tester::test]
fn main(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        let mut contents = r
            .set_main_content(Window::init(module_path!().into()))?
            .set_layout(Layout::ExplicitGrid)
            .contents_mut();

        let mut current_cell_y = 0;
        contents
            .set_next_item(Label::init("Text input label:".into()))?
            .set_grid_cell(0, current_cell_y);
        contents
            .set_next_item(TextInput::init())?
            .set_grid_cell(1, current_cell_y);

        current_cell_y += 1;
        contents
            .set_next_item(Label::init("Scroll bar label:".into()))?
            .set_grid_cell(0, current_cell_y);
        contents
            .set_next_item(ScrollBar::init(Axis::X))?
            .set_grid_cell(1, current_cell_y);

        current_cell_y += 1;
        contents
            .set_next_item(Label::init("Multiline label\nSecond line".into()))?
            .set_grid_cell(1, current_cell_y);

        current_cell_y += 1;
        contents
            .set_next_item(Button::init("Submit".into()))?
            .set_grid_cell(1, current_cell_y);

        Ok(())
    })?;
    Ok(())
}
