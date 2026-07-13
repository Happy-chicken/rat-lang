use inkwell::AddressSpace;
use inkwell::IntPredicate;
use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::{BasicType, BasicTypeEnum, StructType};
use inkwell::values::{BasicValue, BasicValueEnum, IntValue, PointerValue};
use std::collections::HashMap;
use std::cell::Cell;
use crate::common::DiagCtxt;
use crate::common::span::Span;
use crate::frontend::ast::Program;
use crate::frontend::ast::{expr::*, item::*, stmt::*, typ::Type as AstType};
use crate::frontend::sema_checker::sema_ctx::SemaCtxt;

use super::env::{ClassInfo, Env, VarInfo, ListTypeRegistry};

pub struct IrEmitter<'a, 'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    env: Env<'ctx>,
    diag: &'a mut DiagCtxt,
    current_function: Option<inkwell::values::FunctionValue<'ctx>>,
    class_info: HashMap<String, ClassInfo<'ctx>>,
    list_types: ListTypeRegistry<'ctx>,
    str_counter: Cell<u64>,
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
            current_function: None,
            class_info: HashMap::new(),
            list_types: ListTypeRegistry::new(),
            str_counter: Cell::new(0),
        }
    }

    pub fn generate(&mut self, program: &Program, sema_ctx: &SemaCtxt) {
        self.build_class_types(program, sema_ctx);
        self.build_class_constructors();
        for item_node in &program.items {
            let span = item_node.span;
            match &item_node.item {
                Item::VarDef(global) => self.compile_global_var(global, span),
                Item::FunctionDef(func) => self.compile_function(func, span),
                Item::Impl(imp) => {
                    for method in &imp.methods {
                        self.compile_method(&imp.class_name, method, span);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn dump_module(&self) {
        self.module.print_to_stderr();
    }

    pub fn optimize_llvm(&self) -> Result<bool, String> {
        crate::midend::optimizer::init_native_target();
        crate::midend::optimizer::run_llvm_optimizations(&self.module, "default<O2>")
    }

    pub fn optimize_llvm_targeted(&self, passes: &str) -> Result<bool, String> {
        crate::midend::optimizer::init_native_target();
        crate::midend::optimizer::run_llvm_optimizations(&self.module, passes)
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

    fn build_class_types(&mut self, program: &Program, _sema_ctx: &SemaCtxt) {
        for item_node in &program.items {
            if let Item::Class(class) = &item_node.item {
                let mut field_tys: Vec<BasicTypeEnum<'ctx>> = Vec::new();
                let mut field_indices: HashMap<String, u32> = HashMap::new();
                let mut field_defaults: Vec<Option<ExprNode>> = Vec::new();
                for (i, field) in class.fields.iter().enumerate() {
                    field_tys.push(self.ast_type_to_llvm(&field.ty));
                    field_indices.insert(field.name.clone(), i as u32);
                    field_defaults.push(field.init.clone());
                }
                let field_refs: Vec<_> = field_tys.iter().map(|t| (*t).into()).collect();
                let named = self.context.opaque_struct_type(&class.name);
                named.set_body(&field_refs, false);
                self.class_info.insert(
                    class.name.clone(),
                    ClassInfo {
                        struct_ty: named,
                        field_indices,
                        field_types: field_tys,
                        field_defaults,
                        methods: HashMap::new(),
                    },
                );
            }
            else if let Item::Impl(imp) = &item_node.item {
                let prefix = format!("{}_", imp.class_name);
                for method in &imp.methods {
                    let mangled = format!("{}{}", prefix, method.function_header.name);
                    if let Some(info) = self.class_info.get_mut(&imp.class_name) {
                        info.methods.insert(method.function_header.name.clone(), mangled);
                    }
                }
            }
        }
    }

    fn build_class_constructors(&mut self) {
        let class_data: Vec<(String, StructType<'ctx>, Vec<BasicTypeEnum<'ctx>>)> =
            self.class_info
                .iter()
                .map(|(name, info)| (name.clone(), info.struct_ty, info.field_types.clone()))
                .collect();

        for (name, struct_ty, field_tys) in class_data {
            self.compile_class_constructor(&name, struct_ty, &field_tys);
        }
    }

    fn compile_class_constructor(
        &mut self,
        class_name: &str,
        struct_ty: StructType<'ctx>,
        field_tys: &[BasicTypeEnum<'ctx>],
    ) {
        let mangled = format!("{}_new", class_name);

        let param_meta: Vec<_> = field_tys.iter().map(|t| (*t).into()).collect();
        let fn_type = struct_ty.fn_type(&param_meta, false);

        let function = self.module.add_function(&mangled, fn_type, None);
        let saved_fn = self.current_function;
        self.current_function = Some(function);
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        let alloca = self
            .builder
            .build_alloca(struct_ty, "ctor.tmp")
            .unwrap();

        for (i, _) in field_tys.iter().enumerate() {
            if let Some(param) = function.get_nth_param(i as u32) {
                let field_ptr = self
                    .builder
                    .build_struct_gep(struct_ty, alloca, i as u32, "ctor.field")
                    .unwrap_or(alloca);
                let _ = self.builder.build_store(field_ptr, param);
            }
        }

        let loaded = self
            .builder
            .build_load(struct_ty, alloca, "ctor.load")
            .unwrap();
        let _ = self.builder.build_return(Some(&loaded));

        self.current_function = saved_fn;
    }

    fn get_class_info(&self, name: &str) -> Option<&ClassInfo<'ctx>> {
        self.class_info.get(name)
    }

    fn get_class_struct_ty(&self, name: &str) -> Option<StructType<'ctx>> {
        self.class_info.get(name).map(|c| c.struct_ty)
    }

    fn get_list_struct_type(&self, elem_ty: BasicTypeEnum<'ctx>) -> StructType<'ctx> {
        self.list_types.make(self.context, elem_ty)
    }

    fn get_list_elem(&self, st: StructType<'ctx>) -> BasicTypeEnum<'ctx> {
        self.list_types.elem_type(st)
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

    fn compile_method(&mut self, class_name: &str, func: &FunctionDef, span: Span) {
        let mangled = format!("{}_{}", class_name, func.function_header.name);
        self.compile_function_named(func, &mangled, span);
    }

    fn compile_function(&mut self, func: &FunctionDef, span: Span) {
        self.compile_function_named(func, &func.function_header.name, span);
    }

    fn compile_function_named(&mut self, func: &FunctionDef, name: &str, span: Span) {
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
            _ => {
                let ret_llvm = self.ast_type_to_llvm(&ret_ast_ty);
                ret_llvm.fn_type(&param_meta, false)
            }
        };

        let function = self
            .module
            .add_function(name, fn_type, None);
        self.current_function = Some(function);
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
                    None => {
                        if let Some(init_expr) = init {
                            match &init_expr.expr {
                                Expr::List { .. } => self.context.i64_type().into(),
                                _ => self.infer_lit_type(&init_expr.expr),
                            }
                        } else {
                            self.context.i64_type().into()
                        }
                    }
                };
                match self.builder.build_alloca(llvm_ty, name) {
                    Ok(alloca) => {
                        if let Some(init_expr) = init {
                            match &init_expr.expr {
                                Expr::Assign { target, value } => {
                                    let _ = self.compile_assign(target, value, span);
                                }
                                Expr::List { elements } => {
                                    self.emit_list_init(alloca, elements, span);
                                }
                                _ => {
                                    let value = self.compile_expr(init_expr);
                                    let _ = self.builder.build_store(alloca, value);
                                }
                            }
                        } else if let Some(AstType::Class(class_name)) = ty {
                            if let Some(info) = self.class_info.get(class_name) {
                                let defaults = info.field_defaults.clone();
                                if let Some(class_st) = self.get_class_struct_ty(class_name) {
                                    for (i, default) in defaults.iter().enumerate() {
                                        let val = if let Some(default_expr) = default {
                                            self.compile_expr(default_expr)
                                        } else {
                                            self.context.i64_type().const_zero().into()
                                        };
                                        let field_ptr = self
                                            .builder
                                            .build_struct_gep(
                                                class_st,
                                                alloca,
                                                i as u32,
                                                "init.field",
                                            )
                                            .unwrap_or(alloca);
                                    let _ = self.builder.build_store(field_ptr, val);
                                    }
                                }
                            }
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
            Stmt::If {
                condition,
                then_branch,
                elif_branch,
                else_branch,
            } => {
                self.compile_if(condition, then_branch, elif_branch, else_branch, span);
            }
            Stmt::Loop { condition, body } => {
                self.compile_while(condition, body, span);
            }
            Stmt::Break => {
                if let Some(info) = self.env.lookup_loop() {
                    let _ = self.builder.build_unconditional_branch(info.exit_bb);
                } else {
                    let d = self.diag.error(span, "break outside of loop").build();
                    self.diag.emit(d);
                }
            }
            Stmt::Continue => {
                if let Some(info) = self.env.lookup_loop() {
                    let _ = self.builder.build_unconditional_branch(info.cond_bb);
                } else {
                    let d = self.diag.error(span, "continue outside of loop").build();
                    self.diag.emit(d);
                }
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
                                .error(span, format!("failed to load '{}': {}", name, e))
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
            Expr::Unary { op, expr } => self.compile_unary(op, expr, span),
            Expr::Call { callee, args } => {
                if let Expr::Member { object, field } = &callee.expr {
                    let mangled = self.resolve_method(object, field);
                    match self.module.get_function(&mangled) {
                        Some(function) => {
                            let mut arg_vals: Vec<BasicValueEnum> = vec![self.compile_expr(object)];
                            for arg in args {
                                arg_vals.push(self.compile_expr(arg));
                            }
                            let arg_refs: Vec<_> = arg_vals.iter().map(|v| (*v).into()).collect();
                            match self.builder.build_call(function, &arg_refs, "call") {
                                Ok(call) => call.try_as_basic_value().basic().unwrap_or(zero),
                                Err(e) => {
                                    let d = self
                                        .diag
                                        .error(span, format!("call failed: {}", e))
                                        .build();
                                    self.diag.emit(d);
                                    zero
                                }
                            }
                        }
                        None => {
                            let d = self
                                .diag
                                .error(span, format!("undefined method: {}", field))
                                .build();
                            self.diag.emit(d);
                            zero
                        }
                    }
                } else if let Expr::Variable(ref name) = callee.expr {
                    match self.module.get_function(name) {
                        Some(function) => {
                            let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
                            for arg in args {
                                arg_vals.push(self.compile_expr(arg));
                            }
                            let arg_refs: Vec<_> = arg_vals.iter().map(|v| (*v).into()).collect();
                            match self.builder.build_call(function, &arg_refs, "call") {
                                Ok(call) => call.try_as_basic_value().basic().unwrap_or(zero),
                                Err(e) => {
                                    let d = self
                                        .diag
                                        .error(span, format!("call failed: {}", e))
                                        .build();
                                    self.diag.emit(d);
                                    zero
                                }
                            }
                        }
                        None => {
                            if let Some(info) = self.class_info.get(name) {
                                let defaults = info.field_defaults.clone();
                                let total_fields = defaults.len();

                                let new_name = format!("{}_new", name);
                                if let Some(function) = self.module.get_function(&new_name) {
                                    let mut arg_vals: Vec<BasicValueEnum> = Vec::new();
                                    for i in 0..total_fields {
                                        let val = if i < args.len() {
                                            self.compile_expr(&args[i])
                                        } else {
                                            let default_val =
                                                defaults.get(i).and_then(|o| o.as_ref());
                                            if let Some(default_expr) = default_val {
                                                self.compile_expr(default_expr)
                                            } else {
                                                self.context.i64_type().const_zero().into()
                                            }
                                        };
                                        arg_vals.push(val);
                                    }
                                    let arg_refs: Vec<_> =
                                        arg_vals.iter().map(|v| (*v).into()).collect();
                                    match self.builder.build_call(function, &arg_refs, "ctor") {
                                        Ok(call) => {
                                            call.try_as_basic_value().basic().unwrap_or(zero)
                                        }
                                        Err(e) => {
                                            let d = self
                                                .diag
                                                .error(
                                                    span,
                                                    format!("ctor call failed: {}", e),
                                                )
                                                .build();
                                            self.diag.emit(d);
                                            zero
                                        }
                                    }
                                } else {
                                    let d = self
                                        .diag
                                        .error(
                                            span,
                                            format!("undefined function: {}", name),
                                        )
                                        .build();
                                    self.diag.emit(d);
                                    zero
                                }
                            } else {
                                let d = self
                                    .diag
                                    .error(span, format!("undefined function: {}", name))
                                    .build();
                                self.diag.emit(d);
                                zero
                            }
                        }
                    }
                } else {
                    let d = self
                        .diag
                        .error(span, "indirect function calls not supported")
                        .build();
                    self.diag.emit(d);
                    zero
                }
            }
            Expr::Assign { target, value } => self.compile_assign(target, value, span),
            Expr::Member { object, field } => self.compile_member_access(object, field, span),
            Expr::List { elements } => self.emit_list_literal_expr(elements, span),
            Expr::Index { object, index } => self.compile_index(object, index, span),
            _ => {
                let d = self.diag.error(span, "unsupported expression").build();
                self.diag.emit(d);
                zero
            }
        }
    }

    fn emit_list_literal_expr(
        &mut self,
        elements: &[ExprNode],
        span: Span,
    ) -> BasicValueEnum<'ctx> {
        let zero = self.context.i64_type().const_zero().into();

        let elem_llvm_ty: BasicTypeEnum = if !elements.is_empty() {
            self.infer_lit_type(&elements[0].expr)
        } else {
            self.context.i64_type().into()
        };

        let list_struct = self.get_list_struct_type(elem_llvm_ty);
        let alloca = match self.builder.build_alloca(list_struct, "list.tmp") {
            Ok(a) => a,
            Err(e) => {
                let d = self
                    .diag
                    .error(span, format!("alloca failed: {}", e))
                    .build();
                self.diag.emit(d);
                return zero;
            }
        };

        self.emit_list_init_fields(alloca, list_struct, elements, elem_llvm_ty, span);

        match self.builder.build_load(list_struct, alloca, "list.load") {
            Ok(v) => v.into(),
            Err(e) => {
                let d = self.diag.error(span, format!("load failed: {}", e)).build();
                self.diag.emit(d);
                zero
            }
        }
    }

    fn emit_list_init(&mut self, alloca: PointerValue<'ctx>, elements: &[ExprNode], span: Span) {
        let elem_llvm_ty: BasicTypeEnum = if !elements.is_empty() {
            self.infer_lit_type(&elements[0].expr)
        } else {
            self.context.i64_type().into()
        };

        let list_struct = self.get_list_struct_type(elem_llvm_ty);
        self.emit_list_init_fields(alloca, list_struct, elements, elem_llvm_ty, span);
    }

    fn emit_list_init_fields(
        &mut self,
        alloca: PointerValue<'ctx>,
        list_struct: StructType<'ctx>,
        elements: &[ExprNode],
        elem_llvm_ty: BasicTypeEnum<'ctx>,
        span: Span,
    ) {
        let elem_count = elements.len() as u64;
        let i64_ty = self.context.i64_type();

        let len_ptr = self
            .builder
            .build_struct_gep(list_struct, alloca, 0, "list.len")
            .unwrap_or_else(|_| alloca);
        let _ = self
            .builder
            .build_store(len_ptr, i64_ty.const_int(elem_count, false));

        let cap_ptr = self
            .builder
            .build_struct_gep(list_struct, alloca, 1, "list.cap")
            .unwrap_or_else(|_| alloca);
        let _ = self
            .builder
            .build_store(cap_ptr, i64_ty.const_int(elem_count, false));

        if elements.is_empty() {
            let data_ptr = self
                .builder
                .build_struct_gep(list_struct, alloca, 2, "list.data")
                .unwrap_or_else(|_| alloca);
            let _ = self.builder.build_store(
                data_ptr,
                self.context.ptr_type(AddressSpace::default()).const_zero(),
            );
            return;
        }

        let array_ty = match elem_llvm_ty {
            BasicTypeEnum::IntType(t) => t.array_type(elements.len() as u32),
            BasicTypeEnum::FloatType(t) => t.array_type(elements.len() as u32),
            BasicTypeEnum::StructType(t) => t.array_type(elements.len() as u32),
            _ => self.context.i64_type().array_type(elements.len() as u32),
        };
        let array_alloca = match self.builder.build_alloca(array_ty, "list.buf") {
            Ok(a) => a,
            Err(_) => return,
        };

        for (i, elem_expr) in elements.iter().enumerate() {
            let elem_val = self.compile_expr(elem_expr);
            let elem_ptr = unsafe {
                self.builder
                    .build_gep(
                        array_ty,
                        array_alloca,
                        &[
                            i64_ty.const_zero().into(),
                            i64_ty.const_int(i as u64, false).into(),
                        ],
                        "list.elem",
                    )
                    .unwrap_or(array_alloca)
            };
            let _ = self.builder.build_store(elem_ptr, elem_val);
        }

        let data_ptr = self
            .builder
            .build_struct_gep(list_struct, alloca, 2, "list.data")
            .unwrap_or_else(|_| alloca);

        let i64_ty = self.context.i64_type();
        let first_elem = unsafe {
            self.builder
                .build_gep(
                    array_ty,
                    array_alloca,
                    &[i64_ty.const_zero().into(), i64_ty.const_zero().into()],
                    "list.data.cast",
                )
                .unwrap_or(array_alloca)
        };
        let _ = self.builder.build_store(data_ptr, first_elem);
    }

    fn compile_if(
        &mut self,
        condition: &ExprNode,
        then_branch: &Block,
        elif_branch: &[(ExprNode, Block)],
        else_branch: &Block,
        span: Span,
    ) {
        let cond_val = self.compile_expr(condition);
        let cond_bool = self.to_bool(cond_val);

        let parent = self
            .builder
            .get_insert_block()
            .and_then(|bb| bb.get_parent())
            .or(self.current_function);

        let then_bb = self.context.append_basic_block(parent.unwrap(), "if.then");
        let else_bb = self.context.append_basic_block(parent.unwrap(), "if.else");
        let merge_bb = self.context.append_basic_block(parent.unwrap(), "if.merge");

        let _ = self
            .builder
            .build_conditional_branch(cond_bool, then_bb, else_bb);

        self.builder.position_at_end(then_bb);
        self.enter_scope();
        self.compile_block(then_branch);
        self.exit_scope();
        if self
            .builder
            .get_insert_block()
            .unwrap()
            .get_terminator()
            .is_none()
        {
            let _ = self.builder.build_unconditional_branch(merge_bb);
        }

        self.builder.position_at_end(else_bb);

        if elif_branch.is_empty() && else_branch.stmts.is_empty() {
            let _ = self.builder.build_unconditional_branch(merge_bb);
        } else if !elif_branch.is_empty() {
            self.compile_elif_chain(elif_branch, else_branch, merge_bb, span);
        } else {
            self.enter_scope();
            self.compile_block(else_branch);
            self.exit_scope();
            if self
                .builder
                .get_insert_block()
                .unwrap()
                .get_terminator()
                .is_none()
            {
                let _ = self.builder.build_unconditional_branch(merge_bb);
            }
        }

        self.builder.position_at_end(merge_bb);
    }

    fn compile_elif_chain(
        &mut self,
        elif_branch: &[(ExprNode, Block)],
        else_branch: &Block,
        merge_bb: BasicBlock<'ctx>,
        span: Span,
    ) {
        for (i, (cond, block)) in elif_branch.iter().enumerate() {
            let cond_val = self.compile_expr(cond);
            let cond_bool = self.to_bool(cond_val);

            let then_bb = self.context.append_basic_block(
                self.builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap(),
                &format!("if.elif{}", i),
            );
            let next_bb = self.context.append_basic_block(
                self.builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap(),
                &format!("if.elif{}.next", i),
            );

            let _ = self
                .builder
                .build_conditional_branch(cond_bool, then_bb, next_bb);

            self.builder.position_at_end(then_bb);
            self.enter_scope();
            self.compile_block(block);
            self.exit_scope();
            if self
                .builder
                .get_insert_block()
                .unwrap()
                .get_terminator()
                .is_none()
            {
                let _ = self.builder.build_unconditional_branch(merge_bb);
            }

            self.builder.position_at_end(next_bb);
        }

        if !else_branch.stmts.is_empty() {
            self.enter_scope();
            self.compile_block(else_branch);
            self.exit_scope();
            if self
                .builder
                .get_insert_block()
                .unwrap()
                .get_terminator()
                .is_none()
            {
                let _ = self.builder.build_unconditional_branch(merge_bb);
            }
        } else {
            let _ = self.builder.build_unconditional_branch(merge_bb);
        }
    }

    fn compile_while(&mut self, condition: &ExprNode, body: &Block, span: Span) {
        let parent = self
            .builder
            .get_insert_block()
            .and_then(|bb| bb.get_parent())
            .or(self.current_function)
            .unwrap();

        let cond_bb = self.context.append_basic_block(parent, "while.cond");
        let body_bb = self.context.append_basic_block(parent, "while.body");
        let exit_bb = self.context.append_basic_block(parent, "while.exit");

        let _ = self.builder.build_unconditional_branch(cond_bb);

        self.builder.position_at_end(cond_bb);
        let cond_val = self.compile_expr(condition);
        let cond_bool = self.to_bool(cond_val);
        let _ = self
            .builder
            .build_conditional_branch(cond_bool, body_bb, exit_bb);

        self.builder.position_at_end(body_bb);
        self.enter_scope();
        self.env.set_loop(cond_bb, exit_bb);
        self.compile_block(body);
        self.exit_scope();

        if self
            .builder
            .get_insert_block()
            .unwrap()
            .get_terminator()
            .is_none()
        {
            let _ = self.builder.build_unconditional_branch(cond_bb);
        }

        self.builder.position_at_end(exit_bb);
    }

    fn to_bool(&self, val: BasicValueEnum<'ctx>) -> IntValue<'ctx> {
        val.into_int_value()
    }

    fn compile_index(
        &mut self,
        object: &ExprNode,
        index: &ExprNode,
        span: Span,
    ) -> BasicValueEnum<'ctx> {
        let zero = self.context.i64_type().const_zero().into();

        let obj_val = self.compile_expr(object);
        let idx_val = self.compile_expr(index);
        let idx_int = idx_val.into_int_value();

        if obj_val.is_pointer_value() {
            let i8_ty: BasicTypeEnum = self.context.i8_type().into();
            let elem_ptr = unsafe {
                self.builder.build_gep(i8_ty, obj_val.into_pointer_value(), &[idx_int.into()], "str.idx")
                    .unwrap_or(obj_val.into_pointer_value())
            };
            return match self.builder.build_load(i8_ty, elem_ptr, "str.char") {
                Ok(v) => v,
                Err(_) => zero,
            };
        }

        let obj_struct_val = obj_val.into_struct_value();

        let list_struct = obj_struct_val.get_type();

        let len_val = match self
            .builder
            .build_extract_value(obj_struct_val, 0, "list.len")
        {
            Ok(v) => v.into_int_value(),
            Err(_) => {
                let d = self.diag.error(span, "failed to extract list len").build();
                self.diag.emit(d);
                return zero;
            }
        };
        let data_val = match self
            .builder
            .build_extract_value(obj_struct_val, 2, "list.data")
        {
            Ok(v) => v.into_pointer_value(),
            Err(_) => {
                let d = self.diag.error(span, "failed to extract list data").build();
                self.diag.emit(d);
                return zero;
            }
        };

        self.emit_bounds_check(idx_int, len_val, span);

        let elem_llvm_ty: BasicTypeEnum = self.get_list_elem(list_struct);

        let elem_ptr = unsafe {
            self.builder
                .build_gep(elem_llvm_ty, data_val, &[idx_int.into()], "list.idx")
                .unwrap_or(data_val)
        };

        match self
            .builder
            .build_load(elem_llvm_ty, elem_ptr, "list.elem.load")
        {
            Ok(v) => v,
            Err(e) => {
                let d = self
                    .diag
                    .error(span, format!("load element failed: {}", e))
                    .build();
                self.diag.emit(d);
                zero
            }
        }
    }

    fn emit_bounds_check(&mut self, index: IntValue<'ctx>, len: IntValue<'ctx>, _span: Span) {
        let zero = self.context.i64_type().const_zero();

        let is_negative = match self.builder.build_int_compare(
            inkwell::IntPredicate::SLT,
            index,
            zero,
            "bounds.low",
        ) {
            Ok(v) => v,
            Err(_) => return,
        };
        let is_oob = match self.builder.build_int_compare(
            inkwell::IntPredicate::SGE,
            index,
            len,
            "bounds.high",
        ) {
            Ok(v) => v,
            Err(_) => return,
        };
        let is_bad = match self.builder.build_or(is_negative, is_oob, "bounds.bad") {
            Ok(v) => v,
            Err(_) => return,
        };

        let trap_bb = self.context.append_basic_block(
            self.builder
                .get_insert_block()
                .unwrap()
                .get_parent()
                .unwrap(),
            "bounds.trap",
        );
        let ok_bb = self.context.append_basic_block(
            self.builder
                .get_insert_block()
                .unwrap()
                .get_parent()
                .unwrap(),
            "bounds.ok",
        );

        let _ = self
            .builder
            .build_conditional_branch(is_bad, trap_bb, ok_bb);

        self.builder.position_at_end(trap_bb);
        let _ = self.builder.build_return(None);

        self.builder.position_at_end(ok_bb);
    }

    // --- assignment ---

    fn compile_assign(
        &mut self,
        target: &ExprNode,
        value: &ExprNode,
        span: Span,
    ) -> BasicValueEnum<'ctx> {
        let zero = self.context.i64_type().const_zero().into();

        match &target.expr {
            Expr::Variable(name) => {
                if let Some(info) = self.env.lookup(name) {
                    match &value.expr {
                        Expr::List { elements } => {
                            self.emit_list_init(info.ptr, elements, span);
                            zero
                        }
                        _ => {
                            let val = self.compile_expr(value);
                            let _ = self.builder.build_store(info.ptr, val);
                            val
                        }
                    }
                } else {
                    let d = self
                        .diag
                        .error(span, format!("cannot find `{}` for assignment", name))
                        .build();
                    self.diag.emit(d);
                    zero
                }
            }

            Expr::Index { .. } => {
                let ptr = self.compile_index_ptr(target, span);
                let val = self.compile_expr(value);
                let _ = self.builder.build_store(ptr, val);
                val
            }

            Expr::Member { .. } => {
                let ptr = self.compile_member_ptr(target, span);
                let val = self.compile_expr(value);
                let _ = self.builder.build_store(ptr, val);
                val
            }

            Expr::Unary { op: UnaryOp::Deref, expr: inner } => {
                let ptr_val = self.compile_expr(inner).into_pointer_value();
                let val = self.compile_expr(value);
                let _ = self.builder.build_store(ptr_val, val);
                val
            }

            _ => {
                let d = self
                    .diag
                    .error(span, "unsupported assignment target")
                    .build();
                self.diag.emit(d);
                zero
            }
        }
    }

    fn compile_index_ptr(&mut self, expr_node: &ExprNode, span: Span) -> PointerValue<'ctx> {
        if let Expr::Index { object, index } = &expr_node.expr {
            let obj_val = self.compile_expr(object);
            let idx_val = self.compile_expr(index);

            let obj_struct_val = obj_val.into_struct_value();

            let len_val = match self
                .builder
                .build_extract_value(obj_struct_val, 0, "list.len")
            {
                Ok(v) => v.into_int_value(),
                Err(_) => return self.context.ptr_type(AddressSpace::default()).const_null(),
            };
            let data_val = match self
                .builder
                .build_extract_value(obj_struct_val, 2, "list.data")
            {
                Ok(v) => v.into_pointer_value(),
                Err(_) => return self.context.ptr_type(AddressSpace::default()).const_null(),
            };

            let idx_int = idx_val.into_int_value();
            self.emit_bounds_check(idx_int, len_val, span);

            let list_struct = obj_struct_val.get_type();
            let elem_llvm_ty: BasicTypeEnum = self.get_list_elem(list_struct);
            unsafe {
                self.builder
                    .build_gep(elem_llvm_ty, data_val, &[idx_int.into()], "list.assign.ptr")
                    .unwrap_or(data_val)
            }
        } else {
            self.context.ptr_type(AddressSpace::default()).const_null()
        }
    }

    // --- class: field access ---

    fn compile_member_access(
        &mut self,
        object: &ExprNode,
        field: &str,
        span: Span,
    ) -> BasicValueEnum<'ctx> {
        let zero = self.context.i64_type().const_zero().into();

        if let Some((class_name, field_idx)) = self.resolve_member_field(field) {
            if let Some(class_st) = self.get_class_struct_ty(&class_name) {
                if let Some(info) = self.get_class_info(&class_name) {
                    let obj_ptr = self.get_object_ptr(object, span);
                    let field_ptr = self
                        .builder
                        .build_struct_gep(class_st, obj_ptr, field_idx, "member.field")
                        .unwrap_or(obj_ptr);
                    let field_ty = info
                        .field_types
                        .get(field_idx as usize)
                        .copied()
                        .unwrap_or_else(|| self.context.i64_type().into());
                    return match self.builder.build_load(
                        field_ty,
                        field_ptr,
                        &format!("load.{}", field),
                    ) {
                        Ok(v) => v,
                        Err(_) => zero,
                    };
                }
            }
        }
        zero
    }

    fn compile_member_ptr(&mut self, expr_node: &ExprNode, span: Span) -> PointerValue<'ctx> {
        if let Expr::Member { object, field } = &expr_node.expr {
            if let Some((class_name, field_idx)) = self.resolve_member_field(field) {
                if let Some(class_st) = self.get_class_struct_ty(&class_name) {
                    let obj_ptr = self.get_object_ptr(object, span);
                    return self
                        .builder
                        .build_struct_gep(class_st, obj_ptr, field_idx, "member.ptr")
                        .unwrap_or(obj_ptr);
                }
            }
        }
        self.context.ptr_type(AddressSpace::default()).const_null()
    }

    fn resolve_member_field(&self, field: &str) -> Option<(String, u32)> {
        for (class_name, info) in &self.class_info {
            if let Some(&idx) = info.field_indices.get(field) {
                return Some((class_name.clone(), idx));
            }
        }
        None
    }

    fn resolve_method(&self, object: &ExprNode, method: &str) -> String {
        if let Expr::Variable(name) = &object.expr {
            if let Some(info) = self.env.lookup(name) {
                for (class_name, class_info) in &self.class_info {
                    if info.ty == class_info.struct_ty.into() {
                        if let Some(mangled) = class_info.methods.get(method) {
                            return mangled.clone();
                        }
                    }
                }
            }
        }
        method.to_string()
    }

    fn get_object_ptr(&self, expr_node: &ExprNode, _span: Span) -> PointerValue<'ctx> {
        match &expr_node.expr {
            Expr::Variable(name) => {
                if let Some(info) = self.env.lookup(name) {
                    return info.ptr;
                }
            }
            Expr::Member { object, .. } => {
                return self.get_object_ptr(object, _span);
            }
            _ => {}
        }
        self.context.ptr_type(AddressSpace::default()).const_null()
    }

    fn get_object_class(&self, expr_node: &ExprNode) -> Option<String> {
        match &expr_node.expr {
            Expr::Variable(name) => {
                for (class_name, _) in &self.class_info {
                    if self.env.lookup(name).is_some() {
                        return Some(class_name.clone());
                    }
                }
            }
            Expr::Member { object, .. } => {
                return self.get_object_class(object);
            }
            _ => {}
        }
        None
    }

    fn compile_const_literal(&self, lit: &Literal) -> Option<BasicValueEnum<'ctx>> {
        match lit {
            Literal::Int(i) => Some(self.context.i64_type().const_int(*i as u64, true).into()),
            Literal::Bool(b) => Some(self.context.bool_type().const_int(*b as u64, false).into()),
            Literal::Float(f) => Some(self.context.f32_type().const_float(*f as f64).into()),
            Literal::Char(c) => Some(self.context.i8_type().const_int(*c as u64, false).into()),
            _ => None,
        }
    }

    fn infer_lit_type(&self, expr: &Expr) -> BasicTypeEnum<'ctx> {
        match expr {
            Expr::Literal(Literal::Int(_)) => self.context.i64_type().into(),
            Expr::Literal(Literal::Float(_)) => self.context.f32_type().into(),
            Expr::Literal(Literal::Bool(_)) => self.context.bool_type().into(),
            Expr::Literal(Literal::Char(_)) => self.context.i8_type().into(),
            Expr::Literal(Literal::StringLiteral(_)) => self.context.ptr_type(AddressSpace::default()).into(),
            Expr::List { elements } => {
                let inner = if elements.is_empty() {
                    self.context.i64_type().into()
                } else {
                    self.infer_lit_type(&elements[0].expr)
                };
                self.get_list_struct_type(inner).into()
            }
            Expr::Unary { op: UnaryOp::AddrOf, .. } => {
                self.context.ptr_type(AddressSpace::default()).into()
            }
            _ => self.context.i64_type().into(),
        }
    }

    fn compile_literal(&self, lit: &Literal) -> BasicValueEnum<'ctx> {
        match lit {
            Literal::Int(i) => self.context.i64_type().const_int(*i as u64, true).into(),
            Literal::Float(f) => self.context.f32_type().const_float(*f as f64).into(),
            Literal::Bool(b) => self.context.bool_type().const_int(*b as u64, false).into(),
            Literal::Char(c) => self.context.i8_type().const_int(*c as u64, false).into(),
            Literal::StringLiteral(s) => self.emit_string_literal(s),
            _ => self.context.i64_type().const_zero().into(),
        }
    }

    fn emit_string_literal(&self, s: &str) -> BasicValueEnum<'ctx> {
        let id = self.str_counter.get();
        self.str_counter.set(id + 1);
        let name = format!(".str.{}", id);

        let bytes: Vec<u8> = s.bytes().chain(std::iter::once(0)).collect();
        let ty = self.context.i8_type().array_type(bytes.len() as u32);
        let global = self.module.add_global(ty, Some(AddressSpace::default()), &name);
        global.set_initializer(&self.context.i8_type().const_array(
            &bytes.iter().map(|&b| self.context.i8_type().const_int(b as u64, false)).collect::<Vec<_>>(),
        ));

        let ptr = unsafe {
            self.builder.build_gep(
                ty, global.as_pointer_value(),
                &[self.context.i64_type().const_zero().into(), self.context.i64_type().const_zero().into()],
                &format!("{}.ptr", name),
            ).unwrap_or(global.as_pointer_value())
        };
        ptr.into()
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
        let zero_i1 = self.context.bool_type().const_zero();

        let result: IntValue = match op {
            BinaryOp::Add => self
                .builder
                .build_int_add(lhs_int, rhs_int, "add")
                .unwrap_or(zero),
            BinaryOp::Sub => self
                .builder
                .build_int_sub(lhs_int, rhs_int, "sub")
                .unwrap_or(zero),
            BinaryOp::Mul => self
                .builder
                .build_int_mul(lhs_int, rhs_int, "mul")
                .unwrap_or(zero),
            BinaryOp::Div => self
                .builder
                .build_int_signed_div(lhs_int, rhs_int, "div")
                .unwrap_or(zero),
            BinaryOp::Eq => self
                .builder
                .build_int_compare(IntPredicate::EQ, lhs_int, rhs_int, "eq")
                .unwrap_or(zero_i1),
            BinaryOp::NotEq => self
                .builder
                .build_int_compare(IntPredicate::NE, lhs_int, rhs_int, "ne")
                .unwrap_or(zero_i1),
            BinaryOp::Lt => self
                .builder
                .build_int_compare(IntPredicate::SLT, lhs_int, rhs_int, "lt")
                .unwrap_or(zero_i1),
            BinaryOp::Gt => self
                .builder
                .build_int_compare(IntPredicate::SGT, lhs_int, rhs_int, "gt")
                .unwrap_or(zero_i1),
            BinaryOp::Le => self
                .builder
                .build_int_compare(IntPredicate::SLE, lhs_int, rhs_int, "le")
                .unwrap_or(zero_i1),
            BinaryOp::Ge => self
                .builder
                .build_int_compare(IntPredicate::SGE, lhs_int, rhs_int, "ge")
                .unwrap_or(zero_i1),
            BinaryOp::And => self
                .builder
                .build_and(lhs_int, rhs_int, "and")
                .unwrap_or(zero_i1),
            BinaryOp::Or => self
                .builder
                .build_or(lhs_int, rhs_int, "or")
                .unwrap_or(zero_i1),
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

    fn compile_unary(
        &mut self,
        op: &UnaryOp,
        expr: &ExprNode,
        span: Span,
    ) -> BasicValueEnum<'ctx> {
        let zero = self.context.i64_type().const_zero().into();
        match op {
            UnaryOp::AddrOf => {
                if let Expr::Variable(name) = &expr.expr {
                    if let Some(info) = self.env.lookup(name) {
                        return info.ptr.into();
                    }
                }
                let d = self.diag.error(span, "& requires a variable").build();
                self.diag.emit(d);
                self.context.ptr_type(AddressSpace::default()).const_null().into()
            }
            UnaryOp::Deref => {
                let ptr_val = self.compile_expr(expr).into_pointer_value();
                let i64_ty: BasicTypeEnum = self.context.i64_type().into();
                match self.builder.build_load(i64_ty, ptr_val, "deref") {
                    Ok(v) => v,
                    Err(_) => zero,
                }
            }
            UnaryOp::Neg => {
                let val = self.compile_expr(expr).into_int_value();
                self.builder.build_int_neg(val, "neg").unwrap_or(val).into()
            }
            UnaryOp::Not => {
                let val = self.compile_expr(expr).into_int_value();
                self.builder.build_not(val, "not").unwrap_or(val).into()
            }
            _ => {
                let d = self.diag.error(span, format!("unsupported unary op: {:?}", op)).build();
                self.diag.emit(d);
                zero
            }
        }
    }

    fn ast_type_to_llvm(&self, ty: &AstType) -> BasicTypeEnum<'ctx> {
        match ty {
            AstType::Int => self.context.i64_type().into(),
            AstType::Float => self.context.f32_type().into(),
            AstType::Bool => self.context.bool_type().into(),
            AstType::Char => self.context.i8_type().into(),
            AstType::Str => self.context.ptr_type(AddressSpace::default()).into(),
            AstType::Void => self.context.i64_type().into(),
            AstType::Ptr(_inner) => self.context.ptr_type(AddressSpace::default()).into(),
            AstType::List(inner) => {
                let elem = self.ast_type_to_llvm(inner);
                self.get_list_struct_type(elem).into()
            }
            AstType::Class(name) => self
                .get_class_struct_ty(name)
                .map(|st| st.into())
                .unwrap_or(self.context.i64_type().into()),
            _ => self.context.i64_type().into(),
        }
    }
}
