use inkwell::AddressSpace;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValueEnum, IntValue};

use crate::common::DiagCtxt;
use crate::common::span::Span;
use crate::frontend::ast::Program;
use crate::frontend::ast::{expr::*, item::*, stmt::*, typ::Type as AstType};
use crate::frontend::sema_checker::sema_ctx::SemaCtxt;

use super::env::{Env, VarInfo};

pub struct IrEmitter<'a, 'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    env: Env<'ctx>,
    diag: &'a mut DiagCtxt,
}

impl<'a, 'ctx> IrEmitter<'a, 'ctx> {
    pub fn new(context: &'ctx Context, name: &str, diag: &'a mut DiagCtxt) -> Self {
        let module = context.create_module(name);
        let builder = context.create_builder();
        IrEmitter {
            context,
            module,
            builder,
            env: Env::new(),
            diag,
        }
    }

    pub fn generate(&mut self, program: &Program, _sema_ctx: &SemaCtxt) {
        for item_node in &program.items {
            let span = item_node.span;
            match &item_node.item {
                Item::VarDef(global) => self.compile_global_var(global, span),
                Item::FunctionDef(func) => self.compile_function(func, span),
                _ => {}
            }
        }
    }

    pub fn dump_module(&self) {
        self.module.print_to_stderr();
    }

    pub fn module(&self) -> &Module<'ctx> {
        &self.module
    }

    pub fn has_errors(&self) -> bool {
        self.diag.has_errors()
    }

    fn enter_scope(&mut self) {
        let parent = std::mem::take(&mut self.env);
        self.env = Env::push(parent);
    }

    fn exit_scope(&mut self) {
        let old = std::mem::take(&mut self.env);
        self.env = old.pop();
    }

    fn compile_global_var(&mut self, global: &GlobalVar, span: Span) {
        let llvm_ty = match &global.ty {
            Some(t) => self.ast_type_to_llvm(t),
            None => {
                if let Some(init) = &global.init {
                    self.infer_lit_type(&init.expr)
                } else {
                    self.context.i64_type().into()
                }
            }
        };

        let global_val =
            self.module
                .add_global(llvm_ty, Some(AddressSpace::default()), &global.name);

        match global.init.as_ref().map(|n| &n.expr) {
            Some(Expr::Literal(lit)) => {
                if let Some(const_val) = self.compile_const_literal(lit) {
                    global_val.set_initializer(&const_val);
                }
            }
            Some(_) => {
                let err = self
                    .diag
                    .error(
                        span,
                        format!("non-constant initializer for global '{}'", global.name),
                    )
                    .build();
                self.diag.emit(err);
            }
            None => {}
        }

        self.env.declare(
            global.name.clone(),
            VarInfo {
                ptr: global_val.as_pointer_value(),
                ty: llvm_ty,
            },
        );
    }

    fn compile_function(&mut self, func: &FunctionDef, span: Span) {
        self.enter_scope();

        let ret_ast_ty = func
            .function_header
            .return_type
            .clone()
            .unwrap_or(AstType::Void);

        let param_tys: Vec<BasicTypeEnum<'ctx>> = func
            .function_header
            .params
            .iter()
            .map(|p| self.ast_type_to_llvm(&p.ty))
            .collect();

        let param_meta: Vec<_> = param_tys.iter().map(|t| (*t).into()).collect();

        let fn_type = match &ret_ast_ty {
            AstType::Void => self.context.void_type().fn_type(&param_meta, false),
            AstType::Int => self.context.i64_type().fn_type(&param_meta, false),
            AstType::Float => self.context.f32_type().fn_type(&param_meta, false),
            AstType::Bool => self.context.bool_type().fn_type(&param_meta, false),
            _ => self.context.i64_type().fn_type(&param_meta, false),
        };

        let function = self
            .module
            .add_function(&func.function_header.name, fn_type, None);
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        for (i, param) in func.function_header.params.iter().enumerate() {
            if let Some(llvm_param) = function.get_nth_param(i as u32) {
                let param_ty = self.ast_type_to_llvm(&param.ty);
                match self.builder.build_alloca(param_ty, &param.name) {
                    Ok(alloca) => {
                        let _ = self.builder.build_store(alloca, llvm_param);
                        self.env.declare(
                            param.name.clone(),
                            VarInfo {
                                ptr: alloca,
                                ty: param_ty,
                            },
                        );
                    }
                    Err(e) => {
                        let d = self
                            .diag
                            .error(
                                span,
                                format!("failed to allocate parameter '{}': {}", param.name, e),
                            )
                            .build();
                        self.diag.emit(d);
                    }
                }
            }
        }

        self.compile_block(&func.body);

        if matches!(ret_ast_ty, AstType::Void) {
            match self.builder.get_insert_block() {
                Some(last_bb) if last_bb.get_terminator().is_none() => {
                    let _ = self.builder.build_return(None);
                }
                None => {
                    let d = self
                        .diag
                        .error(
                            span,
                            format!(
                                "no insertion block in function '{}'",
                                func.function_header.name
                            ),
                        )
                        .build();
                    self.diag.emit(d);
                }
                _ => {}
            }
        }

        self.exit_scope();
    }

    fn compile_block(&mut self, block: &Block) {
        for stmt_node in &block.stmts {
            self.compile_stmt(stmt_node);
        }
    }

    fn compile_stmt(&mut self, stmt_node: &StmtNode) {
        let span = stmt_node.span;
        match &stmt_node.stmt {
            Stmt::VarDef { name, ty, init } => {
                let llvm_ty = match ty {
                    Some(t) => self.ast_type_to_llvm(t),
                    None => self.context.i64_type().into(),
                };
                match self.builder.build_alloca(llvm_ty, name) {
                    Ok(alloca) => {
                        if let Some(init_expr) = init {
                            let value = self.compile_expr(init_expr);
                            let _ = self.builder.build_store(alloca, value);
                        }
                        self.env.declare(
                            name.clone(),
                            VarInfo {
                                ptr: alloca,
                                ty: llvm_ty,
                            },
                        );
                    }
                    Err(e) => {
                        let d = self
                            .diag
                            .error(
                                span,
                                format!("failed to allocate variable '{}': {}", name, e),
                            )
                            .build();
                        self.diag.emit(d);
                    }
                }
            }
            Stmt::Return(Some(expr)) => {
                let value = self.compile_expr(expr);
                let _ = self.builder.build_return(Some(&value));
            }
            Stmt::Return(None) => {
                let _ = self.builder.build_return(None);
            }
            Stmt::ExprStmt(expr) => {
                self.compile_expr(expr);
            }
            Stmt::BlockStmt(block) => {
                self.enter_scope();
                self.compile_block(block);
                self.exit_scope();
            }
            _ => {
                let d = self
                    .diag
                    .error(span, format!("unsupported statement"))
                    .build();
                self.diag.emit(d);
            }
        }
    }

    fn compile_expr(&mut self, expr_node: &ExprNode) -> BasicValueEnum<'ctx> {
        let span = expr_node.span;
        let zero: BasicValueEnum = self.context.i64_type().const_zero().into();
        match &expr_node.expr {
            Expr::Literal(lit) => self.compile_literal(lit),
            Expr::Variable(name) => {
                if let Some(info) = self.env.lookup(name) {
                    match self
                        .builder
                        .build_load(info.ty, info.ptr, &format!("load_{}", name))
                    {
                        Ok(v) => v,
                        Err(e) => {
                            let d = self
                                .diag
                                .error(span, format!("failed to load variable '{}': {}", name, e))
                                .build();
                            self.diag.emit(d);
                            zero
                        }
                    }
                } else {
                    let d = self
                        .diag
                        .error(span, format!("cannot find value `{}` in this scope", name))
                        .build();
                    self.diag.emit(d);
                    zero
                }
            }
            Expr::Binary { op, lhs, rhs } => {
                let lhs_val = self.compile_expr(lhs);
                let rhs_val = self.compile_expr(rhs);
                self.compile_binary(op, lhs_val, rhs_val, span)
            }
            _ => {
                let d = self
                    .diag
                    .error(span, format!("unsupported expression"))
                    .build();
                self.diag.emit(d);
                zero
            }
        }
    }

    fn compile_const_literal(&self, lit: &Literal) -> Option<BasicValueEnum<'ctx>> {
        match lit {
            Literal::Int(i) => Some(self.context.i64_type().const_int(*i as u64, true).into()),
            Literal::Bool(b) => Some(self.context.bool_type().const_int(*b as u64, false).into()),
            Literal::Float(f) => Some(self.context.f32_type().const_float(*f as f64).into()),
            _ => None,
        }
    }

    fn infer_lit_type(&self, expr: &Expr) -> BasicTypeEnum<'ctx> {
        match expr {
            Expr::Literal(Literal::Int(_)) => self.context.i64_type().into(),
            Expr::Literal(Literal::Float(_)) => self.context.f32_type().into(),
            Expr::Literal(Literal::Bool(_)) => self.context.bool_type().into(),
            _ => self.context.i64_type().into(),
        }
    }

    fn compile_literal(&self, lit: &Literal) -> BasicValueEnum<'ctx> {
        match lit {
            Literal::Int(i) => self.context.i64_type().const_int(*i as u64, true).into(),
            Literal::Float(f) => self.context.f32_type().const_float(*f as f64).into(),
            Literal::Bool(b) => self.context.bool_type().const_int(*b as u64, false).into(),
            _ => self.context.i64_type().const_zero().into(),
        }
    }

    fn compile_binary(
        &mut self,
        op: &BinaryOp,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
        span: Span,
    ) -> BasicValueEnum<'ctx> {
        let lhs_int = lhs.into_int_value();
        let rhs_int = rhs.into_int_value();
        let zero = self.context.i64_type().const_zero();

        let result: IntValue = match op {
            BinaryOp::Add => match self.builder.build_int_add(lhs_int, rhs_int, "add") {
                Ok(v) => v,
                Err(e) => {
                    let d = self
                        .diag
                        .error(span, format!("failed to build add: {}", e))
                        .build();
                    self.diag.emit(d);
                    zero
                }
            },
            BinaryOp::Sub => match self.builder.build_int_sub(lhs_int, rhs_int, "sub") {
                Ok(v) => v,
                Err(e) => {
                    let d = self
                        .diag
                        .error(span, format!("failed to build sub: {}", e))
                        .build();
                    self.diag.emit(d);
                    zero
                }
            },
            BinaryOp::Mul => match self.builder.build_int_mul(lhs_int, rhs_int, "mul") {
                Ok(v) => v,
                Err(e) => {
                    let d = self
                        .diag
                        .error(span, format!("failed to build mul: {}", e))
                        .build();
                    self.diag.emit(d);
                    zero
                }
            },
            BinaryOp::Div => match self.builder.build_int_signed_div(lhs_int, rhs_int, "div") {
                Ok(v) => v,
                Err(e) => {
                    let d = self
                        .diag
                        .error(span, format!("failed to build div: {}", e))
                        .build();
                    self.diag.emit(d);
                    zero
                }
            },
            _ => {
                let d = self
                    .diag
                    .error(span, format!("unsupported binary op: {:?}", op))
                    .build();
                self.diag.emit(d);
                lhs_int
            }
        };
        result.into()
    }

    fn ast_type_to_llvm(&self, ty: &AstType) -> BasicTypeEnum<'ctx> {
        match ty {
            AstType::Int => self.context.i64_type().into(),
            AstType::Float => self.context.f32_type().into(),
            AstType::Bool => self.context.bool_type().into(),
            _ => self.context.i64_type().into(),
        }
    }
}
