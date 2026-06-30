// common/error.rs
use crate::common::span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Error,
    Warning,
    Note,
    Help,
}

#[derive(Debug, Clone)]
pub struct Label {
    pub span: Span,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub level: Level,
    pub message: String,
    pub primary_label: Option<Label>, // figure out primiary issue
    pub secondary_labels: Vec<Label>, // other related issues
    pub notes: Vec<String>,           // additional notes like help
}

#[derive(Debug)]
pub struct DiagnosticBuilder {
    diag: Diagnostic,
}

impl DiagnosticBuilder {
    pub fn new(level: Level, message: impl Into<String>) -> Self {
        Self {
            diag: Diagnostic {
                level,
                message: message.into(),
                primary_label: None,
                secondary_labels: Vec::new(),
                notes: Vec::new(),
            },
        }
    }

    // set primary label with span and optional message
    pub fn span_label(mut self, span: Span, msg: impl Into<String>) -> Self {
        self.diag.primary_label = Some(Label {
            span,
            message: Some(msg.into()),
        });
        self
    }
    // set primary label with span only
    pub fn span(mut self, span: Span) -> Self {
        self.diag.primary_label = Some(Label {
            span,
            message: None,
        });
        self
    }

    pub fn secondary_label(mut self, span: Span, msg: impl Into<String>) -> Self {
        self.diag.secondary_labels.push(Label {
            span,
            message: Some(msg.into()),
        });
        self
    }

    pub fn note(mut self, msg: impl Into<String>) -> Self {
        self.diag.notes.push(msg.into());
        self
    }
    // chain call
    pub fn build(self) -> Diagnostic {
        self.diag
    }
}

