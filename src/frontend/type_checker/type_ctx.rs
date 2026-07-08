use std::collections::HashMap;
use crate::frontend::type_checker::typ::{Type, ExprId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypedVar(pub u32);

pub struct TypeCtxt {
    next_var_id: u32,
    substitutions: HashMap<TypedVar, Type>,
    expr_types: HashMap<ExprId, Type>,
    list_literal_lengths: HashMap<ExprId, usize>,
}

impl TypeCtxt {
    pub fn new() -> Self {
        Self {
            next_var_id: 0,
            substitutions: HashMap::new(),
            expr_types: HashMap::new(),
            list_literal_lengths: HashMap::new(),
        }
    }

    // Generate a fresh type variable (holdplacer) for type inference.
    pub fn fresh_type_var(&mut self) -> Type {
        let id = self.next_var_id;
        self.next_var_id += 1;
        Type::Var(id)
    }

    pub fn fresh_var(&mut self) -> TypedVar {
        let id = self.next_var_id;
        self.next_var_id += 1;
        TypedVar(id)
    }

    pub fn substitute(&mut self, var: TypedVar, ty: Type) {
        self.substitutions.insert(var, ty);
    }

    pub fn resolve_type(&self, ty: &Type) -> Type {
        match ty {
            Type::Var(id) => {
                let tv = TypedVar(*id);
                if let Some(resolved) = self.substitutions.get(&tv) {
                    self.resolve_type(resolved)
                } else {
                    ty.clone()
                }
            }
            Type::List(inner) => Type::List(Box::new(self.resolve_type(inner))),
            Type::Ptr(inner) => Type::Ptr(Box::new(self.resolve_type(inner))),
            Type::Func(params, ret) => {
                let p: Vec<Type> = params.iter().map(|t| self.resolve_type(t)).collect();
                Type::Func(p, Box::new(self.resolve_type(ret)))
            }
            other => other.clone(),
        }
    }

    pub fn record_expr_type(&mut self, expr_id: ExprId, ty: Type) {
        self.expr_types.insert(expr_id, ty);
    }

    pub fn get_expr_type(&self, expr_id: ExprId) -> Option<&Type> {
        self.expr_types.get(&expr_id)
    }

    pub fn lookup_subst(&self, var: TypedVar) -> Option<&Type> {
        self.substitutions.get(&var)
    }

    pub fn record_list_length(&mut self, expr_id: ExprId, length: usize) {
        self.list_literal_lengths.insert(expr_id, length);
    }

    pub fn get_list_length(&self, expr_id: ExprId) -> Option<usize> {
        self.list_literal_lengths.get(&expr_id).copied()
    }
}
