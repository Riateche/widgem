use crate::app_builder::AppBuilder;

pub struct App {}

impl App {
    pub fn builder() -> AppBuilder {
        AppBuilder::new()
    }
}
