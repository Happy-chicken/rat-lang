use std::collections::HashMap;
use std::cell::{Cell, RefCell};

use inkwell::AddressSpace;
use inkwell::basic_block::BasicBlock;
use inkwell::context::Context;
use inkwell::types::{BasicType, BasicTypeEnum, StructType};
use inkwell::values::PointerValue;

use crate::frontend::ast::expr::ExprNode;

#[derive(Clone, Copy)]
pub struct VarInfo<'ctx> {
    pub ptr: PointerValue<'ctx>,
    pub ty: BasicTypeEnum<'ctx>,
}

#[derive(Clone, Copy)]
pub struct LoopInfo<'ctx> {
    pub cond_bb: BasicBlock<'ctx>,
    pub exit_bb: BasicBlock<'ctx>,
}

pub struct ClassInfo<'ctx> {
    pub struct_ty: StructType<'ctx>,
    pub field_indices: HashMap<String, u32>,
    pub field_types: Vec<BasicTypeEnum<'ctx>>,
    pub field_defaults: Vec<Option<ExprNode>>,
    pub methods: HashMap<String, String>,
}

pub struct ListTypeRegistry<'ctx> {
    entries: RefCell<Vec<(StructType<'ctx>, BasicTypeEnum<'ctx>)>>,
    counter: Cell<u64>,
}

impl<'ctx> ListTypeRegistry<'ctx> {
    pub fn new() -> Self {
        Self { entries: RefCell::new(Vec::new()), counter: Cell::new(0) }
    }

    pub fn make(&self, context: &'ctx Context, elem_ty: BasicTypeEnum<'ctx>) -> StructType<'ctx> {
        if let Some(&(st, _)) = self.entries.borrow().iter().find(|(_, e)| *e == elem_ty) {
            return st;
        }
        let i64 = context.i64_type();
        let ptr_ty = context.ptr_type(AddressSpace::default());
        let id = self.counter.get();
        self.counter.set(id + 1);
        let st = context.opaque_struct_type(&format!("list.{}", id));
        st.set_body(&[i64.into(), i64.into(), ptr_ty.into()], false);
        self.entries.borrow_mut().push((st, elem_ty));
        st
    }

    pub fn elem_type(&self, st: StructType<'ctx>) -> BasicTypeEnum<'ctx> {
        self.entries.borrow().iter()
            .find(|(s, _)| *s == st)
            .map(|(_, e)| *e)
            .unwrap_or_else(|| unreachable!())
    }
}

pub struct Env<'ctx> {
    locals: HashMap<String, VarInfo<'ctx>>,
    loop_info: Option<LoopInfo<'ctx>>,
    parent: Option<Box<Env<'ctx>>>,
}

impl<'ctx> Default for Env<'ctx> {
    fn default() -> Self {
        Env::new()
    }
}

impl<'ctx> Env<'ctx> {
    pub fn new() -> Self {
        Env {
            locals: HashMap::new(),
            loop_info: None,
            parent: None,
        }
    }

    pub fn push(parent: Env<'ctx>) -> Env<'ctx> {
        Env {
            locals: HashMap::new(),
            loop_info: None,
            parent: Some(Box::new(parent)),
        }
    }

    pub fn pop(self) -> Env<'ctx> {
        self.parent.map(|boxed| *boxed).unwrap_or_else(Env::new)
    }

    pub fn declare(&mut self, name: String, info: VarInfo<'ctx>) {
        self.locals.insert(name, info);
    }

    pub fn lookup(&self, name: &str) -> Option<VarInfo<'ctx>> {
        if let Some(v) = self.locals.get(name) {
            return Some(*v);
        }
        self.parent.as_ref().and_then(|p| p.lookup(name))
    }

    pub fn set_loop(&mut self, cond_bb: BasicBlock<'ctx>, exit_bb: BasicBlock<'ctx>) {
        self.loop_info = Some(LoopInfo { cond_bb, exit_bb });
    }

    pub fn lookup_loop(&self) -> Option<LoopInfo<'ctx>> {
        if self.loop_info.is_some() {
            return self.loop_info;
        }
        self.parent.as_ref().and_then(|p| p.lookup_loop())
    }
}
