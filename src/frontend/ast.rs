pub mod expr;
pub mod item;
pub mod stmt;
pub mod typ;

use item::Item;

#[derive(Debug)]
pub struct Program {
    pub items: Vec<Item>,
}
