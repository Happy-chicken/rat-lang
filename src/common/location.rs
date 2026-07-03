// common/location.rs
use crate::common::span::BytePos;

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub name: String,        // file name
    pub src: String,         // source code
    line_starts: Vec<usize>, // offset of each line start (0-based)
}

impl SourceFile {
    pub fn new(name: String, src: String) -> Self {
        let line_starts = std::iter::once(0)
            .chain(src.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        Self {
            name,
            src,
            line_starts,
        }
    }

    /// 根据字节偏移计算行列号（1-based 行号，1-based 列号）。
    pub fn lookup_pos(&self, pos: BytePos) -> Location {
        let offset = pos.0;
        let line = self
            .line_starts
            .binary_search(&offset)
            .unwrap_or_else(|i| i.saturating_sub(1));
        let col = offset - self.line_starts[line];
        println!("o{},{}", offset, self.line_starts[line]);
        Location {
            line: line + 1,
            col: col,
        }
    }

    /// 获取指定行的源码（不含换行符）。
    pub fn get_line(&self, line_1based: usize) -> Option<&str> {
        let idx = line_1based.checked_sub(1)?;
        let start = *self.line_starts.get(idx)?;
        let end = self
            .line_starts
            .get(idx + 1)
            .copied()
            .unwrap_or(self.src.len());
        // 去除末尾的 \n 或 \r\n
        let line = self.src.get(start..end).unwrap_or("");
        Some(line.trim_end_matches(|c| c == '\n' || c == '\r'))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    pub line: usize,
    pub col: usize,
}
