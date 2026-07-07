use crate::frontend::ast::{Program, stmt::*};
use crate::frontend::type_checker::{typ::Type};
use crate::common::DiagCtxt;
use crate::frontend::sema_checker::{pass::Pass, sema_ctx::SemaCtxt};
/// 描述一段语句执行后"是否一定会退出当前函数"
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Terminator {
    Always,     // 一定 return/break/continue/panic,后续代码不可达
    Sometimes,  // 只有部分分支 return(如 if 无 else)
    Never,      // 正常执行到结尾
}

pub struct FlowAnalyzer {
    // 可以添加字段来存储分析过程中需要的状态或信息
}

impl FlowAnalyzer {
    pub fn new() -> Self {
        Self { }
    }
}

impl Pass for FlowAnalyzer {
    fn name(&self) -> &'static str { "flow_analyzer" }

    fn run(&mut self, program: & Program, ctx: &mut SemaCtxt, diag: &mut DiagCtxt) -> bool {
        true
    }
}

impl FlowAnalyzer {
    /// 分析函数体是否在所有路径都有返回值(非 unit 返回类型时必需)
    pub fn check_function_returns(&mut self, body: &Block, return_type: &Type, diag: &mut DiagCtxt){}

    fn analyze_block(&mut self, block: &Block) -> Terminator{
        Terminator::Never
    }
    fn analyze_stmt(&mut self, stmt: &Stmt) -> Terminator{
        Terminator::Never
    }
    fn analyze_if(&mut self, then: &Block, els: &Option<Block>) -> Terminator{
        Terminator::Never
    }

    /// 死代码检测:Terminator::Always 之后还有语句
    pub fn check_unreachable_code(&mut self, block: &Block, diag: &mut DiagCtxt){}
}