use crate::frontend::ast::expr::*;
use crate::common::{DiagCtxt, span::Span};
use crate::frontend::sema_checker::sema_ctx::SemaCtxt;
use crate::frontend::sema_checker::symbol::SymbolKind;
use crate::frontend::type_checker::typ::{Type, PrimType};
use crate::frontend::type_checker::unifier::Unifier;
use crate::common::error::UnifyError;

pub struct TypeInferer;

impl TypeInferer {
    pub fn new() -> Self {
        TypeInferer
    }

    pub fn infer_expr(
        &mut self,
        expr: &Expr,
        span: Span,
        ctx: &mut SemaCtxt,
        diag: &mut DiagCtxt,
    ) -> Type {
        match expr {
            Expr::Literal(lit) => self.infer_literal(lit),
            Expr::Variable(name) => self.infer_ident(name, span, ctx, diag),
            Expr::Assign { target, value } => self.infer_assign(target, value, span, ctx, diag),
            Expr::Binary { op, lhs, rhs } => self.infer_binary(op, lhs, rhs, span, ctx, diag),
            Expr::Unary { op, expr: inner } => self.infer_unary(op, inner, span, ctx, diag),
            Expr::Call { callee, args } => self.infer_call(callee, args, span, ctx, diag),
            Expr::Member { object, field } => self.infer_field_access(object, field, span, ctx, diag),
            Expr::Index { object, index } => self.infer_index(object, index, span, ctx, diag),
            Expr::List { elements } => {
                self.infer_list_literal(elements, span, ctx, diag)
            }
        }
    }

    pub fn infer_let_binding(
        &mut self,
        declared_ty: &Option<Type>,
        init: &Option<ExprNode>,
        span: Span,
        ctx: &mut SemaCtxt,
        diag: &mut DiagCtxt,
    ) -> Type {
        let inferred = if let Some(init_expr) = init {
            self.infer_expr(&init_expr.expr, init_expr.span, ctx, diag)
        } else {
            ctx.type_ctx.fresh_type_var()
        };

        match declared_ty {
            Some(decl_ty) => {
                let mut unifier = Unifier::new(&mut ctx.type_ctx);
                match unifier.unify(&inferred, decl_ty) {
                    Ok(ty) => ty,
                    Err(UnifyError::Mismatch { expected, found }) => {
                        let err = diag
                            .error(
                                span,
                                format!(
                                    "type mismatch: expected {}, found {}",
                                    expected.display_name(),
                                    found.display_name()
                                ),
                            )
                            .build();
                        diag.emit(err);
                        Type::Error
                    }
                    Err(UnifyError::InfiniteType { .. }) => {
                        let err = diag
                            .error(span, "recursive type in variable binding")
                            .build();
                        diag.emit(err);
                        Type::Error
                    }
                }
            }
            None => inferred,
        }
    }

    // --- private helpers ---

    fn infer_literal(&mut self, lit: &Literal) -> Type {
        match lit {
            Literal::Int(_) => Type::Prim(PrimType::Int),
            Literal::Bool(_) => Type::Prim(PrimType::Bool),
            Literal::Float(_) => Type::Prim(PrimType::Float),
            Literal::Char(_) => Type::Prim(PrimType::Char),
            Literal::StringLiteral(_) => Type::Prim(PrimType::Str),
        }
    }

    fn infer_ident(
        &mut self,
        name: &str,
        span: Span,
        ctx: &SemaCtxt,
        diag: &mut DiagCtxt,
    ) -> Type {
        if let Some(sym) = ctx.symbol_table.resolve(name) {
            let s = sym.borrow();
            return match &s.kind {
                crate::frontend::sema_checker::symbol::SymbolKind::Variable { .. }
                | crate::frontend::sema_checker::symbol::SymbolKind::Parameter { .. } => {
                    s.ty.clone().map_or(Type::Error, |ast_ty| ast_type_to_tc(&ast_ty))
                }
                crate::frontend::sema_checker::symbol::SymbolKind::Function {
                    params,
                    return_type,
                } => {
                    let p: Vec<Type> = params.iter().map(ast_type_to_tc).collect();
                    let r = Box::new(ast_type_to_tc(return_type));
                    Type::Func(p, r)
                }
                crate::frontend::sema_checker::symbol::SymbolKind::Class { .. } => {
                    Type::Class(name.to_string())
                }
                _ => Type::Error,
            }
        } else {
            Type::Error
        }
    }

    fn infer_assign(
        &mut self,
        target: &ExprNode,
        value: &ExprNode,
        span: Span,
        ctx: &mut SemaCtxt,
        diag: &mut DiagCtxt,
    ) -> Type {
        let lhs_ty = self.infer_expr(&target.expr, target.span, ctx, diag);
        let rhs_ty = self.infer_expr(&value.expr, value.span, ctx, diag);

        let mut unifier = Unifier::new(&mut ctx.type_ctx);
        match unifier.unify(&lhs_ty, &rhs_ty) {
            Ok(ty) => ty,
            Err(_) => {
                let err = diag
                    .error(
                        span,
                        format!(
                            "type mismatch in assignment: {} and {}",
                            lhs_ty.display_name(),
                            rhs_ty.display_name()
                        ),
                    )
                    .build();
                diag.emit(err);
                Type::Error
            }
        }
    }

    fn infer_binary(
        &mut self,
        op: &BinaryOp,
        lhs: &ExprNode,
        rhs: &ExprNode,
        span: Span,
        ctx: &mut SemaCtxt,
        diag: &mut DiagCtxt,
    ) -> Type {
        let lhs_ty = self.infer_expr(&lhs.expr, lhs.span, ctx, diag);
        let rhs_ty = self.infer_expr(&rhs.expr, rhs.span, ctx, diag);

        match op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                let mut unifier = Unifier::new(&mut ctx.type_ctx);
                if let Err(_) = unifier.unify(&lhs_ty, &rhs_ty) {
                    let err = diag
                        .error(
                            span,
                            format!(
                                "arithmetic on mismatched types: {} and {}",
                                lhs_ty.display_name(),
                                rhs_ty.display_name()
                            ),
                        )
                        .build();
                    diag.emit(err);
                    return Type::Error;
                }

                let resolved = ctx.type_ctx.resolve_type(&lhs_ty);
                if !resolved.is_numeric() && resolved != Type::Error {
                    let err = diag
                        .error(
                            span,
                            format!(
                                "arithmetic requires numeric types, got {}",
                                resolved.display_name()
                            ),
                        )
                        .build();
                    diag.emit(err);
                    return Type::Error;
                }
                resolved
            }

            BinaryOp::Eq | BinaryOp::NotEq | BinaryOp::Lt | BinaryOp::Gt
            | BinaryOp::Le | BinaryOp::Ge => {
                let mut unifier = Unifier::new(&mut ctx.type_ctx);
                let _ = unifier.unify(&lhs_ty, &rhs_ty);
                Type::Prim(PrimType::Bool)
            }

            BinaryOp::And | BinaryOp::Or => {
                let mut unifier = Unifier::new(&mut ctx.type_ctx);
                let _ = unifier.unify(&lhs_ty, &Type::Prim(PrimType::Bool));
                let _ = unifier.unify(&rhs_ty, &Type::Prim(PrimType::Bool));
                Type::Prim(PrimType::Bool)
            }
        }
    }

    fn infer_unary(
        &mut self,
        op: &UnaryOp,
        expr: &ExprNode,
        span: Span,
        ctx: &mut SemaCtxt,
        diag: &mut DiagCtxt,
    ) -> Type {
        let inner_ty = self.infer_expr(&expr.expr, expr.span, ctx, diag);

        match op {
            UnaryOp::Neg => {
                let resolved = ctx.type_ctx.resolve_type(&inner_ty);
                if !resolved.is_numeric() && resolved != Type::Error {
                    let err = diag
                        .error(
                            span,
                            format!(
                                "negation requires numeric type, got {}",
                                resolved.display_name()
                            ),
                        )
                        .build();
                    diag.emit(err);
                    Type::Error
                } else {
                    resolved
                }
            }
            UnaryOp::Not => {
                let mut unifier = Unifier::new(&mut ctx.type_ctx);
                let _ = unifier.unify(&inner_ty, &Type::Prim(PrimType::Bool));
                Type::Prim(PrimType::Bool)
            }
            UnaryOp::Deref => {
                let resolved = ctx.type_ctx.resolve_type(&inner_ty);
                match resolved {
                    Type::Ptr(inner) => *inner,
                    _ => {
                        let err = diag
                            .error(span, "dereference requires pointer type")
                            .build();
                        diag.emit(err);
                        Type::Error
                    }
                }
            }
            UnaryOp::AddrOf => Type::Ptr(Box::new(inner_ty)),
            UnaryOp::Inc | UnaryOp::Dec => inner_ty,
        }
    }

    fn infer_call(
        &mut self,
        callee: &ExprNode,
        args: &[ExprNode],
        span: Span,
        ctx: &mut SemaCtxt,
        diag: &mut DiagCtxt,
    ) -> Type {
        if let Expr::Member { object, field } = &callee.expr {
            let obj_ty = self.infer_expr(&object.expr, object.span, ctx, diag);
            let resolved_obj = ctx.type_ctx.resolve_type(&obj_ty);

            let mangled = if let Type::Class(ref class_name) = resolved_obj {
                format!("{}_{}", class_name, field)
            } else {
                field.clone()
            };

            if let Some(sym) = ctx.symbol_table.resolve(&mangled) {
                let s = sym.borrow();
                if let SymbolKind::Function { params, return_type } = &s.kind {
                    let mut arg_types: Vec<Type> = vec![obj_ty];
                    for a in args {
                        arg_types.push(self.infer_expr(&a.expr, a.span, ctx, diag));
                    }

                    let tc_params: Vec<Type> =
                        params.iter().map(ast_type_to_tc).collect();
                    let tc_ret = ast_type_to_tc(return_type);

                    if tc_params.len() != arg_types.len() {
                        let err = diag
                            .error(span, format!(
                                "method `{}` expects {} arguments (including self), got {}",
                                field, tc_params.len(), arg_types.len()
                            )).build();
                        diag.emit(err);
                        return Type::Error;
                    }

                    let mut unifier = Unifier::new(&mut ctx.type_ctx);
                    for (p, a) in tc_params.iter().zip(arg_types.iter()) {
                        let _ = unifier.unify(p, a);
                    }
                    return tc_ret;
                }
            }
        }

        let func_ty = self.infer_expr(&callee.expr, callee.span, ctx, diag);
        let resolved = ctx.type_ctx.resolve_type(&func_ty);

        let arg_types: Vec<Type> = args
            .iter()
            .map(|a| self.infer_expr(&a.expr, a.span, ctx, diag))
            .collect();

        match resolved {
            Type::Func(param_types, ret_type) => {
                if param_types.len() != arg_types.len() {
                    let err = diag
                        .error(
                            span,
                            format!(
                                "expected {} arguments, got {}",
                                param_types.len(),
                                arg_types.len()
                            ),
                        )
                        .build();
                    diag.emit(err);
                    return Type::Error;
                }

                let mut unifier = Unifier::new(&mut ctx.type_ctx);
                for (p, a) in param_types.iter().zip(arg_types.iter()) {
                    if let Err(UnifyError::Mismatch { expected, found }) =
                        unifier.unify(p, a)
                    {
                        let err = diag
                            .error(
                                span,
                                format!(
                                    "argument type mismatch: expected {}, found {}",
                                    expected.display_name(),
                                    found.display_name()
                                ),
                            )
                            .build();
                        diag.emit(err);
                        return Type::Error;
                    }
                }
                *ret_type
            }
            Type::Class(name) => {
                if let Some(sym) = ctx.symbol_table.resolve_global(&name) {
                    let s = sym.borrow();
                    if let SymbolKind::Class { fields } = &s.kind {
                        if args.len() > fields.len() {
                            let err = diag.error(span, format!(
                                "class `{}` has {} fields but got {} arguments",
                                name, fields.len(), args.len()
                            )).build();
                            diag.emit(err);
                            return Type::Error;
                        }
                        for (i, arg_type) in arg_types.iter().enumerate() {
                            if i < fields.len() {
                                let field_ty = ast_type_to_tc(&fields[i].1);
                                let mut unifier = Unifier::new(&mut ctx.type_ctx);
                                if let Err(UnifyError::Mismatch { expected, found }) =
                                    unifier.unify(arg_type, &field_ty)
                                {
                                    let err = diag.error(span, format!(
                                        "constructor arg {} type mismatch: expected {}, found {}",
                                        i + 1, expected.display_name(), found.display_name()
                                    )).build();
                                    diag.emit(err);
                                }
                            }
                        }
                    }
                }
                Type::Class(name)
            }
            _ => {
                if func_ty != Type::Error {
                    let err = diag
                        .error(span, "called value is not a function")
                        .build();
                    diag.emit(err);
                }
                Type::Error
            }
        }
    }

    fn infer_field_access(
        &mut self,
        object: &ExprNode,
        field: &str,
        span: Span,
        ctx: &mut SemaCtxt,
        diag: &mut DiagCtxt,
    ) -> Type {
        let obj_ty = self.infer_expr(&object.expr, object.span, ctx, diag);
        let resolved = ctx.type_ctx.resolve_type(&obj_ty);

        match resolved {
            Type::Class(name) => {
                if let Some(sym) = ctx.symbol_table.resolve_global(&name) {
                    let s = sym.borrow();
                    match &s.kind {
                        crate::frontend::sema_checker::symbol::SymbolKind::Class {
                            fields,
                        } => {
                            if let Some(field_ty) = fields.iter().find(|(n, _)| n == field).map(|(_, t)| t) {
                                ast_type_to_tc(field_ty)
                            } else if ctx.symbol_table.resolve_global(field).is_some() {
                                ctx.type_ctx.fresh_type_var()
                            } else {
                                let err = diag
                                    .error(
                                        span,
                                        format!(
                                            "class `{}` has no field `{}`",
                                            name, field
                                        ),
                                    )
                                    .build();
                                diag.emit(err);
                                Type::Error
                            }
                        }
                        _ => Type::Error,
                    }
                } else {
                    Type::Error
                }
            }
            _ => {
                ctx.type_ctx.fresh_type_var()
            }
        }
    }

    fn infer_index(
        &mut self,
        object: &ExprNode,
        index: &ExprNode,
        span: Span,
        ctx: &mut SemaCtxt,
        diag: &mut DiagCtxt,
    ) -> Type {
        let obj_ty = self.infer_expr(&object.expr, object.span, ctx, diag);
        let _idx_ty = self.infer_expr(&index.expr, index.span, ctx, diag);

        let mut unifier = Unifier::new(&mut ctx.type_ctx);
        let _ = unifier.unify(&_idx_ty, &Type::Prim(PrimType::Int));

        let resolved = ctx.type_ctx.resolve_type(&obj_ty);
        match resolved {
            Type::List(elem_ty) => *elem_ty,
            Type::Prim(PrimType::Str) => Type::Prim(PrimType::Char),
            _ => {
                let err = diag
                    .error(span, "indexing requires array, list, or string")
                    .build();
                diag.emit(err);
                Type::Error
            }
        }
    }

    fn infer_list_literal(
        &mut self,
        elements: &[ExprNode],
        span: Span,
        ctx: &mut SemaCtxt,
        diag: &mut DiagCtxt,
    ) -> Type {
        if elements.is_empty() {
            return Type::List(Box::new(ctx.type_ctx.fresh_type_var()));
        }

        let first_ty = self.infer_expr(&elements[0].expr, elements[0].span, ctx, diag);

        for elem in &elements[1..] {
            let elem_ty = self.infer_expr(&elem.expr, elem.span, ctx, diag);
            let mut unifier = Unifier::new(&mut ctx.type_ctx);
            if let Err(_) = unifier.unify(&first_ty, &elem_ty) {
                let err = diag
                    .error(
                        span,
                        format!(
                            "list elements have mismatched types: {} and {}",
                            first_ty.display_name(),
                            elem_ty.display_name()
                        ),
                    )
                    .build();
                diag.emit(err);
            }
        }

        Type::List(Box::new(first_ty))
    }
}

pub fn ast_type_to_tc(ast_ty: &crate::frontend::ast::typ::Type) -> Type {
    use crate::frontend::ast::typ::Type as AstType;
    match ast_ty {
        AstType::Int => Type::Prim(PrimType::Int),
        AstType::Float => Type::Prim(PrimType::Float),
        AstType::Bool => Type::Prim(PrimType::Bool),
        AstType::Char => Type::Prim(PrimType::Char),
        AstType::Str => Type::Prim(PrimType::Str),
        AstType::Void => Type::Prim(PrimType::Void),
        AstType::Ptr(inner) => Type::Ptr(Box::new(ast_type_to_tc(inner))),
        AstType::List(inner) => Type::List(Box::new(ast_type_to_tc(inner))),
        AstType::Class(name) => Type::Class(name.clone()),
    }
}
