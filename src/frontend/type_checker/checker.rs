use crate::frontend::type_checker::{
    typ::{Type, PrimType},
    inferer::TypeInferer,
    unifier::Unifier,
};
use crate::frontend::ast::{Program, item::*, stmt::*, expr::*};
use crate::frontend::ast::typ::Type as AstType;
use crate::common::{DiagCtxt, span::Span};
use crate::common::error::UnifyError;
use crate::frontend::sema_checker::{
    pass::Pass,
    sema_ctx::SemaCtxt,
    scope::ScopeKind,
    symbol::Symbol,
};

pub struct TypeChecker {
    inferer: TypeInferer,
    next_expr_id: u32,
}

impl Pass for TypeChecker {
    fn name(&self) -> &'static str {
        "type_checker"
    }

    fn run(&mut self, program: &Program, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) -> bool {
        for item_node in &program.items {
            match &item_node.item {
                Item::FunctionDef(def) => self.check_function(def, ctx, diag),
                Item::Impl(imp) => {
                    for method in &imp.methods {
                        self.check_function(method, ctx, diag);
                    }
                }
                Item::VarDef(global) => self.check_global_var(global, ctx, diag),
                Item::Class(class) => self.check_class_defaults(class, ctx, diag),
                _ => {}
            }
        }
        !diag.has_errors()
    }
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            inferer: TypeInferer::new(),
            next_expr_id: 0,
        }
    }

    fn fresh_expr_id(&mut self) -> u32 {
        let id = self.next_expr_id;
        self.next_expr_id += 1;
        id
    }

    fn check_function(
        &mut self,
        func: &FunctionDef,
        ctx: &mut SemaCtxt,
        diag: &mut DiagCtxt,
    ) {
        ctx.symbol_table.enter_scope(ScopeKind::Function);

        for param in &func.function_header.params {
            let symbol = Symbol::new_parameter(
                param.name.clone(),
                param.ty.clone(),
                false,
                Span::new(0.into(), 0.into()),
            );
            let _ = ctx.symbol_table.declare(symbol);
        }

        let declared_return = func
            .function_header
            .return_type
            .clone()
            .map(|ast_ty| crate::frontend::type_checker::inferer::ast_type_to_tc(&ast_ty))
            .unwrap_or(Type::Prim(PrimType::Void));

        for stmt_node in &func.body.stmts {
            self.check_stmt(&stmt_node.stmt, stmt_node.span, &declared_return, ctx, diag);
        }

        ctx.symbol_table.exit_scope();
    }

    fn check_block(
        &mut self,
        block: &Block,
        expected_return: &Type,
        ctx: &mut SemaCtxt,
        diag: &mut DiagCtxt,
    ) {
        ctx.symbol_table.enter_scope(ScopeKind::Block);
        for stmt_node in &block.stmts {
            self.check_stmt(&stmt_node.stmt, stmt_node.span, expected_return, ctx, diag);
        }
        ctx.symbol_table.exit_scope();
    }

    fn check_stmt(
        &mut self,
        stmt: &Stmt,
        span: Span,
        expected_return: &Type,
        ctx: &mut SemaCtxt,
        diag: &mut DiagCtxt,
    ) {
        match stmt {
            Stmt::VarDef { name, ty, init } => {
                let declared_ty: Option<Type> =
                    ty.as_ref().map(|ast_ty| crate::frontend::type_checker::inferer::ast_type_to_tc(ast_ty));

                let inferred = self.inferer.infer_let_binding(
                    &declared_ty,
                    init,
                    span,
                    ctx,
                    diag,
                );

                let resolved = ctx.type_ctx.resolve_type(&inferred);
                let ast_ty = Self::tc_to_ast_type(&resolved)
                    .or_else(|| ty.clone());

                let symbol = Symbol::new_variable(
                    name.clone(),
                    ast_ty,
                    false,
                    span,
                );
                let _ = ctx.symbol_table.declare(symbol);

                let id = self.fresh_expr_id();
                ctx.type_ctx.record_expr_type(id, inferred);
                match init {
                    Some(init_expr) if matches!(&init_expr.expr, Expr::List { .. }) => {
                        if let Expr::List { elements } = &init_expr.expr {
                            ctx.type_ctx.record_list_length(id, elements.len());
                        }
                    }
                    _ => {
                        if matches!(&resolved, Type::List(_)) {
                            ctx.type_ctx.record_list_length(id, 0);
                        }
                    }
                }
            }

            Stmt::If {
                condition,
                then_branch,
                elif_branch,
                else_branch,
            } => {
                let cond_ty = self.inferer.infer_expr(
                    &condition.expr,
                    condition.span,
                    ctx,
                    diag,
                );
                self.check_condition_is_bool(&cond_ty, condition.span, diag, ctx);

                self.check_block(then_branch, expected_return, ctx, diag);
                for (cond, block) in elif_branch {
                    let _ = self.inferer.infer_expr(&cond.expr, cond.span, ctx, diag);
                    self.check_block(block, expected_return, ctx, diag);
                }

                if !else_branch.stmts.is_empty() {
                    self.check_block(else_branch, expected_return, ctx, diag);
                }
            }

            Stmt::Loop { condition, body } => {
                let cond_ty = self.inferer.infer_expr(
                    &condition.expr,
                    condition.span,
                    ctx,
                    diag,
                );
                self.check_condition_is_bool(&cond_ty, condition.span, diag, ctx);

                ctx.symbol_table.enter_scope(ScopeKind::Loop);
                self.check_block(body, expected_return, ctx, diag);
                ctx.symbol_table.exit_scope();
            }

            Stmt::ExprStmt(expr) => {
                let id = self.fresh_expr_id();
                let ty = self.inferer.infer_expr(&expr.expr, expr.span, ctx, diag);
                ctx.type_ctx.record_expr_type(id, ty);
                if let Expr::List { elements } = &expr.expr {
                    ctx.type_ctx.record_list_length(id, elements.len());
                }
            }

            Stmt::Return(expr) => {
                let ret_ty = match expr {
                    Some(e) => self.inferer.infer_expr(&e.expr, e.span, ctx, diag),
                    None => Type::Prim(PrimType::Void),
                };

                let mut unifier = Unifier::new(&mut ctx.type_ctx);
                if let Err(UnifyError::Mismatch { expected, found }) =
                    unifier.unify(&ret_ty, expected_return)
                {
                    let err = diag
                        .error(
                            span,
                            format!(
                                "return type mismatch: expected {}, found {}",
                                expected.display_name(),
                                found.display_name()
                            ),
                        )
                        .build();
                    diag.emit(err);
                }
            }

            Stmt::Break | Stmt::Continue => {}

            Stmt::BlockStmt(block) => {
                self.check_block(block, expected_return, ctx, diag);
            }
        }
    }

    fn check_condition_is_bool(
        &mut self,
        ty: &Type,
        span: Span,
        diag: &mut DiagCtxt,
        ctx: &mut SemaCtxt,
    ) {
        let resolved = ctx.type_ctx.resolve_type(ty);
        if !resolved.is_bool() && resolved != Type::Error {
            let err = diag
                .error(
                    span,
                    format!(
                        "condition must be bool, got {}",
                        resolved.display_name()
                    ),
                )
                .build();
            diag.emit(err);
        }
    }

    fn tc_to_ast_type(tc: &Type) -> Option<AstType> {
        match tc {
            Type::Prim(PrimType::Int) => Some(AstType::Int),
            Type::Prim(PrimType::Float) => Some(AstType::Float),
            Type::Prim(PrimType::Bool) => Some(AstType::Bool),
            Type::Prim(PrimType::Char) => Some(AstType::Char),
            Type::Prim(PrimType::Str) => Some(AstType::Str),
            Type::Prim(PrimType::Void) => Some(AstType::Void),
            Type::Ptr(inner) => Self::tc_to_ast_type(inner).map(|t| AstType::Ptr(Box::new(t))),
            Type::List(inner) => Self::tc_to_ast_type(inner).map(|t| AstType::List(Box::new(t))),
            Type::Class(name) => Some(AstType::Class(name.clone())),
            Type::Var(_) | Type::Func(_, _) | Type::TraitObject(_) | Type::Error => None,
        }
    }

    fn check_global_var(
        &mut self,
        global: &GlobalVar,
        ctx: &mut SemaCtxt,
        diag: &mut DiagCtxt,
    ) {
        let declared_ty: Option<Type> = global
            .ty
            .as_ref()
            .map(|ast_ty| crate::frontend::type_checker::inferer::ast_type_to_tc(ast_ty));

        let init_span = global.init.as_ref().map(|e| e.span).unwrap_or(Span::new(0.into(), 0.into()));

        let inferred = self.inferer.infer_let_binding(
            &declared_ty,
            &global.init,
            init_span,
            ctx,
            diag,
        );

        let resolved = ctx.type_ctx.resolve_type(&inferred);
        let ast_ty = Self::tc_to_ast_type(&resolved).or_else(|| global.ty.clone());

        let symbol = Symbol::new_variable(
            global.name.clone(),
            ast_ty,
            false,
            Span::new(0.into(), 0.into()),
        );
        let _ = ctx.symbol_table.declare(symbol);

        let id = self.fresh_expr_id();
        ctx.type_ctx.record_expr_type(id, inferred);
        if let Some(ref init_expr) = global.init {
            if let Expr::List { elements } = &init_expr.expr {
                ctx.type_ctx.record_list_length(id, elements.len());
            }
        } else if matches!(&resolved, Type::List(_)) {
            ctx.type_ctx.record_list_length(id, 0);
        }
    }

    fn check_class_defaults(
        &mut self,
        class: &Class,
        ctx: &mut SemaCtxt,
        diag: &mut DiagCtxt,
    ) {
        for field in &class.fields {
            if let Some(ref default_expr) = field.init {
                let field_ty = crate::frontend::type_checker::inferer::ast_type_to_tc(&field.ty);
                let inferred = self.inferer.infer_expr(
                    &default_expr.expr, default_expr.span, ctx, diag,
                );
                let mut unifier = Unifier::new(&mut ctx.type_ctx);
                if let Err(UnifyError::Mismatch { expected, found }) =
                    unifier.unify(&inferred, &field_ty)
                {
                    let err = diag.error(
                        default_expr.span,
                        format!(
                            "field `{}` default type mismatch: expected {}, found {}",
                            field.name,
                            expected.display_name(),
                            found.display_name(),
                        ),
                    ).build();
                    diag.emit(err);
                }
            }
        }
    }
}
