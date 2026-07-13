use crate::frontend::ast::item::{FunctionDef, Item};
use crate::frontend::ast::Program;

pub mod algebraic_simplifier;
pub mod block_simplifier;
pub mod boolean_simplifier;
pub mod canonicalization;
pub mod constant_folder;
pub mod dead_branch;
pub mod utils;

/// AST optimization pass operating on a single function.
/// Returns `true` if the function was modified.
pub trait AstPass {
    fn name(&self) -> &'static str;
    fn run_on_function(&self, func: &mut FunctionDef) -> bool;
}

/// Manages a sequence of AST passes and runs them iteratively to a fixed point.
pub struct PassManager {
    passes: Vec<Box<dyn AstPass>>,
    max_iterations: usize,
}

impl PassManager {
    pub fn new() -> Self {
        Self {
            passes: Vec::new(),
            max_iterations: 5,
        }
    }

    #[allow(dead_code)]
    pub fn with_max_iterations(mut self, n: usize) -> Self {
        self.max_iterations = n;
        self
    }

    pub fn add_pass(&mut self, pass: Box<dyn AstPass>) {
        self.passes.push(pass);
    }

    /// Returns the standard optimization pipeline.
    /// Order matters: constant folding may expose dead branches,
    /// dead branch elimination may expose new constant expressions, etc.
    pub fn standard() -> Self {
        let mut pm = Self::new();
        pm.add_pass(Box::new(constant_folder::ConstantFolder));
        pm.add_pass(Box::new(algebraic_simplifier::AlgebraicSimplifier));
        pm.add_pass(Box::new(boolean_simplifier::BooleanSimplifier));
        pm.add_pass(Box::new(canonicalization::Canonicalization));
        pm.add_pass(Box::new(block_simplifier::BlockSimplifier));
        pm.add_pass(Box::new(dead_branch::DeadBranchElimination));
        pm
    }

    /// Runs all passes on the program, iterating until no more changes
    /// or max_iterations is reached. Returns total number of changes.
    pub fn run(&self, program: &mut Program) -> usize {
        let mut total_changes = 0;

        for _round in 0..self.max_iterations {
            let mut round_changes = 0;

            for pass in &self.passes {
                for item_node in &mut program.items {
                    let changed = match &mut item_node.item {
                        Item::FunctionDef(func) => pass.run_on_function(func),
                        Item::Impl(imp) => {
                            let mut c = false;
                            for method in &mut imp.methods {
                                c |= pass.run_on_function(method);
                            }
                            c
                        }
                        _ => false,
                    };
                    if changed {
                        round_changes += 1;
                    }
                }
            }

            total_changes += round_changes;
            if round_changes == 0 {
                break;
            }
        }

        total_changes
    }
}
