use std::collections::{BTreeSet, HashMap, VecDeque};

use inkwell::basic_block::BasicBlock;
use inkwell::module::Module;
use inkwell::values::{InstructionOpcode, Operand};

// Type alias: BTreeSet<String> represents a set of variables (ordered for easy debugging and comparison)
pub type VarSet = BTreeSet<String>;

// ============================================================================
// 1. Data Structures: Representing elements of Control Flow Graph (CFG)
// ============================================================================

#[derive(Debug, Clone)]
pub struct BlockInfo {
    pub id: usize,
    pub def: VarSet,            // Variables defined (assigned) in this block
    pub r#use: VarSet,          // Variables used (read) in this block
    pub successors: Vec<usize>, // List of successor block IDs
}

#[derive(Debug, Clone)]
pub struct Cfg {
    pub blocks: Vec<BlockInfo>,
    pub entry: usize,
    pub exit: usize,
}

pub enum Direction {
    Forward,  // Forward analysis (e.g., Reaching Definitions)
    Backward, // Backward analysis (e.g., Live Variables)
}

pub fn build_dummy_cfg() -> Cfg {
    let blocks = vec![
        BlockInfo {
            id: 0,
            def: Vec::new().into_iter().collect(),
            r#use: Vec::new().into_iter().collect(),
            successors: vec![1,4],
        },
        BlockInfo {
            id: 1,
            def: Vec::new().into_iter().collect(),
            r#use: Vec::new().into_iter().collect(),
            successors: vec![2, 3],
        },
        BlockInfo {
            id: 2,
            def: Vec::new().into_iter().collect(),
            r#use: Vec::new().into_iter().collect(),
            successors: vec![5],
        },
        BlockInfo {
            id: 3,
            def: Vec::new().into_iter().collect(),
            r#use: Vec::new().into_iter().collect(),
            successors: vec![5],
        },
        BlockInfo {
            id: 4,
            def: Vec::new().into_iter().collect(),
            r#use: Vec::new().into_iter().collect(),
            successors: vec![6],
        },
        BlockInfo {
            id: 5,
            def: Vec::new().into_iter().collect(),
            r#use: Vec::new().into_iter().collect(),
            successors: vec![6],
        },
        BlockInfo {
            id: 6,
            def: Vec::new().into_iter().collect(),
            r#use: Vec::new().into_iter().collect(),
            successors: vec![7],
        },
        BlockInfo {
            id: 7,
            def: Vec::new().into_iter().collect(),
            r#use: Vec::new().into_iter().collect(),
            successors: vec![],
        },
    ];

    Cfg {
        blocks,
        entry: 0,
        exit: 7,
    }
}

// ============================================================================
// 2. Trait Definition: Abstract interface for dataflow analysis
// ============================================================================

/// Dataflow Analysis Trait
/// Any concrete dataflow analysis (e.g., Live Variables, Reaching Definitions)
/// must implement this trait
pub trait DataflowAnalysis {
    // Associated type: Represents the state type of the dataflow analysis
    // requires Clone (for copying), Eq (for equality comparison), and Debug (for printing)
    type State: Clone + Eq + std::fmt::Debug;

    fn direction(&self) -> Direction;

    /// Initial state: The state for all blocks before analysis begins
    fn initial_state(&self) -> Self::State;

    /// Boundary state: Special state for entry block (forward) or exit block (backward)
    fn boundary_state(&self) -> Self::State;

    /// Transfer function: Computes new output state from input state
    /// Forward: OUT[block] = transfer(block, IN[block])
    /// Backward: IN[block] = transfer(block, OUT[block])
    fn transfer(&self, block: &BlockInfo, input: &Self::State) -> Self::State;

    /// Meet function: Merges states from multiple predecessors (forward) or
    /// successors (backward) into a single state
    /// Forward: IN[block] = meet(OUT[pred1], OUT[pred2], ...)
    /// Backward: OUT[block] = meet(IN[succ1], IN[succ2], ...)
    fn meet(&self, states: &[Self::State]) -> Self::State;
}

// ============================================================================
// 3. Dataflow Solver: Executes iterative algorithm until reaching fixed point
// ============================================================================

pub struct DataflowSolver<'a, A: DataflowAnalysis> {
    cfg: &'a Cfg,
    analysis: &'a A,
}

impl<'a, A: DataflowAnalysis> DataflowSolver<'a, A> {
    pub fn new(cfg: &'a Cfg, analysis: &'a A) -> Self {
        Self { cfg, analysis }
    }

    /// Core solving algorithm: Uses worklist algorithm for iterative solving
    /// Returns the final state for each block (OUT for forward, IN for backward)
    pub fn solve(&self) -> Vec<A::State> {
        let n = self.cfg.blocks.len(); // Number of basic blocks

        // Initialize IN and OUT states for all blocks
        let mut in_states: Vec<A::State> = vec![self.analysis.initial_state(); n];
        let mut out_states: Vec<A::State> = vec![self.analysis.initial_state(); n];
        let boundary = self.analysis.boundary_state();

        // Set boundary state based on analysis direction
        // Forward: entry block's IN = boundary
        // Backward: exit block's OUT = boundary
        match self.analysis.direction() {
            Direction::Forward => {
                in_states[self.cfg.entry] = boundary.clone();
                out_states[self.cfg.entry] = self
                    .analysis
                    .transfer(&self.cfg.blocks[self.cfg.entry], &boundary);
            }
            Direction::Backward => {
                out_states[self.cfg.exit] = boundary.clone();
                in_states[self.cfg.exit] = self
                    .analysis
                    .transfer(&self.cfg.blocks[self.cfg.exit], &boundary);
            }
        }

        // Initialize worklist: contains all basic blocks
        let mut worklist: VecDeque<usize> = VecDeque::new();
        for i in 0..n {
            worklist.push_back(i);
        }

        // Iterate until worklist is empty
        while let Some(block_id) = worklist.pop_front() {
            let block = &self.cfg.blocks[block_id];

            match self.analysis.direction() {
                Direction::Forward => {
                    // ===== Forward Analysis =====
                    // 1. Collect OUT states from all predecessors
                    let mut pred_states: Vec<A::State> = Vec::new();
                    for i in 0..n {
                        if i != block_id && self.cfg.blocks[i].successors.contains(&block_id) {
                            pred_states.push(out_states[i].clone());
                        }
                    }
                    if block_id == self.cfg.entry {
                        pred_states.push(boundary.clone());
                    }

                    if pred_states.is_empty() {
                        continue; // No predecessors, cannot compute
                    }

                    // 2. Meet: IN[block] = meet(OUT[preds])
                    let new_in = self.analysis.meet(&pred_states);
                    in_states[block_id] = new_in.clone();

                    // 3. Transfer: OUT[block] = transfer(block, IN[block])
                    let new_out = self.analysis.transfer(block, &in_states[block_id]);

                    // 4. If changed, add successors to worklist
                    if new_out != out_states[block_id] {
                        out_states[block_id] = new_out;
                        for &succ in &block.successors {
                            if !worklist.contains(&succ) {
                                worklist.push_back(succ);
                            }
                        }
                    }
                }
                Direction::Backward => {
                    // ===== Backward Analysis =====
                    // 1. Collect IN states from all successors
                    let succ_states: Vec<A::State> = block
                        .successors
                        .iter()
                        .map(|&s| in_states[s].clone())
                        .chain(if block_id == self.cfg.exit {
                            vec![boundary.clone()]
                        } else {
                            vec![]
                        })
                        .collect();

                    if succ_states.is_empty() {
                        continue; // No successors, cannot compute
                    }

                    // 2. Meet: OUT[block] = meet(IN[succs])
                    let new_out = self.analysis.meet(&succ_states);
                    out_states[block_id] = new_out.clone();

                    // 3. Transfer: IN[block] = transfer(block, OUT[block])
                    let new_in = self.analysis.transfer(block, &out_states[block_id]);

                    // 4. If changed, add predecessors to worklist
                    if new_in != in_states[block_id] {
                        in_states[block_id] = new_in;
                        for i in 0..n {
                            if i != block_id
                                && self.cfg.blocks[i].successors.contains(&block_id)
                                && !worklist.contains(&i)
                            {
                                worklist.push_back(i);
                            }
                        }
                    }
                }
            }
        }

        // Return final results: OUT for forward, IN for backward
        match self.analysis.direction() {
            Direction::Forward => out_states,
            Direction::Backward => in_states,
        }
    }
}
