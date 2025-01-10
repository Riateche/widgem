pub mod action;
pub mod edit;
pub mod editor;

use {
    cosmic_text::{Attrs, AttrsOwned, Buffer, BufferLine},
    derive_more::{From, Into},
    itertools::Itertools,
    std::{borrow::Cow, ops::Range},
};

pub fn text_without_preedit(buffer: &Buffer) -> String {
    buffer
        .lines
        .iter()
        .map(|line| line_text_without_preedit(line))
        .join("\n")
}

/// Get current text, excluding preedit (if any)
fn line_text_without_preedit(line: &BufferLine) -> Cow<'_, str> {
    if let Some(range) = preedit_range(line) {
        let mut text = line.text()[..range.start].to_string();
        text.push_str(&line.text()[range.end..]);
        text.into()
    } else {
        line.text().into()
    }
}

/// Returns the range of byte indices of the text corresponding
/// to the preedit
fn preedit_range(line: &BufferLine) -> Option<Range<usize>> {
    line.attrs_list()
        .spans()
        .iter()
        .find(|(_, attrs)| attrs.is_preedit())
        .map(|(range, _)| (*range).clone())
}

/// Get current preedit text
pub fn preedit_text(line: &BufferLine) -> Option<&str> {
    let range = preedit_range(line)?;
    Some(&line.text()[range])
}

pub trait AttrsExt {
    fn is_preedit(&self) -> bool;
    fn preedit(self, preedit: bool) -> Self;
}

impl AttrsExt for AttrsOwned {
    fn is_preedit(&self) -> bool {
        Metadata(self.metadata).is_preedit()
    }

    fn preedit(mut self, preedit: bool) -> Self {
        self.metadata = Metadata(self.metadata).with_preedit(preedit).into();
        self
    }
}

impl AttrsExt for Attrs<'_> {
    fn is_preedit(&self) -> bool {
        self.metadata & 0x1 != 0
    }

    fn preedit(mut self, preedit: bool) -> Self {
        self.metadata = Metadata(self.metadata).with_preedit(preedit).into();
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, From, Into)]
pub struct Metadata(pub usize);

impl Metadata {
    pub fn is_preedit(self) -> bool {
        self.0 & 0x1 != 0
    }

    pub fn with_preedit(mut self, preedit: bool) -> Self {
        self.0 = (self.0 & !1) | (preedit as usize);
        self
    }
}
