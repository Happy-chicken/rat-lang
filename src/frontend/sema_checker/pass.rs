use crate::frontend::ast::Program;
use crate::common::DiagCtxt;
use crate::frontend::sema_checker::{sema_ctx::SemaCtxt};
pub trait Pass {
    fn name(&self) -> &'static str;

    fn run(&mut self, program: & Program, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) -> bool;
}