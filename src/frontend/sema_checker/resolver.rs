use crate::frontend::ast::typ::Type;
// sema_checker/resolver.rs
use crate::frontend::sema_checker::pass::Pass;
use crate::frontend::sema_checker::sema_ctx::SemaCtxt;
use crate::common::DiagCtxt;
use crate::frontend::ast::{Program, item::*};
use crate::frontend::sema_checker::symbol::Symbol;
use std::collections::HashMap;
pub struct Resolver {
    // 可以添加字段来存储解析过程中需要的状态或信息
}

impl Resolver {
    pub fn new() -> Self {
        Self { }
    }
}

impl Pass for Resolver {
    fn name(&self) -> &'static str { "resolvers" }

    fn run(&mut self, program: & Program, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) -> bool {
        for item_node in program.items.iter() {
            if matches!(item_node.item, Item::Class {..}) {
                self.declare_type_item(item_node, ctx, diag);
            } else if matches!(item_node.item, Item::FunctionDecl{..} | Item::FunctionDef {..}) {
                self.declare_value_item(item_node, ctx, diag);
            }
        }
        self.check_struct_recursion(program, diag);
        !diag.has_errors() 
    }
}

fn find_duplicate_field(fields: &[Field]) -> Option<String> {
    let mut seen = std::collections::HashSet::new();
    for field in fields {
        if !seen.insert(&field.name) {
            return Some(field.name.clone());
        }
    }
    None
}

fn find_duplicate_param(params: &[Parameter]) -> Option<String> {
    let mut seen = std::collections::HashSet::new();
    for p in params {
        if !seen.insert(p.name.clone()) {
            return Some(p.name.clone());
        }
    }
    None
}

impl Resolver {
    fn declare_top_level(&mut self, symbol: Symbol, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) {
        let name = symbol.name.clone();
        let span = symbol.span;
        let kind_name = symbol.kind_name();

        match ctx.symbol_table.declare(symbol) {
            Ok(()) => {}
            Err(existing) => {
                let existing_name = existing.borrow().name.clone();
                let existing_kind_name = existing.borrow().kind_name();
                let err = diag
                                        .error(span, format!("the name `{}({})` is defined multiple times", name, kind_name))
                                        .note(format!("`{}` ({}) redefined here", existing_name, existing_kind_name))
                                        .build();
                diag.emit(err);
                // 不覆盖：保留第一次的定义，避免后续解析被第二次(可能不完整)的定义污染
            }
        }
    }

    fn declare_type_item(&mut self, item_node: & ItemNode, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) {
        let symbol = match item_node.item {
            Item::Class(Class {ref name, ref fields}) => {
                let field_types: HashMap<String, Type> = fields
                                                    .iter()
                                                    .map(|f| (f.name.clone(), f.ty.clone()))
                                                    .collect();
                if let Some(existing) = find_duplicate_field(fields) {
                    let err = diag
                                        .error(item_node.span, format!("Duplicate field name: {}", existing))
                                        .build();
                    diag.emit(err);
                }
                Symbol::new_class(name.clone(), field_types, item_node.span)
            },
            _ => return,
        };
        self.declare_top_level(symbol, ctx, diag);
    }
    fn declare_value_item(&mut self, item_node: & ItemNode, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) {
        let symbol = match item_node.item {
            Item::FunctionDecl(FunctionDecl {ref name, ref params, ref return_type}) => {
                if let Some(existing) = find_duplicate_param(params) {
                    let err = diag
                                        .error(item_node.span, format!("Duplicate parameter name: {}", existing))
                                        .build();
                    diag.emit(err);
                }
                let param_types = params.iter().map(|p| p.ty.clone()).collect();
                match return_type {
                    Some(ret_ty) => Symbol::new_function(name.clone(), param_types, ret_ty.clone(), item_node.span),
                    None => Symbol::new_function(name.clone(), param_types, Type::Void, item_node.span),
                }
                // Symbol::new_function(name.clone(), param_types, return_type.clo, item_node.span)
            },
            Item::FunctionDef(FunctionDef {ref function_header,  ..}) => {
                let FunctionDecl {name, params, return_type, ..} = function_header;
                if let Some(existing) = find_duplicate_param(params) {
                    let err = diag
                                        .error(item_node.span, format!("Duplicate parameter name: {}", existing))
                                        .build();
                    diag.emit(err);
                }
                let param_types = params.iter().map(|p| p.ty.clone()).collect();
                match return_type {
                    Some(ret_ty) => Symbol::new_function(name.clone(), param_types, ret_ty.clone(), item_node.span),
                    None => Symbol::new_function(name.clone(), param_types, Type::Void, item_node.span),
                }
            },
            _ => return,
        };
        self.declare_top_level(symbol, ctx, diag);
    }

    fn check_struct_recursion(&mut self, program: & Program, diag: &mut DiagCtxt) {
        
    }
    fn direct_struct_dependency(&mut self)->Option<String> {
        None
    }
    fn has_cycle(&mut self, program: & Program, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) {
        
    }
}