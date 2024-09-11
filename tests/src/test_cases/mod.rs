use strum::{EnumIter, EnumString, IntoStaticStr};

use crate::context::Context;
use salvation::{widgets::WidgetExt, App};

pub mod scroll_bar;
pub mod text_input;

macro_rules! tests {
    ($($name:ident,)*) => {
        #[derive(Debug, Clone, Copy, EnumString, EnumIter, IntoStaticStr)]
        #[allow(non_camel_case_types)]
        pub enum TestCase {
            $($name,)*
        }

        pub fn run_test_case(app: App, test_case: TestCase) -> anyhow::Result<()> {
            match test_case {
                $(
                    TestCase::$name => app.run(|| $name::RootWidget::new().boxed()),
                )*
            }
        }

        pub fn run_test_check(ctx: &mut Context, test_case: TestCase) -> anyhow::Result<()> {
            match test_case {
                $(
                    TestCase::$name => $name::check(ctx),
                )*
            }
        }
    }
}

tests! {
    scroll_bar,
    text_input,
}
