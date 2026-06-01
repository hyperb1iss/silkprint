//! Width-independent document model for terminal rendering.
//!
//! The terminal walker lowers a comrak AST into a [`RenderedDoc`]: a tree of
//! blocks whose inline runs carry semantic [`Role`]s and [`Mods`], **not**
//! resolved colors. Concrete colors are applied later by the
//! [`ContentStyleResolver`](super::style::ContentStyleResolver) when the active
//! theme and terminal capabilities are known. Storing roles keeps the model
//! theme-independent, so a live theme switch only re-resolves styles instead of
//! re-walking the source.

use crate::render::origin::DocumentOrigin;

/// Index into [`RenderedDoc::links`].
pub type LinkId = usize;

/// 24-bit color. Resolved styles use this; the model itself does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb(pub u8, pub u8, pub u8);

/// Inline text decorations, accumulated through nested inline nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct Mods {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub dim: bool,
}

impl Mods {
    #[must_use]
    pub fn with_bold(mut self) -> Self {
        self.bold = true;
        self
    }
    #[must_use]
    pub fn with_italic(mut self) -> Self {
        self.italic = true;
        self
    }
    #[must_use]
    pub fn with_underline(mut self) -> Self {
        self.underline = true;
        self
    }
    #[must_use]
    pub fn with_strikethrough(mut self) -> Self {
        self.strikethrough = true;
        self
    }
    #[must_use]
    pub fn with_dim(mut self) -> Self {
        self.dim = true;
        self
    }
}

/// Semantic role of an inline run. Selects the foreground color slot when the
/// run is resolved against a theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Default body text.
    Body,
    /// Heading text at the given level (1-6).
    Heading(u8),
    /// Hyperlink text.
    Link,
    /// Inline `code`.
    InlineCode,
    /// `==highlighted==` text.
    Highlight,
    /// Block-quote text.
    Quote,
    /// Inline or display math source.
    Math,
    /// Secondary text: captions, metadata labels, footnote markers.
    Muted,
    /// A syntax-highlighted code token, classified from syntect scopes.
    Syntax(SyntaxRole),
}

/// The 16 syntax token classes (plus default text) shared with the theme's
/// `[syntax]` section. Mirrors `theme::syntax::TOKEN_SCOPE_MAP`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxRole {
    Text,
    Keyword,
    String,
    Number,
    Function,
    Type,
    Comment,
    Constant,
    Boolean,
    Operator,
    Property,
    Tag,
    Attribute,
    Variable,
    Builtin,
    Punctuation,
    Escape,
}

/// A styled run of text. The atom of inline content.
#[derive(Debug, Clone)]
pub struct Span {
    pub text: String,
    pub role: Role,
    pub mods: Mods,
    pub link: Option<LinkId>,
}

impl Span {
    pub fn new(text: impl Into<String>, role: Role, mods: Mods) -> Self {
        Self {
            text: text.into(),
            role,
            mods,
            link: None,
        }
    }

    pub fn body(text: impl Into<String>) -> Self {
        Self::new(text, Role::Body, Mods::default())
    }
}

/// Column alignment for a table cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
    None,
}

/// GitHub-style alert (admonition) kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertKind {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

/// The marker that precedes a list item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemMarker {
    Bullet,
    Ordered(usize),
    Task(bool),
}

/// A single list item: its marker plus nested block content.
#[derive(Debug, Clone)]
pub struct ListItem {
    pub marker: ItemMarker,
    pub blocks: Vec<Block>,
}

/// A list (ordered, unordered, or task list).
#[derive(Debug, Clone)]
pub struct ListBlock {
    pub ordered: bool,
    pub tight: bool,
    pub items: Vec<ListItem>,
}

/// A table: alignments, a header row of cells, and data rows.
#[derive(Debug, Clone)]
pub struct TableBlock {
    pub aligns: Vec<Align>,
    pub header: Vec<Vec<Span>>,
    pub rows: Vec<Vec<Vec<Span>>>,
}

/// A block-level element.
#[derive(Debug, Clone)]
pub enum Block {
    Heading {
        level: u8,
        spans: Vec<Span>,
        anchor: String,
    },
    Paragraph(Vec<Span>),
    /// A fenced code block. `lines` is pre-highlighted: one `Vec<Span>` per
    /// source line, each span tagged with a [`SyntaxRole`].
    CodeBlock {
        lang: Option<String>,
        lines: Vec<Vec<Span>>,
    },
    Quote(Vec<Block>),
    /// Center-aligned content (from HTML `align="center"`, `<center>`, or an
    /// inline `text-align:center` style). Each rendered line of the contained
    /// blocks is horizontally centered within the available width.
    Center(Vec<Block>),
    List(ListBlock),
    Table(TableBlock),
    Alert {
        kind: AlertKind,
        title: String,
        body: Vec<Block>,
    },
    Image {
        src: String,
        alt: String,
    },
    Rule,
    Math {
        source: String,
        display: bool,
    },
    /// Term/definition pairs.
    DescriptionList(Vec<DescriptionItem>),
    /// A run of `**Label:** value` metadata lines kept as hard-broken lines.
    FieldStack(Vec<Vec<Span>>),
}

/// One term/definition entry in a description list.
#[derive(Debug, Clone)]
pub struct DescriptionItem {
    pub term: Vec<Span>,
    pub details: Vec<Block>,
}

/// Where a link points.
#[derive(Debug, Clone)]
pub enum LinkTarget {
    Url(String),
    /// In-document heading anchor.
    Anchor(String),
}

/// An entry in the document outline (table of contents).
#[derive(Debug, Clone)]
pub struct OutlineItem {
    pub level: u8,
    pub title: String,
    pub anchor: String,
    /// Index into [`RenderedDoc::blocks`] of the heading.
    pub block_index: usize,
}

/// A fully-walked document, ready to render to ANSI (one-shot) or lay out for
/// the TUI. Width-independent: no wrapping or screen coordinates here.
#[derive(Debug, Clone, Default)]
pub struct RenderedDoc {
    pub blocks: Vec<Block>,
    pub outline: Vec<OutlineItem>,
    pub links: Vec<LinkTarget>,
    pub title: Option<String>,
    pub origin: Option<DocumentOrigin>,
}

impl RenderedDoc {
    /// Register a link target, returning its id.
    pub fn add_link(&mut self, target: LinkTarget) -> LinkId {
        let id = self.links.len();
        self.links.push(target);
        id
    }
}
