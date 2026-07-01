// 文件路径：crate::frontend::ast::printer

use std::fmt::Write;

// ---------------------------------------------------------------------------
// 树形打印核心 trait 和辅助函数
// ---------------------------------------------------------------------------

/// 一切 AST 节点可实现的格式化打印接口。
pub trait AstPrint {
    /// 递归打印当前节点。
    ///
    /// - `prefix`: 当前行之前的前缀字符串（由祖先节点的分支状态决定）。
    /// - `is_last`: 当前节点在其兄弟节点中是否为最后一个，决定使用 `└──` 还是 `├──`。
    /// - `output`: 可写的字符串缓冲区。
    fn print(&self, prefix: &str, is_last: bool, output: &mut impl Write) -> std::fmt::Result;
}

/// 根据是否最后一个兄弟生成分支符号。
///
/// 例如:
/// - `branch(true)`  -> `"└── "`
/// - `branch(false)` -> `"├── "`
pub fn branch(is_last: bool) -> &'static str {
    if is_last { "└── " } else { "├── " }
}

/// 计算下一级递归的前缀。
///
/// - 当前节点是最后一个兄弟时，其后子节点前缀仅追加空格 `"    "`。
/// - 否则追加竖线 `"│   "`，以保持与右侧兄弟的连线。
pub fn next_prefix(prefix: &str, is_last: bool) -> String {
    if is_last {
        format!("{}    ", prefix)
    } else {
        format!("{}│   ", prefix)
    }
}
