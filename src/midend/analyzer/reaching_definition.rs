use std::collections::{BTreeSet, HashMap};

use super::dataflow::{BlockInfo, Cfg, DataflowAnalysis, DataflowSolver, Direction};
use super::dataflow::VarSet;

/// Maps variable name → set of block IDs where that variable was last defined.
pub type DefMap = HashMap<String, BTreeSet<usize>>;

/// Reaching Definitions Analysis
///
/// For each program point, computes which definitions of each variable
/// may reach that point along some execution path.
///
/// Dataflow equations (forward, union):
///   in[n]  = ⋃ out[p]     for all predecessors p
///   out[n] = gen[n] ∪ (in[n] − kill[n])
///
/// where:
///   gen[n]  = {(v, n) | v ∈ def[n]}        — definitions generated in this block
///   kill[n] = {v | v ∈ def[n]}             — variables whose old defs are killed
///
/// Uses explicit pre-computed gen/kill maps for consistency with other analyses.
pub struct ReachingDefinitionAnalysis {
    gen_map: Vec<DefMap>,
    kill_vars: Vec<VarSet>,
}

impl ReachingDefinitionAnalysis {
    /// Builds the analysis from a CFG by pre-computing gen/kill sets.
    /// gen[n]: for each v ∈ def[n], maps v → {n}
    /// kill_vars[n]: the set of variables defined in block n (their old defs are killed)
    pub fn from_cfg(cfg: &Cfg) -> Self {
        let n = cfg.blocks.len();
        let mut gen_map = vec![DefMap::new(); n];
        let mut kill_vars = vec![VarSet::new(); n];

        for block in &cfg.blocks {
            let mut g = DefMap::new();
            let mut k = VarSet::new();
            for v in &block.def {
                let mut set = BTreeSet::new();
                set.insert(block.id);
                g.insert(v.clone(), set);
                k.insert(v.clone());
            }
            gen_map[block.id] = g;
            kill_vars[block.id] = k;
        }

        Self { gen_map, kill_vars }
    }
}

impl DataflowAnalysis for ReachingDefinitionAnalysis {
    type State = DefMap;

    fn direction(&self) -> Direction {
        Direction::Forward
    }

    fn initial_state(&self) -> Self::State {
        HashMap::new()
    }

    fn boundary_state(&self) -> Self::State {
        HashMap::new()
    }

    fn transfer(&self, block: &BlockInfo, input: &Self::State) -> Self::State {
        let mut output = input.clone();

        for v in &self.kill_vars[block.id] {
            output.remove(v);
        }
        for (v, blocks) in &self.gen_map[block.id] {
            output.insert(v.clone(), blocks.clone());
        }

        output
    }

    fn meet(&self, states: &[Self::State]) -> Self::State {
        let mut result = HashMap::new();
        for state in states {
            for (v, blocks) in state {
                result
                    .entry(v.clone())
                    .or_insert_with(BTreeSet::new)
                    .extend(blocks.iter().copied());
            }
        }
        result
    }
}

/// Computes reaching definitions for each block in the CFG.
/// Returns `out[n]` for each block (definitions reaching the block exit).
pub fn compute_reaching_definitions(cfg: &Cfg) -> Vec<DefMap> {
    let analysis = ReachingDefinitionAnalysis::from_cfg(cfg);
    let solver = DataflowSolver::new(cfg, &analysis);
    solver.solve()
}
