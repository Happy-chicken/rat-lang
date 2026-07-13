use std::collections::BTreeSet;

use super::context::FunctionAnalysisData;
use super::dataflow::{BlockInfo, Cfg, DataflowAnalysis, DataflowSolver, Direction};

pub type ExprSet = BTreeSet<String>;

/// check whether a expr string uses a defined variablle
/// if any operands in defs: true (expr is killed)
/// if all operands are not in defs: false (expr is alive)
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

/// Available Expressions Analysis
///
/// An expression x ⊕ y is "available" at a program point if it has been
/// computed on every path to that point and neither x nor y have been
/// redefined since the last computation.
///
/// Dataflow equations (forward, intersection):
///   in[n]  = ⋂ out[p]     for all predecessors p
///   out[n] = gen[n] ∪ (in[n] − kill[n])
///
/// where:
///   gen[n]  = expressions evaluated in n whose operands survive the block
///   kill[n] = all expressions whose operands are redefined in n
///
/// gen/kill are pre-computed from the shared AnalysisContext (one LLVM IR scan).
pub struct AvailableExpressionAnalysis {
    gen_map: Vec<ExprSet>,
    kill_map: Vec<ExprSet>,
}

impl AvailableExpressionAnalysis {
    /// Builds gen/kill maps from the shared analysis context and the CFG.
    /// Uses block_expressions and all_expressions from FunctionAnalysisData
    /// together with per-block def sets from the CFG.
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

impl DataflowAnalysis for AvailableExpressionAnalysis {
    type State = ExprSet;

    fn direction(&self) -> Direction {
        Direction::Forward
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

/// Computes available expressions for each block in the CFG.
/// Returns `out[n]` for each block.
pub fn compute_available_expressions(
    cfg: &Cfg,
    analysis: &AvailableExpressionAnalysis,
) -> Vec<ExprSet> {
    let solver = DataflowSolver::new(cfg, analysis);
    solver.solve()
}
