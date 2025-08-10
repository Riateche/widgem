use widgem::{
    widgets::{Column, Label, ScrollArea},
    Widget, WidgetExt, Window,
};
use widgem_test_kit::context::Context;

#[widgem_test_kit::test]
pub fn scroll_area(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|r| {
        let content = r
            .base_mut()
            .add_child::<Window>(module_path!().into())
            .set_padding_enabled(false)
            .base_mut()
            .add_child::<ScrollArea>(())
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
    window.close()?;
    Ok(())
}
