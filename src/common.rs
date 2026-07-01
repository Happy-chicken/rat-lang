pub mod location;
pub mod error;
pub mod span;

use crate::common::location::SourceFile;
use crate::common::error::{Diagnostic, DiagnosticBuilder, Level};
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
    pub fn error(& mut self, span: crate::common::span::Span, msg: impl Into<String>) -> DiagnosticBuilder {
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

        // 假设 primary_label 所在的文件可通过某种方式获取，这里简化：遍历所有文件查找
        // 真实场景中 Span 应持有文件 ID，此处暂用硬编码或根据 primary_label 查找
        let (file, loc_lo, loc_hi) = if let Some(ref primary) = diag.primary_label {
            // 简单策略：取第一个文件（应改进）
            let (name, file) = self.files.iter().next().unwrap(); // 注意 unwrap 安全性，演示用
            let loc_lo = file.lookup_pos(primary.span.low);
            let loc_hi = file.lookup_pos(primary.span.high);
            (file, loc_lo, loc_hi)
        } else {
            // 没有 span 时，只打印消息
            writeln!(writer, "{}: {}", level_str, diag.message)?;
            return Ok(());
        };

        // 打印主消息
        writeln!(writer, "{}: {}", level_str, diag.message)?;
        // 打印位置和源码行
        writeln!(writer, "  --> {}:{}:{}", file.name, loc_lo.line, loc_lo.col)?;
        if let Some(line) = file.get_line(loc_lo.line) {
            writeln!(writer, "   |")?;
            writeln!(writer, "   | {}",  line)?;
            // 下划线标注
            let start = loc_lo.col ;
            let end = if loc_lo.line == loc_hi.line {
                loc_hi.col 
            } else {
                line.len()
            };
            let underline: String = (0..line.len())
                .map(|i| if i >= start && i < end { '^' } else { ' ' })
                .collect();
            writeln!(writer, "   | {}{}", " ".repeat(start), underline)?;
        }

        for note in &diag.notes {
            writeln!(writer, "   = note: {}", note)?;
        }

        Ok(())
    }
}