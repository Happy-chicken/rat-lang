use std::cell::RefCell;
use std::rc::Rc;

use crate::frontend::sema_checker::{symbol_table::SymbolTable, symbol::Symbol};
use crate::frontend::type_checker::{type_ctx::TypeCtxt, typ::Type};

pub struct SemaCtxt {
    pub symbol_table:  SymbolTable,
    pub type_ctx:  TypeCtxt,
}

impl SemaCtxt {
   pub fn new() ->Self {
        let symbol_table = SymbolTable::new();
        let type_ctx = TypeCtxt::new();
        SemaCtxt { symbol_table, type_ctx }
    }

    // pub fn declare_symbol_with_fresh_type(&mut self, symbol: Symbol) -> Result<Type, Rc<RefCell<Symbol>>> {
    //     let ty = self.type_ctx.fresh_type_var();
    //     self.symbol_table.declare(symbol)?;
    //     Ok(ty)
    // }
}