use crate::common::DiagCtxt;
use std::collections::{BTreeSet, HashMap};

// Type alias: BTreeSet<String> represents a set of variables (ordered for easy debugging and comparison)
pub type VarSet = BTreeSet<String>;
use super::dataflow::{BlockInfo, Cfg, DataflowAnalysis, DataflowSolver, Direction};

/// Live Variable Analysis
/// IN[block] = (OUT[block] - def[block]) ∪ use[block]
/// OUT[block] = ∪ IN[successor] for all successors

/// In backward dataflow analysis, the live variable set at each program point
/// represents "variables that will be used later in execution"
pub struct LiveVariableAnalysis;

impl DataflowAnalysis for LiveVariableAnalysis {
    // State type is VarSet (set of variables)
    type State = VarSet;

    // Live variable analysis is backward analysis
    fn direction(&self) -> Direction {
        Direction::Backward
    }

    // Initial state is empty set
    fn initial_state(&self) -> Self::State {
        VarSet::new()
    }

    // Boundary state (exit block) is empty set
    // i.e., no variables are live at program exit
    fn boundary_state(&self) -> Self::State {
        VarSet::new()
    }

    /// Transfer function: Computes live variables at block entry
    /// IN[block] = (OUT[block] - def[block]) ∪ use[block]
    ///
    /// Explanation:
    /// - If a variable is defined (assigned) in the block, it is NOT live at entry
    ///   (unless it was used before the definition, but we use the classic formula)
    /// - If a variable is used in the block, it IS live at entry
    fn transfer(&self, block: &BlockInfo, output: &Self::State) -> Self::State {
        let mut input = output.clone();

        // Remove variables that are defined (killed) in this block
        for v in &block.def {
            input.remove(v);
        }

        // Add variables that are used in this block
        for v in &block.r#use {
            input.insert(v.clone());
        }

        input
    }

    /// Meet function: Union
    /// OUT[block] = ∪ IN[successor] for all successors
    /// i.e., a variable is live at block exit if it's live in at least one successor
    fn meet(&self, states: &[Self::State]) -> Self::State {
        let mut result = VarSet::new();
        for s in states {
            result.extend(s.iter().cloned());
        }
        result
    }
}

/// Computes live variable sets for each block in the CFG
/// Returns the entry live variables for each block (since it's backward analysis)
pub fn compute_live_variables(cfg: &Cfg) -> Vec<VarSet> {
    let analysis = LiveVariableAnalysis;
    let solver = DataflowSolver::new(cfg, &analysis);
    solver.solve()
}

/// Uses live variable analysis to detect variables that are defined but never used.
///
/// For each function, runs the backward dataflow analysis to compute `live_in` per block.
/// A variable is "unused" if it appears in some block's `def` set but never appears
/// in any block's `live_in` set — meaning no execution path ever reads its value.
pub fn detect_unused_variables(cfgs: &HashMap<String, Cfg>, diag: &mut DiagCtxt) {
    for (fn_name, cfg) in cfgs {
        let live_in = compute_live_variables(cfg);

        let mut defined: VarSet = BTreeSet::new();
        let mut live_anywhere: VarSet = BTreeSet::new();

        for (i, block) in cfg.blocks.iter().enumerate() {
            for v in &block.def {
                defined.insert(v.clone());
            }
            for v in &live_in[i] {
                live_anywhere.insert(v.clone());
            }
        }

        for var in defined.difference(&live_anywhere) {
            let warn = diag
                .warn(format!(
                    "unused variable `{}` in function `{}`",
                    var, fn_name
                ))
                .build();
            diag.emit(warn);
        }
    }
}
