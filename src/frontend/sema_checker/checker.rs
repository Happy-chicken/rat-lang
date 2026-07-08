use crate::common::DiagCtxt;
use crate::common::span::Span;
use crate::frontend::ast::{Program, expr::*, item::*, stmt::*};
use crate::frontend::sema_checker::pass::Pass;
use crate::frontend::sema_checker::scope::ScopeKind;
use crate::frontend::sema_checker::sema_ctx::SemaCtxt;
use crate::frontend::sema_checker::symbol::Symbol;

pub struct SemaChecker {}

impl SemaChecker {
    pub fn new() -> Self {
        Self {}
    }
}

impl Pass for SemaChecker {
    fn name(&self) -> &'static str {
        "sema_checker"
    }

    fn run(&mut self, program: &Program, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) -> bool {
        for item_node in &program.items {
            match &item_node.item {
                Item::FunctionDef(def) => self.check_function(def, ctx, diag),
                Item::Impl(imp) => self.check_impl(imp, ctx, diag),
                Item::VarDef(global) => self.check_global_var(global, ctx, diag),
                _ => {}
            }
        }

        !diag.has_errors()
    }
}

impl SemaChecker {
    fn check_function(&mut self, def: &FunctionDef, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) {
        ctx.symbol_table.enter_scope(ScopeKind::Function);

        for param in &def.function_header.params {
            let symbol = Symbol::new_parameter(
                param.name.clone(),
                param.ty.clone(),
                false,
                Span::new(0.into(), 0.into()),
            );
            if let Err(existing) = ctx.symbol_table.declare(symbol) {
                let err = diag
                    .error(
                        Span::new(0.into(), 0.into()),
                        format!("duplicate parameter name `{}`", param.name),
                    )
                    .note(format!("`{}` already declared", existing.borrow().name))
                    .build();
                diag.emit(err);
            }
        }

        for stmt_node in &def.body.stmts {
            self.check_stmt(stmt_node, ctx, diag);
        }

        ctx.symbol_table.exit_scope();
    }

    fn check_impl(&mut self, imp: &Impl, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) {
        let dummy_span = Span::new(0.into(), 0.into());

        if ctx.symbol_table.resolve_global(&imp.class_name).is_none() {
            let err = diag
                .error(dummy_span, format!("class `{}` not found", imp.class_name))
                .build();
            diag.emit(err);
        }

        if let Some(trait_name) = &imp.trait_name {
            if ctx.symbol_table.resolve_global(trait_name).is_none() {
                let err = diag
                    .error(dummy_span, format!("trait `{}` not found", trait_name))
                    .build();
                diag.emit(err);
            }
        }

        for method in &imp.methods {
            self.check_function(method, ctx, diag);
        }
    }

    fn check_block(&mut self, block: &Block, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) {
        ctx.symbol_table.enter_scope(ScopeKind::Block);
        for stmt_node in &block.stmts {
            self.check_stmt(stmt_node, ctx, diag);
        }
        ctx.symbol_table.exit_scope();
    }

    fn check_stmt(&mut self, stmt_node: &StmtNode, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) {
        match &stmt_node.stmt {
            Stmt::VarDef { name, ty, init } => {
                let symbol = Symbol::new_variable(name.clone(), ty.clone(), false, stmt_node.span);
                if let Err(existing) = ctx.symbol_table.declare(symbol) {
                    let err = diag
                        .error(
                            stmt_node.span,
                            format!("variable `{}` already declared in this scope", name),
                        )
                        .note(format!(
                            "`{}` was previously declared here",
                            existing.borrow().name
                        ))
                        .build();
                    diag.emit(err);
                }
                if let Some(init_expr) = init {
                    self.check_expr(init_expr, ctx, diag);
                }
            }

            Stmt::If {
                condition,
                then_branch,
                elif_branch,
                else_branch,
            } => {
                self.check_expr(condition, ctx, diag);
                self.check_block(then_branch, ctx, diag);
                for (cond, block) in elif_branch {
                    self.check_expr(cond, ctx, diag);
                    self.check_block(block, ctx, diag);
                }
                if !else_branch.stmts.is_empty() {
                    self.check_block(else_branch, ctx, diag);
                }
            }

            Stmt::Loop { condition, body } => {
                ctx.symbol_table.enter_scope(ScopeKind::Loop);
                self.check_expr(condition, ctx, diag);
                self.check_block(body, ctx, diag);
                ctx.symbol_table.exit_scope();
            }

            Stmt::ExprStmt(expr) => {
                self.check_expr(expr, ctx, diag);
            }

            Stmt::Return(expr) => {
                if !ctx.symbol_table.nearest_of_kind(ScopeKind::Function) {
                    let err = diag
                        .error(stmt_node.span, "return statement outside of function")
                        .build();
                    diag.emit(err);
                }
                if let Some(ret_expr) = expr {
                    self.check_expr(ret_expr, ctx, diag);
                }
            }

            Stmt::Break => {
                if !ctx.symbol_table.nearest_of_kind(ScopeKind::Loop) {
                    let err = diag
                        .error(stmt_node.span, "break statement outside of loop")
                        .build();
                    diag.emit(err);
                }
            }

            Stmt::Continue => {
                if !ctx.symbol_table.nearest_of_kind(ScopeKind::Loop) {
                    let err = diag
                        .error(stmt_node.span, "continue statement outside of loop")
                        .build();
                    diag.emit(err);
                }
            }

            Stmt::BlockStmt(block) => {
                self.check_block(block, ctx, diag);
            }
        }
    }

    fn check_expr(&mut self, expr_node: &ExprNode, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) {
        match &expr_node.expr {
            Expr::Literal(_) => {}

            Expr::Variable(name) => {
                if ctx.symbol_table.resolve(name).is_none() {
                    let err = diag
                        .error(
                            expr_node.span,
                            format!("cannot find value `{}` in this scope", name),
                        )
                        .build();
                    diag.emit(err);
                }
            }

            Expr::Assign { target, value } => {
                self.check_expr(target, ctx, diag);
                self.check_expr(value, ctx, diag);
            }

            Expr::Binary { lhs, rhs, .. } => {
                self.check_expr(lhs, ctx, diag);
                self.check_expr(rhs, ctx, diag);
            }

            Expr::Unary { expr, .. } => {
                self.check_expr(expr, ctx, diag);
            }

            Expr::Call { callee, args } => {
                self.check_expr(callee, ctx, diag);
                if let Expr::Variable(ref name) = callee.expr {
                    if let Some(sym) = ctx.symbol_table.resolve(name) {
                        if !sym.borrow().is_callable() {
                            let err = diag
                                .error(expr_node.span, format!("`{}` is not callable", name))
                                .build();
                            diag.emit(err);
                        }
                    }
                }
                for arg in args {
                    self.check_expr(arg, ctx, diag);
                }
            }

            Expr::Member { object, .. } => {
                self.check_expr(object, ctx, diag);
            }

            Expr::Index { object, index } => {
                self.check_expr(object, ctx, diag);
                self.check_expr(index, ctx, diag);
            }

            Expr::List { elements } => {
                for elem in elements {
                    self.check_expr(elem, ctx, diag);
                }
            }

            Expr::New { cons, args } => {
                if ctx.symbol_table.resolve_global(cons).is_none() {
                    let err = diag
                        .error(expr_node.span, format!("class `{}` not found", cons))
                        .build();
                    diag.emit(err);
                }
                for arg in args {
                    self.check_expr(arg, ctx, diag);
                }
            }
        }
    }

    fn check_global_var(
        &mut self,
        global: &GlobalVar,
        ctx: &mut SemaCtxt,
        diag: &mut DiagCtxt,
    ) {
        if let Some(ref init_expr) = global.init {
            self.check_expr(init_expr, ctx, diag);
        }
    }
}
