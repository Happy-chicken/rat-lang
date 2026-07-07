pub mod expr;
pub mod item;
pub mod printer;
pub mod stmt;
pub mod typ;

use item::ItemNode;
use printer::{AstPrint, next_prefix};
use std::fmt::Write;

#[derive(Debug)]
pub struct Program {
    pub items: Vec<ItemNode>,
}

// ---- Program ----
impl AstPrint for Program {
    fn print(&self, prefix: &str, is_last: bool, output: &mut impl Write) -> std::fmt::Result {
        // let branch_str = branch(is_last);
        writeln!(output, "{}Program", prefix)?;
        let child = next_prefix(prefix, is_last);
        let count = self.items.len();
        for (i, item_node) in self.items.iter().enumerate() {
            item_node.item.print(&child, i == count - 1, output)?;
        }
        Ok(())
    }
}
