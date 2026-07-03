pub mod error;
pub mod location;
pub mod span;

use crate::common::error::{Diagnostic, DiagnosticBuilder, Level};
use crate::common::location::SourceFile;
use std::collections::HashMap;
use std::io::{self, Write};

pub struct DiagCtxt {
    files: HashMap<String, SourceFile>,
    diagnostics: Vec<Diagnostic>,
    error_count: usize,
}

impl DiagCtxt {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            diagnostics: Vec::new(),
            error_count: 0,
        }
    }

    pub fn add_file(&mut self, file: SourceFile) {
        self.files.insert(file.name.clone(), file);
    }

    pub fn emit(&mut self, diag: Diagnostic) {
        if diag.level == Level::Error {
            self.error_count += 1;
        }
        self.diagnostics.push(diag);
    }

    /// 快捷创建错误并直接提交（常用于解析阶段）。
    pub fn error(
        &mut self,
        span: crate::common::span::Span,
        msg: impl Into<String>,
    ) -> DiagnosticBuilder {
        // 返回 builder，用户附加信息后手动调用 build 并 emit
        DiagnosticBuilder::new(Level::Error, msg).span(span)
    }

    pub fn warn(&mut self, msg: impl Into<String>) -> DiagnosticBuilder {
        DiagnosticBuilder::new(Level::Warning, msg)
    }

    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    /// 打印所有诊断信息（简单彩色输出可以使用 `termcolor` 库，此处用纯文本）。
    pub fn print_all(&self, writer: &mut impl Write) -> io::Result<()> {
        for diag in &self.diagnostics {
            self.print_diagnostic(writer, diag)?;
        }
        Ok(())
    }

    fn print_diagnostic(&self, writer: &mut impl Write, diag: &Diagnostic) -> io::Result<()> {
        let level_str = match diag.level {
            Level::Error => "error",
            Level::Warning => "warning",
            Level::Note => "note",
            Level::Help => "help",
        };

        // 获取主标签的位置
        let (file, loc_lo, loc_hi) = if let Some(ref primary) = diag.primary_label {
            // TODO: 改进文件查找（用 span 中的 file_id）
            let (name, file) = self.files.iter().next().unwrap();
            let loc_lo = file.lookup_pos(primary.span.low);
            let loc_hi = file.lookup_pos(primary.span.high);
            (file, loc_lo, loc_hi)
        } else {
            writeln!(writer, "{}: {}", level_str, diag.message)?;
            return Ok(());
        };

        // 主消息
        writeln!(writer, "{}: {}", level_str, diag.message)?;
        // 位置（1‑based 行和列）
        writeln!(
            writer,
            "  --> {}:{}:{}",
            file.name,
            loc_lo.line,
            loc_lo.col + 1
        )?;

        // 打印源码行
        if let Some(line) = file.get_line(loc_lo.line) {
            writeln!(writer, "   |")?;
            writeln!(writer, "   | {}", line)?;

            // 计算下划线
            let start = loc_lo.col; // 0‑based
            let end = if loc_lo.line == loc_hi.line {
                loc_hi.col
            } else {
                line.len()
            };
            let carets = "^".repeat(end.saturating_sub(start));
            let padding = " ".repeat(start);
            // 关键：只加一次 padding，underline 内部不要再包含空格
            writeln!(writer, "   | {}{}", padding, carets)?;
        }

        for note in &diag.notes {
            writeln!(writer, "   = note: {}", note)?;
        }

        Ok(())
    }
}
