use widgem::{
    widgets::{Column, Label, ScrollArea},
    WidgetExt, Window,
};
use widgem_tester::context::Context;

#[widgem_tester::test]
pub fn scroll_area(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|root| {
        let mut root_items = root.items_mut();
        let mut window_items = root_items
            .set_next_item(Window::init(module_path!().into()))
            .set_padding_enabled(false)
            .items_mut();
        let mut content_items = window_items
            .set_next_item(ScrollArea::init())
            .set_size_x_fixed(Some(false))
            .set_content(Column::init())
            .items_mut();

        for i in 0..20 {
            content_items.set_next_item(Label::init(format!("text item {i}")));
        }
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.snapshot("scroll area")?;
    window.resize(150, 150)?;
    window.snapshot("resized 150x150")?;
    // scroll down button
    window.mouse_move(146, 146)?;
    ctx.ui().mouse_left_click()?;
    window.snapshot("step down")?;
    ctx.ui().mouse_scroll_down()?;
    window.snapshot("scroll down")?;
    window.resize(110, 150)?;
    window.snapshot("resized 110x150")?;
    window.resize(100, 150)?;
    window.snapshot("resized 100x150")?;
    // horizontal scroll slider
    window.mouse_move(30, 145)?;
    ctx.ui().mouse_left_press()?;
    window.mouse_move(100, 145)?;
    ctx.ui().mouse_left_release()?;
    window.snapshot("scroll right")?;
    window.resize(160, 150)?;
    window.snapshot("resized 160x150")?;
    window.resize(160, 500)?;
    window.snapshot("resized 160x500")?;
    window.resize(160, 600)?;
    window.snapshot("resized 160x600")?;

    window.close()?;
    Ok(())
}

#[widgem_tester::test]
pub fn layout(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|root| {
        let mut root_items = root.items_mut();
        let mut window_items = root_items
            .set_next_item(Window::init(module_path!().into()))
            .items_mut();
        window_items.set_next_item(Label::init("before".into()));
        let mut content = window_items
            .set_next_item(ScrollArea::init())
            .set_content(Column::init())
            .set_padding_enabled(false)
            .items_mut();

        for i in 0..20 {
            content.set_next_item(Label::init(format!("text item {i}")));
        }
        window_items.set_next_item(Label::init("after".into()));
        Ok(())
    })?;

    let window = ctx.wait_for_window_by_pid()?;
    window.snapshot("initial")?;
    window.resize(300, 800)?;
    window.snapshot("resized to 300x800")?;
    window.resize(300, 350)?;
    window.snapshot("resized to 300x350")?;

    window.close()?;
    Ok(())
}
