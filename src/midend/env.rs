use std::collections::HashMap;

use inkwell::types::BasicTypeEnum;
use inkwell::values::PointerValue;

#[derive(Clone, Copy)]
pub struct VarInfo<'ctx> {
    pub ptr: PointerValue<'ctx>,
    pub ty: BasicTypeEnum<'ctx>,
}

pub struct Env<'ctx> {
    locals: HashMap<String, VarInfo<'ctx>>,
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
            parent: None,
        }
    }

    pub fn push(parent: Env<'ctx>) -> Env<'ctx> {
        Env {
            locals: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    pub fn pop(self) -> Env<'ctx> {
        self.parent
            .map(|boxed| *boxed)
            .unwrap_or_else(Env::new)
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
}
