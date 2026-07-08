use crate::frontend::type_checker::{typ::Type, type_ctx::TypeCtxt, type_ctx::TypedVar};
use crate::common::error::UnifyError;

pub struct Unifier<'a> {
    ctx: &'a mut TypeCtxt,
}

impl<'a> Unifier<'a> {
    pub fn new(ctx: &'a mut TypeCtxt) -> Self {
        Self { ctx }
    }

    pub fn unify(&mut self, a: &Type, b: &Type) -> Result<Type, UnifyError> {
        let a = self.ctx.resolve_type(a);
        let b = self.ctx.resolve_type(b);

        if a == Type::Error || b == Type::Error {
            return Ok(Type::Error);
        }

        match (&a, &b) {
            (Type::Var(a_id), Type::Var(b_id)) if a_id == b_id => Ok(a),

            (Type::Var(id), _) => self.unify_var(TypedVar(*id), &b),
            (_, Type::Var(id)) => self.unify_var(TypedVar(*id), &a),

            (Type::Prim(pa), Type::Prim(pb)) if pa == pb => Ok(a),

            (Type::List(ia), Type::List(ib))
            | (Type::Ptr(ia), Type::Ptr(ib)) => {
                self.unify(ia, ib)
            }

            (Type::Func(pa, ra), Type::Func(pb, rb)) if pa.len() == pb.len() => {
                for (pa_i, pb_i) in pa.iter().zip(pb.iter()) {
                    self.unify(pa_i, pb_i)?;
                }
                self.unify(ra, rb)
            }

            (Type::Class(na), Type::Class(nb)) if na == nb => Ok(a),

            _ => Err(UnifyError::Mismatch {
                expected: a,
                found: b,
            }),
        }
    }

    fn unify_var(&mut self, var: TypedVar, ty: &Type) -> Result<Type, UnifyError> {
        if let Some(existing) = self.ctx.lookup_subst(var).cloned() {
            return self.unify(&existing, ty);
        }

        if self.occurs_check(var, ty) {
            return Err(UnifyError::InfiniteType {
                var,
                ty: ty.clone(),
            });
        }

        self.ctx.substitute(var, ty.clone());
        Ok(ty.clone())
    }

    fn occurs_check(&self, var: TypedVar, ty: &Type) -> bool {
        let ty = self.ctx.resolve_type(ty);
        match &ty {
            Type::Var(id) => TypedVar(*id) == var,
            Type::List(inner) | Type::Ptr(inner) => {
                self.occurs_check(var, inner)
            }
            Type::Func(params, ret) => {
                params
                    .iter()
                    .any(|p| self.occurs_check(var, p))
                    || self.occurs_check(var, ret)
            }
            _ => false,
        }
    }
}
