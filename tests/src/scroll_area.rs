use widgem::{
    widgets::{Column, Label, ScrollArea},
    Widget, WidgetExt, Window,
};
use widgem_tester::context::Context;

#[widgem_tester::test]
pub fn scroll_area(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        let content = r
            .base_mut()
            .add_child::<Window>(module_path!().into())
            .set_padding_enabled(false)
            .base_mut()
            .add_child::<ScrollArea>(())
            .set_size_x_fixed(Some(false))
            .set_content::<Column>(());

        for i in 0..20 {
            content
                .base_mut()
                .add_child::<Label>(format!("text item {i}"));
        }
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.snapshot("scroll area")?;
    window.resize(150, 150)?;
    window.snapshot("resized 150x150")?;
    // scroll down button
    window.mouse_move(146, 146)?;
    ctx.connection().mouse_click(1)?;
    window.snapshot("step down")?;
    ctx.connection().mouse_scroll_down()?;
    window.snapshot("scroll down")?;
    window.resize(110, 150)?;
    window.snapshot("resized 110x150")?;
    window.resize(100, 150)?;
    window.snapshot("resized 100x150")?;
    // horizontal scroll slider
    window.mouse_move(30, 145)?;
    ctx.connection().mouse_down(1)?;
    window.mouse_move(100, 145)?;
    ctx.connection().mouse_up(1)?;
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
    ctx.run(|r| {
        let window = r.base_mut().add_child::<Window>(module_path!().into());
        window.base_mut().add_child::<Label>("before".into());
        let content = window
            .base_mut()
            .add_child::<ScrollArea>(())
            .set_content::<Column>(())
            .set_padding_enabled(false);

        for i in 0..20 {
            content
                .base_mut()
                .add_child::<Label>(format!("text item {i}"));
        }
        window.base_mut().add_child::<Label>("after".into());
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
