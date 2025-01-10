use {cosmic_text::AttrsOwned, derive_more::From};

/// A motion to perform on a [`Cursor`]
#[derive(Clone, Copy, Debug, Eq, PartialEq, From)]
pub enum Motion {
    /// Move cursor to start of document
    DocumentStart,
    /// Move cursor to end of document
    DocumentEnd,
    Other(cosmic_text::Motion),
}

/// An action to perform on an [`Editor`]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    /// Move the cursor with some motion
    Motion {
        motion: Motion,
        select: bool,
    },
    /// Escape, clears selection
    Escape,
    /// Select text from start to end
    SelectAll,
    /// Insert character at cursor
    Insert(char),
    /// Create new line
    Enter,
    /// Delete text behind cursor
    Backspace,
    /// Delete text behind cursor to next word boundary
    DeleteStartOfWord,
    /// Delete text in front of cursor
    Delete,
    /// Delete text in front of cursor to next word boundary
    DeleteEndOfWord,
    // Indent text (typically Tab)
    Indent,
    // Unindent text (typically Shift+Tab)
    Unindent,
    /// Mouse click at specified position
    Click {
        x: i32,
        y: i32,
        select: bool,
    },
    /// Mouse double click at specified position
    DoubleClick {
        x: i32,
        y: i32,
    },
    /// Mouse triple click at specified position
    TripleClick {
        x: i32,
        y: i32,
    },
    /// Mouse drag to specified position
    Drag {
        x: i32,
        y: i32,
    },
    /// Scroll specified number of lines
    Scroll {
        lines: i32,
    },
    /// Set preedit text, replacing any previous preedit text
    ///
    /// If `cursor` is specified, it contains a start and end cursor byte positions
    /// within the preedit. If no cursor is specified for a non-empty preedit,
    /// the cursor should be hidden.
    ///
    /// If `attrs` is specified, these attributes will be assigned to the preedit's span.
    /// However, regardless of `attrs` setting, the preedit's span will always have
    /// `is_preedit` set to `true`.
    SetPreedit {
        preedit: String,
        cursor: Option<(usize, usize)>,
        attrs: Option<AttrsOwned>,
    },
}
