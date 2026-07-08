pub mod checker;
pub mod flow;
pub mod pass;
pub mod resolver;
pub mod scope;
pub mod sema_ctx;
pub mod symbol;
pub mod symbol_table;

use crate::common::DiagCtxt;
use crate::frontend::ast::Program;
use crate::frontend::sema_checker::{
    checker::SemaChecker, flow::FlowAnalyzer, pass::Pass, resolver::Resolver, sema_ctx::SemaCtxt,
};
use crate::frontend::type_checker::checker::TypeChecker;
pub struct AnalysisPipeline {
    passes: Vec<Box<dyn Pass>>,
}

impl AnalysisPipeline {
    pub fn standard() -> Self {
        Self {
            passes: vec![
                Box::new(Resolver::new()),
                Box::new(SemaChecker::new()),
                Box::new(FlowAnalyzer::new()),
                Box::new(TypeChecker::new()),
            ],
        }
    }

    pub fn run(&mut self, program: &Program, diag: &mut DiagCtxt) -> SemaCtxt {
        let mut ctx = SemaCtxt::new();

        for pass in &mut self.passes {
            let should_continue = pass.run(program, &mut ctx, diag);
            if !should_continue {
                break; // 前面阶段有致命错误,没必要跑后面(比如符号都没收集全,类型检查会满屏假错误)
            }
        }

        ctx
    }
}
