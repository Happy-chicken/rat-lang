use std::rc::Rc;
use crate::frontend::ast::Program;
use crate::frontend::ast::item::*;
use crate::frontend::ast::typ::Type as AstType;
use crate::frontend::sema_checker::symbol_table;
use crate::frontend::type_checker::typ::PrimType;
use super::symbol::{Symbol, SymbolKind};
use super::symbol_table::SymbolTable;
use super::scope::ScopeKind;
use crate::common::error::{ ResolveError, ResolveResult};
use crate::common::DiagCtxt;
use crate::common::span::Span;
use crate::frontend::type_checker::typ::Type as TypeCheckerType;

pub struct Resolver<'a, 'diag> {
    table: &'a mut SymbolTable,
    diag: &'diag mut DiagCtxt,
}

impl<'a, 'diag> Resolver<'a, 'diag> {
    pub fn new(table: &'a mut SymbolTable, diag: &'diag mut DiagCtxt) -> Self {
        Self { table, diag }
    }

    /// 入口：对整个模块跑第一趟收集
    pub fn resolve(&mut self, program: &Program) -> ResolveResult<()> {
        for item in &program.items {
            self.resolve_item(item)?;
        }
        Ok(())
    }

    fn resolve_item(&mut self, item: &Item) -> ResolveResult<()> {
        match item {
            Item::FunctionDef(func) => self.resolve_function(func),
            Item::FunctionDecl(func_decl) => self.resolve_function_decl(func_decl),
            Item::Class(strct) => self.resolve_class(strct),
            Item::Trait(trt) => self.resolve_trait(trt),
            Item::Impl(imple) => self.resolve_impl(imple),
            _ => Ok(()), // 其他类型暂不处理
        }
    }

    fn resolve_function(&mut self, func: &FunctionDef) -> ResolveResult<()> {
        let header = &func.function_header; 
        let params = &header.params;
        let func_name = &header.name;
        let return_type = self.resolve_primary_type(&header.return_type)?;
        Ok(())
    }

    fn resolve_function_decl(&mut self, func_decl: &FunctionDecl) -> ResolveResult<()> {
        Ok(())
    }

    fn resolve_class(&mut self, strct: &Class) -> ResolveResult<()> {
        Ok(())
    }

    fn resolve_trait(&mut self, trt: &Trait) -> ResolveResult<()> {
        Ok(())
    }

    fn resolve_impl(&mut self, imple: &Impl) -> ResolveResult<()> {
        Ok(())
    }

    fn resolve_primary_type(&mut self, typ: &Option<AstType>) -> ResolveResult<TypeCheckerType> {
        match typ {
            Some(_ty) => {
                // TODO: 根据 _ty 解析出 TypeCheckerType
                
                    
                }
            }
            None => Ok(TypeCheckerType::Prim(PrimType::Void)), // 无分号
        }
} 
