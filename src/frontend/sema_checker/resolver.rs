use crate::frontend::ast::typ::Type;
use crate::frontend::sema_checker::pass::Pass;
use crate::frontend::sema_checker::sema_ctx::SemaCtxt;
use crate::frontend::sema_checker::scope::ScopeKind;
use crate::common::DiagCtxt;
use crate::common::span::Span;
use crate::frontend::ast::{Program, item::*, stmt::*, expr::*};
use crate::frontend::sema_checker::symbol::Symbol;
use std::collections::{HashMap, HashSet};

pub struct Resolver {}

impl Resolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl Pass for Resolver {
    fn name(&self) -> &'static str {
        "resolver"
    }

    fn run(&mut self, program: &Program, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) -> bool {
        for item_node in &program.items {
            match &item_node.item {
                Item::Class(_) => self.declare_type_item(item_node, ctx, diag),
                Item::Trait(_) => self.declare_trait_item(item_node, ctx, diag),
                _ => {}
            }
        }

        for item_node in &program.items {
            match &item_node.item {
                Item::FunctionDef(_) | Item::FunctionDecl(_) => {
                    self.declare_value_item(item_node, ctx, diag)
                }
                _ => {}
            }
        }

        for item_node in &program.items {
            match &item_node.item {
                Item::FunctionDef(def) => self.resolve_function(def, ctx, diag),
                Item::Impl(imp) => self.resolve_impl(imp, ctx, diag),
                _ => {}
            }
        }

        self.check_struct_recursion(program, diag);

        !diag.has_errors()
    }
}

fn find_duplicate_field(fields: &[Field]) -> Option<String> {
    let mut seen = HashSet::new();
    for field in fields {
        if !seen.insert(&field.name) {
            return Some(field.name.clone());
        }
    }
    None
}

fn find_duplicate_param(params: &[Parameter]) -> Option<String> {
    let mut seen = HashSet::new();
    for p in params {
        if !seen.insert(&p.name) {
            return Some(p.name.clone());
        }
    }
    None
}

impl Resolver {
    fn declare_top_level(
        &mut self,
        symbol: Symbol,
        ctx: &mut SemaCtxt,
        diag: &mut DiagCtxt,
    ) {
        let name = symbol.name.clone();
        let span = symbol.span;
        let kind_name = symbol.kind_name();

        match ctx.symbol_table.declare(symbol) {
            Ok(()) => {}
            Err(existing) => {
                let existing_name = existing.borrow().name.clone();
                let existing_kind_name = existing.borrow().kind_name();
                let err = diag
                    .error(
                        span,
                        format!(
                            "the name `{}` ({}) is defined multiple times",
                            name, kind_name
                        ),
                    )
                    .note(format!(
                        "`{}` ({}) redefined here",
                        existing_name, existing_kind_name
                    ))
                    .build();
                diag.emit(err);
            }
        }
    }

    fn declare_type_item(
        &mut self,
        item_node: &ItemNode,
        ctx: &mut SemaCtxt,
        diag: &mut DiagCtxt,
    ) {
        let symbol = match &item_node.item {
            Item::Class(Class { name, fields }) => {
                if let Some(dup) = find_duplicate_field(fields) {
                    let err = diag
                        .error(item_node.span, format!("duplicate field name `{}`", dup))
                        .build();
                    diag.emit(err);
                }
                let field_types: HashMap<String, Type> = fields
                    .iter()
                    .map(|f| (f.name.clone(), f.ty.clone()))
                    .collect();
                Symbol::new_class(name.clone(), field_types, item_node.span)
            }
            _ => return,
        };
        self.declare_top_level(symbol, ctx, diag);
    }

    fn declare_trait_item(
        &mut self,
        item_node: &ItemNode,
        ctx: &mut SemaCtxt,
        diag: &mut DiagCtxt,
    ) {
        if let Item::Trait(Trait { name, methods }) = &item_node.item {
            let mut method_names = HashSet::new();
            for method in methods {
                if !method_names.insert(&method.name) {
                    let err = diag
                        .error(
                            item_node.span,
                            format!(
                                "duplicate method name `{}` in trait `{}`",
                                method.name, name
                            ),
                        )
                        .build();
                    diag.emit(err);
                }
                if let Some(dup) = find_duplicate_param(&method.params) {
                    let err = diag
                        .error(
                            item_node.span,
                            format!(
                                "duplicate parameter name `{}` in trait method `{}`",
                                dup, method.name
                            ),
                        )
                        .build();
                    diag.emit(err);
                }
            }
            let symbol = Symbol::new_trait(name.clone(), item_node.span);
            self.declare_top_level(symbol, ctx, diag);
        }
    }

    fn declare_value_item(
        &mut self,
        item_node: &ItemNode,
        ctx: &mut SemaCtxt,
        diag: &mut DiagCtxt,
    ) {
        let (name, params, return_type) = match &item_node.item {
            Item::FunctionDecl(FunctionDecl {
                name,
                params,
                return_type,
            }) => (name, params, return_type),
            Item::FunctionDef(FunctionDef {
                function_header:
                    FunctionDecl {
                        name,
                        params,
                        return_type,
                    },
                ..
            }) => (name, params, return_type),
            _ => return,
        };

        if let Some(dup) = find_duplicate_param(params) {
            let err = diag
                .error(
                    item_node.span,
                    format!("duplicate parameter name `{}`", dup),
                )
                .build();
            diag.emit(err);
        }

        let param_types: Vec<Type> = params.iter().map(|p| p.ty.clone()).collect();
        let ret_ty = return_type.clone().unwrap_or(Type::Void);
        let symbol = Symbol::new_function(name.clone(), param_types, ret_ty, item_node.span);
        self.declare_top_level(symbol, ctx, diag);
    }

    // --- Body resolution ---

    fn resolve_function(
        &mut self,
        def: &FunctionDef,
        ctx: &mut SemaCtxt,
        diag: &mut DiagCtxt,
    ) {
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
            self.resolve_stmt(stmt_node, ctx, diag);
        }

        ctx.symbol_table.exit_scope();
    }

    fn resolve_impl(
        &mut self,
        imp: &Impl,
        ctx: &mut SemaCtxt,
        diag: &mut DiagCtxt,
    ) {
        let dummy_span = Span::new(0.into(), 0.into());

        if ctx.symbol_table.resolve(&imp.class_name).is_none() {
            let err = diag
                .error(
                    dummy_span,
                    format!("class `{}` not found", imp.class_name),
                )
                .build();
            diag.emit(err);
        }

        if let Some(ref trait_name) = imp.trait_name {
            if ctx.symbol_table.resolve(trait_name).is_none() {
                let err = diag
                    .error(
                        dummy_span,
                        format!("trait `{}` not found", trait_name),
                    )
                    .build();
                diag.emit(err);
            }
        }

        for method in &imp.methods {
            self.resolve_function(method, ctx, diag);
        }
    }

    fn resolve_block(
        &mut self,
        block: &Block,
        ctx: &mut SemaCtxt,
        diag: &mut DiagCtxt,
    ) {
        ctx.symbol_table.enter_scope(ScopeKind::Block);
        for stmt_node in &block.stmts {
            self.resolve_stmt(stmt_node, ctx, diag);
        }
        ctx.symbol_table.exit_scope();
    }

    fn resolve_stmt(
        &mut self,
        stmt_node: &StmtNode,
        ctx: &mut SemaCtxt,
        diag: &mut DiagCtxt,
    ) {
        match &stmt_node.stmt {
            Stmt::VarDef { name, ty, init } => {
                let symbol = Symbol::new_variable(
                    name.clone(),
                    ty.clone(),
                    false,
                    stmt_node.span,
                );
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
                    self.resolve_expr(init_expr, ctx, diag);
                }
            }

            Stmt::If {
                condition,
                then_branch,
                elif_branch,
                else_branch,
            } => {
                self.resolve_expr(condition, ctx, diag);
                self.resolve_block(then_branch, ctx, diag);
                for (cond, block) in elif_branch {
                    self.resolve_expr(cond, ctx, diag);
                    self.resolve_block(block, ctx, diag);
                }
                if !else_branch.stmts.is_empty() {
                    self.resolve_block(else_branch, ctx, diag);
                }
            }

            Stmt::Loop { condition, body } => {
                ctx.symbol_table.enter_scope(ScopeKind::Loop);
                self.resolve_expr(condition, ctx, diag);
                self.resolve_block(body, ctx, diag);
                ctx.symbol_table.exit_scope();
            }

            Stmt::ExprStmt(expr) => {
                self.resolve_expr(expr, ctx, diag);
            }

            Stmt::Return(expr) => {
                if !ctx.symbol_table.nearest_of_kind(ScopeKind::Function) {
                    let err = diag
                        .error(stmt_node.span, "return statement outside of function")
                        .build();
                    diag.emit(err);
                }
                if let Some(ret_expr) = expr {
                    self.resolve_expr(ret_expr, ctx, diag);
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
        }
    }

    fn resolve_expr(
        &mut self,
        expr_node: &ExprNode,
        ctx: &mut SemaCtxt,
        diag: &mut DiagCtxt,
    ) {
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
                self.resolve_expr(target, ctx, diag);
                self.resolve_expr(value, ctx, diag);
            }

            Expr::Binary { lhs, rhs, .. } => {
                self.resolve_expr(lhs, ctx, diag);
                self.resolve_expr(rhs, ctx, diag);
            }

            Expr::Unary { expr, .. } => {
                self.resolve_expr(expr, ctx, diag);
            }

            Expr::Call { callee, args } => {
                self.resolve_expr(callee, ctx, diag);
                if let Expr::Variable(ref name) = callee.expr {
                    if let Some(sym) = ctx.symbol_table.resolve(name) {
                        if !sym.borrow().is_callable() {
                            let err = diag
                                .error(
                                    expr_node.span,
                                    format!("`{}` is not callable", name),
                                )
                                .build();
                            diag.emit(err);
                        }
                    }
                }
                for arg in args {
                    self.resolve_expr(arg, ctx, diag);
                }
            }

            Expr::Member { object, .. } => {
                self.resolve_expr(object, ctx, diag);
            }

            Expr::Index { object, index } => {
                self.resolve_expr(object, ctx, diag);
                self.resolve_expr(index, ctx, diag);
            }

            Expr::List { elements } => {
                for elem in elements {
                    self.resolve_expr(elem, ctx, diag);
                }
            }

            Expr::New { cons, args } => {
                if ctx.symbol_table.resolve(cons).is_none() {
                    let err = diag
                        .error(
                            expr_node.span,
                            format!("class `{}` not found", cons),
                        )
                        .build();
                    diag.emit(err);
                }
                for arg in args {
                    self.resolve_expr(arg, ctx, diag);
                }
            }
        }
    }

    // --- Struct recursion check ---

    fn check_struct_recursion(&mut self, program: &Program, diag: &mut DiagCtxt) {
        let mut deps: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut spans: HashMap<&str, Span> = HashMap::new();

        for item_node in &program.items {
            if let Item::Class(Class { name, fields }) = &item_node.item {
                spans.insert(name.as_str(), item_node.span);
                let mut field_deps = Vec::new();
                for field in fields {
                    self.collect_class_deps(&field.ty, &mut field_deps);
                }
                deps.insert(name.as_str(), field_deps);
            }
        }

        let mut visited = HashSet::new();
        let mut stack = Vec::new();
        let mut in_stack = HashSet::new();

        for &name in deps.keys() {
            if !visited.contains(name) {
                self.detect_cycle(
                    name,
                    &deps,
                    &spans,
                    &mut visited,
                    &mut stack,
                    &mut in_stack,
                    diag,
                );
            }
        }
    }

    fn collect_class_deps<'a>(&self, ty: &'a Type, deps: &mut Vec<&'a str>) {
        match ty {
            Type::Class(name) => {
                deps.push(name.as_str());
            }
            Type::Ptr(inner) | Type::List(inner) => {
                self.collect_class_deps(inner, deps);
            }
            Type::Array(_, inner) => {
                self.collect_class_deps(inner, deps);
            }
            Type::Int | Type::Float | Type::Bool | Type::Char | Type::Str | Type::Void => {}
        }
    }

    fn detect_cycle(
        &mut self,
        current: &str,
        deps: &HashMap<&str, Vec<&str>>,
        spans: &HashMap<&str, Span>,
        visited: &mut HashSet<String>,
        stack: &mut Vec<String>,
        in_stack: &mut HashSet<String>,
        diag: &mut DiagCtxt,
    ) {
        visited.insert(current.to_string());
        stack.push(current.to_string());
        in_stack.insert(current.to_string());

        if let Some(neighbors) = deps.get(current) {
            for &next in neighbors {
                if !visited.contains(next) {
                    self.detect_cycle(next, deps, spans, visited, stack, in_stack, diag);
                } else if in_stack.contains(next) {
                    let cycle_start = stack.iter().position(|n| n == next).unwrap();
                    let cycle: Vec<&str> = stack[cycle_start..]
                        .iter()
                        .map(|s| s.as_str())
                        .chain(std::iter::once(next))
                        .collect();
                    let span = spans
                        .get(current)
                        .copied()
                        .unwrap_or_else(|| Span::new(0.into(), 0.into()));
                    let err = diag
                        .error(
                            span,
                            format!(
                                "recursive type definition: {}",
                                cycle.join(" -> ")
                            ),
                        )
                        .build();
                    diag.emit(err);
                }
            }
        }

        stack.pop();
        in_stack.remove(current);
    }
}
