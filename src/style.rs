use std::fmt::Debug;

use anyhow::Result;
use lightningcss::{
    properties::Property, rules::CssRule, selector::Selector, stylesheet::StyleSheet,
};
use serde::{Deserialize, Serialize};
use std::hash::Hash;

use crate::{
    style::css::replace_vars,
    types::{LogicalPixels, Point},
};

pub mod button;
pub mod computed;
mod css;
pub mod defaults;
pub mod text_input;

pub trait ElementState: Eq + Hash + Sized {
    fn all() -> Vec<Self>;
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RelativeOffset {
    // from 0 to 1
    pub x: f32,
    pub y: f32,
}

impl RelativeOffset {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Padding {
    pub x: LogicalPixels,
    pub y: LogicalPixels,
}

impl Padding {
    pub fn new(x: LogicalPixels, y: LogicalPixels) -> Self {
        Self { x, y }
    }
    pub fn to_physical(self, scale: f32) -> Point {
        Point {
            x: self.x.to_physical(scale).get(),
            y: self.y.to_physical(scale).get(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontStyle {
    pub font_size: LogicalPixels,
    pub line_height: LogicalPixels,
    // TODO: font family, attributes, etc.
}

impl FontStyle {
    pub fn to_metrics(&self, scale: f32) -> cosmic_text::Metrics {
        cosmic_text::Metrics {
            font_size: self.font_size.to_physical(scale).get() as f32,
            line_height: self.line_height.to_physical(scale).get() as f32,
        }
    }
}

pub struct Style {
    pub css: StyleSheet<'static, 'static>,
    // TODO: allow to bundle style and referenced files
}

impl Style {
    pub fn load(css: &str) -> Result<Style> {
        let mut style = StyleSheet::parse(css, Default::default()).unwrap();
        replace_vars(&mut style);
        let code = style.to_css(Default::default()).unwrap().code;
        let style = StyleSheet::parse(&code, Default::default()).unwrap();
        // There is no StyleSheet::into_owned, so we have to use serialization :(
        let serialized = serde_value::to_value(&style)?;

        println!("{style:#?}");
        Ok(Style {
            css: serialized.deserialize_into()?,
        })
    }

    // TODO: use specificality and !important to sort from low to high priority
    pub fn find_rules(
        &self,
        check_selector: impl Fn(&Selector) -> bool,
    ) -> Vec<&Property<'static>> {
        let mut results = Vec::new();
        for rule in &self.css.rules.0 {
            if let CssRule::Style(rule) = rule {
                for selector in &rule.selectors.0 {
                    if check_selector(selector) {
                        results.extend(rule.declarations.iter().map(|(dec, _important)| dec));
                    }
                }
            }
        }
        results
    }
}
