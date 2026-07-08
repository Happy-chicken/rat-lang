use crate::frontend::sema_checker::{symbol::Symbol, symbol_table::SymbolTable};
use crate::frontend::type_checker::{typ::Type, type_ctx::TypeCtxt};

pub struct SemaCtxt {
    pub symbol_table: SymbolTable,
    pub type_ctx: TypeCtxt,
}

impl SemaCtxt {
    pub fn new() -> Self {
        let symbol_table = SymbolTable::new();
        let type_ctx = TypeCtxt::new();
        SemaCtxt {
            symbol_table,
            type_ctx,
        }
    }

    // pub fn declare_symbol_with_fresh_type(&mut self, symbol: Symbol) -> Result<Type, Rc<RefCell<Symbol>>> {
    //     let ty = self.type_ctx.fresh_type_var();
    //     self.symbol_table.declare(symbol)?;
    //     Ok(ty)
    // }
}

