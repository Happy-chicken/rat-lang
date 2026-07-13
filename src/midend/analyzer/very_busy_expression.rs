use std::collections::BTreeSet;

use super::available_expression::ExprSet;
use super::context::FunctionAnalysisData;
use super::dataflow::{BlockInfo, Cfg, DataflowAnalysis, DataflowSolver, Direction};

fn expr_uses_any_def(expr: &str, def: &BTreeSet<String>) -> bool {
    let inner = match expr.find('(') {
        Some(i) => &expr[i + 1..expr.len() - 1],
        None => return false,
    };
    for var in inner.split(',') {
        if def.contains(var) {
            return true;
        }
    }
    false
}

/// Very Busy Expressions Analysis
///
/// An expression is "very busy" at a program point if it MUST be evaluated
/// on EVERY path from that point to the exit, and none of its operands have
/// been redefined since the last evaluation.
///
/// This is the backward dual of Available Expressions.
///
/// Dataflow equations (backward, intersection):
///   out[n] = ⋂ in[s]     for all successors s
///   in[n]  = gen[n] ∪ (out[n] − kill[n])
///
/// where:
///   gen[n]  = expressions evaluated in n whose operands survive the block
///   kill[n] = all expressions whose operands are redefined in n
///
/// gen/kill are pre-computed from the shared AnalysisContext (one LLVM IR scan).
pub struct VeryBusyExpressionAnalysis {
    gen_map: Vec<ExprSet>,
    kill_map: Vec<ExprSet>,
}

impl VeryBusyExpressionAnalysis {
    pub fn from_context(cfg: &Cfg, fdata: &FunctionAnalysisData) -> Self {
        let n = cfg.blocks.len();
        let mut gen_map = vec![ExprSet::new(); n];
        let mut kill_map = vec![ExprSet::new(); n];

        for block in &cfg.blocks {
            let i = block.id;
            let def = &block.def;

            if i < fdata.block_expressions.len() {
                let mut gen_set = ExprSet::new();
                for expr in &fdata.block_expressions[i] {
                    if !expr_uses_any_def(expr, def) {
                        gen_set.insert(expr.clone());
                    }
                }
                gen_map[i] = gen_set;
            }

            let mut kill_set = ExprSet::new();
            for expr in &fdata.all_expressions {
                if expr_uses_any_def(expr, def) {
                    kill_set.insert(expr.clone());
                }
            }
            kill_map[i] = kill_set;
        }

        Self { gen_map, kill_map }
    }
}

impl DataflowAnalysis for VeryBusyExpressionAnalysis {
    type State = ExprSet;

    fn direction(&self) -> Direction {
        Direction::Backward
    }

    fn initial_state(&self) -> Self::State {
        BTreeSet::new()
    }

    fn boundary_state(&self) -> Self::State {
        BTreeSet::new()
    }

    fn transfer(&self, block: &BlockInfo, input: &Self::State) -> Self::State {
        let gen_set = &self.gen_map[block.id];
        let kill_set = &self.kill_map[block.id];

        let mut output = input.clone();
        for e in kill_set {
            output.remove(e);
        }
        for e in gen_set {
            output.insert(e.clone());
        }
        output
    }

    fn meet(&self, states: &[Self::State]) -> Self::State {
        if states.is_empty() {
            return BTreeSet::new();
        }
        let mut result = states[0].clone();
        for state in &states[1..] {
            result = result.intersection(state).cloned().collect();
        }
        result
    }
}

/// Computes very busy expressions for each block in the CFG.
/// Returns `in[n]` for each block.
pub fn compute_very_busy_expressions(
    cfg: &Cfg,
    analysis: &VeryBusyExpressionAnalysis,
) -> Vec<ExprSet> {
    let solver = DataflowSolver::new(cfg, analysis);
    solver.solve()
}
