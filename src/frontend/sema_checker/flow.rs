use crate::frontend::ast::typ::Type;
use crate::frontend::ast::expr::ExprNode;
use crate::frontend::ast::{Program, item::*, stmt::*};
use crate::common::DiagCtxt;
use crate::common::span::Span;
use crate::frontend::sema_checker::{pass::Pass, sema_ctx::SemaCtxt};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Terminator {
    Returns,
    Diverges,
    FallsThrough,
}

impl Terminator {
    fn is_terminal(self) -> bool {
        matches!(self, Terminator::Returns | Terminator::Diverges)
    }
}

pub struct FlowAnalyzer {}

impl FlowAnalyzer {
    pub fn new() -> Self {
        Self {}
    }
}

impl Pass for FlowAnalyzer {
    fn name(&self) -> &'static str {
        "flow_analyzer"
    }

    fn run(&mut self, program: &Program, _ctx: &mut SemaCtxt, diag: &mut DiagCtxt) -> bool {
        for item_node in &program.items {
            match &item_node.item {
                Item::FunctionDef(def) => {
                    let ret_ty = def
                        .function_header
                        .return_type
                        .clone()
                        .unwrap_or(Type::Void);
                    self.check_function_returns(&def.body, &ret_ty, item_node.span, diag);
                }
                Item::Impl(imp) => {
                    for method in &imp.methods {
                        let ret_ty = method
                            .function_header
                            .return_type
                            .clone()
                            .unwrap_or(Type::Void);
                        self.check_function_returns(&method.body, &ret_ty, item_node.span, diag);
                    }
                }
                _ => {}
            }
        }
        !diag.has_errors()
    }
}

impl FlowAnalyzer {
    pub fn check_function_returns(
        &mut self,
        body: &Block,
        return_type: &Type,
        span: Span,
        diag: &mut DiagCtxt,
    ) {
        let term = self.analyze_block(body, diag);

        if *return_type != Type::Void && term != Terminator::Returns {
            let err = diag
                .error(span, "not all control paths return a value")
                .note(format!(
                    "the function expects a return value of type {:?}",
                    return_type
                ))
                .build();
            diag.emit(err);
        }
    }

    pub fn check_unreachable_code(&mut self, block: &Block, diag: &mut DiagCtxt) {
        self.analyze_block(block, diag);
    }

    // --- Core analysis ---

    fn analyze_block(&mut self, block: &Block, diag: &mut DiagCtxt) -> Terminator {
        let mut term = Terminator::FallsThrough;

        for stmt_node in &block.stmts {
            if term.is_terminal() {
                let err = diag
                    .warn("unreachable statement")
                    .span(stmt_node.span)
                    .build();
                diag.emit(err);
                continue;
            }
            term = self.analyze_stmt(stmt_node, diag);
        }

        term
    }

    fn analyze_stmt(&mut self, stmt_node: &StmtNode, diag: &mut DiagCtxt) -> Terminator {
        match &stmt_node.stmt {
            Stmt::Return(_) => Terminator::Returns,

            Stmt::Break | Stmt::Continue => Terminator::Diverges,

            Stmt::BlockStmt(block) => self.analyze_block(block, diag),

            Stmt::ExprStmt(_) | Stmt::VarDef { .. } => Terminator::FallsThrough,

            Stmt::If {
                then_branch,
                elif_branch,
                else_branch,
                ..
            } => self.analyze_if(then_branch, elif_branch, else_branch, diag),

            Stmt::Loop { body, .. } => {
                let body_term = self.analyze_block(body, diag);
                if body_term == Terminator::Returns {
                    Terminator::Returns
                } else {
                    Terminator::FallsThrough
                }
            }
        }
    }

    fn analyze_if(
        &mut self,
        then_branch: &Block,
        elif_branch: &[(ExprNode, Block)],
        else_branch: &Block,
        diag: &mut DiagCtxt,
    ) -> Terminator {
        let then_term = self.analyze_block(then_branch, diag);

        let elif_terms: Vec<Terminator> = elif_branch
            .iter()
            .map(|(_, block)| self.analyze_block(block, diag))
            .collect();

        let has_else = !else_branch.stmts.is_empty();
        let else_term = if has_else {
            self.analyze_block(else_branch, diag)
        } else {
            Terminator::FallsThrough
        };

        let all_branches_return = then_term == Terminator::Returns
            && elif_terms.iter().all(|&t| t == Terminator::Returns)
            && (!has_else || else_term == Terminator::Returns);

        if all_branches_return && has_else {
            return Terminator::Returns;
        }

        let all_branches_diverge = then_term == Terminator::Diverges
            && elif_terms.iter().all(|&t| t == Terminator::Diverges)
            && (!has_else || else_term == Terminator::Diverges);

        if all_branches_diverge && has_else {
            return Terminator::Diverges;
        }

        Terminator::FallsThrough
    }
}
