use {
    widgem::{
        layout::Layout,
        types::Axis,
        widgets::{Button, Label, RootWidget, ScrollBar, TextInput},
        WidgetExt, Window,
    },
    widgem_tester::Context,
};

fn init_form(root: &mut RootWidget) -> anyhow::Result<()> {
    let mut contents = root
        .set_main_content(Window::init(module_path!().into()))?
        .set_layout(Layout::ExplicitGrid)
        .contents_mut();

    let mut current_cell_y = 0;
    let user_name_label_id = contents
        .set_next_item(Label::init("User name:".into()))?
        .set_grid_cell(0, current_cell_y)
        .id();
    contents
        .set_next_item(TextInput::init())?
        .set_grid_cell(1, current_cell_y)
        .set_labelled_by(user_name_label_id.raw());

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
}

#[widgem_tester::test]
fn main(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(init_form)?;

    let window = ctx.wait_for_window_by_pid()?;
    ctx.set_blinking_expected(true);
    window.snapshot("form")?;
    window.close()?;
    Ok(())
}

#[cfg(target_os = "windows")]
mod windows {
    use {
        super::*,
        anyhow::{ensure, Context as _},
        cadd::prelude::IntoType,
        std::{thread::sleep, time::Duration},
        uiautomation::{
            controls::ControlType,
            patterns::{UIValuePattern, UIWindowPattern},
            types::{TreeScope, UIProperty},
            UIAutomation, UIElement, UITreeWalker,
        },
    };

    #[widgem_tester::test]
    fn windows_accessibility(ctx: &mut Context) -> anyhow::Result<()> {
        ctx.run(init_form)?;

        let window = ctx.wait_for_window_by_pid()?;
        ctx.set_blinking_expected(true);
        window.snapshot("form")?;
        sleep(Duration::from_secs(1));

        let automation = UIAutomation::new()?;
        let root = automation.get_root_element()?;
        let pid_condition = automation.create_property_condition(
            UIProperty::ProcessId,
            (ctx.pid()? as i32).into(),
            None,
        )?;
        let uia_window = root
            .find_first(TreeScope::Children, &pid_condition)
            .context("failed to find window by pid")?;

        ensure!(uia_window.get_name()? == "widgem_tests::simple_form");
        println!(
            "window get_bounding_rectangle {:?}",
            uia_window.get_bounding_rectangle()?
        );
        let walker = automation.get_control_view_walker()?;
        let title_bar = walker.get_first_child(&uia_window)?;
        ensure!(title_bar.get_control_type()? == ControlType::TitleBar);

        let user_name = walker.get_next_sibling(&title_bar)?;
        ensure!(user_name.get_control_type()? == ControlType::Edit);
        ensure!(user_name.get_name()? == "User name:");
        let user_name_value = user_name.get_pattern::<UIValuePattern>()?;
        ensure!(user_name_value.get_value()? == "");
        ensure!(
            user_name
                .get_property_value(UIProperty::HasKeyboardFocus)?
                .try_into_type::<bool>()?
                == true
        );

        user_name_value.set_value("Hello")?;
        window.snapshot("input hello")?;
        ensure!(user_name_value.get_value()? == "Hello");

        let element1 = walker.get_next_sibling(&user_name)?;
        println!("element1 {:?}", element1);
        println!("get_name {:?}", element1.get_name()?);
        println!("get_classname {:?}", element1.get_classname()?);
        println!(
            "get_bounding_rectangle {:?}",
            element1.get_bounding_rectangle()?
        );

        uia_window.get_pattern::<UIWindowPattern>()?.close()?;
        // print_element(&walker, &root, 0)?;

        Ok(())
    }

    #[allow(dead_code)]
    fn print_element(
        walker: &UITreeWalker,
        element: &UIElement,
        level: usize,
    ) -> anyhow::Result<()> {
        for _ in 0..level {
            print!(" ")
        }
        println!(
            "{} - {} [pid={}]",
            element.get_classname()?,
            element.get_name()?,
            element.get_process_id()?,
        );

        if level < 1 {
            if let Ok(child) = walker.get_first_child(&element) {
                print_element(walker, &child, level + 1)?;

                let mut next = child;
                while let Ok(sibling) = walker.get_next_sibling(&next) {
                    print_element(walker, &sibling, level + 1)?;

                    next = sibling;
                }
            }
        }

        Ok(())
    }
}
