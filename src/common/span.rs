// common/span.rs
use crate::common::location::SourceFile;
use std::sync::Arc;

/// 字节偏移，类型安全。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BytePos(pub usize);

impl From<usize> for BytePos {
    fn from(value: usize) -> Self {
        BytePos(value)
    }
}

/// 源码中的连续区域。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub low: BytePos,
    pub high: BytePos,
}

impl Span {
    pub fn new(low: BytePos, high: BytePos) -> Self {
        Self { low, high }
    }

    pub fn merge(self, other: Span) -> Span {
        Span {
            low: self.low.min(other.low),
            high: self.high.max(other.high),
        }
    }
}

/// 带跨度的值。
#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}
