use std::collections::{BTreeSet, HashMap};

use crate::midend::analyzer::dataflow::DataflowAnalysis;

use super::dataflow::Cfg;

/// Dominator definition: node d dominates node n if every path from entry to n must go through d
struct DominatorAnalysis<'a> {
    cfg: &'a Cfg,
}

impl<'a> DominatorAnalysis<'a> {
    pub fn new(cfg: &'a Cfg) -> Self {
        Self { cfg }
    }
}

impl<'a> DataflowAnalysis for DominatorAnalysis<'a> {
    type State = BTreeSet<usize>;

    fn direction(&self) -> crate::midend::analyzer::dataflow::Direction {
        crate::midend::analyzer::dataflow::Direction::Forward
    }

    fn initial_state(&self) -> Self::State {
        let all: BTreeSet<usize> = (0..self.cfg.size()).collect();
        all
    }

    fn boundary_state(&self) -> Self::State {
        let mut entry_set = BTreeSet::new();
        entry_set.insert(self.cfg.entry);
        entry_set
    }

    fn transfer(&self, block: &crate::midend::analyzer::dataflow::BlockInfo, input: &Self::State) -> Self::State {
        let mut output = input.clone();
        output.insert(block.id);
        output
    }

    ///   Dom(n) = {n} ∪ (⋂_{p ∈ preds(n)} Dom(p))
    ///   That is: the dominators of n = n itself + the intersection of all predecessors' dominators
    fn meet(&self, states: &[Self::State]) -> Self::State {
        if states.is_empty() {
            return BTreeSet::new();
        }
        let mut result = states[0].clone();
        for state in &states[1..] {
            result = result.intersection(state).copied().collect();
        }
        result
    }
}

pub fn compute_dominators(cfg: &Cfg) -> Vec<BTreeSet<usize>> {
    let analysis = DominatorAnalysis::new(cfg);
    let solver = crate::midend::analyzer::dataflow::DataflowSolver::new(cfg, &analysis);
    solver.solve()
}


/// optimization
fn compute_postorder(cfg: &Cfg) -> Vec<usize> {
    let n = cfg.size();
    let mut visited = vec![false; n];
    let mut order = Vec::with_capacity(n);

    fn dfs(node: usize, cfg: &Cfg, visited: &mut [bool], order: &mut Vec<usize>) {
        visited[node] = true;
        for &succ in &cfg.blocks[node].successors {
            if succ == cfg.exit { continue; }
            if !visited[succ] {
                dfs(succ, cfg, visited, order);
            }
        }
        order.push(node);
    }

    dfs(cfg.entry, cfg, &mut visited, &mut order);
    order
}

/// reverse postorder to accelerate convergence of iterative dataflow analysis
pub fn compute_rpo(cfg: &Cfg) -> Vec<usize> {
    let mut po = compute_postorder(cfg);
    po.reverse();
    po
}

/// 路径压缩的交汇操作，在支配者树上寻找两个节点的最近公共支配者。
fn intersect(
    mut finger1: usize,
    mut finger2: usize,
    idom: &[Option<usize>],
    po_idx: &[usize],
) -> usize {
    while finger1 != finger2 {
        // 后序编号小的节点更靠近叶子，需要往上爬
        if po_idx[finger1] < po_idx[finger2] {
            finger1 = idom[finger1].unwrap();
        } else {
            finger2 = idom[finger2].unwrap();
        }
    }
    finger1
}

/// compute immediate dominator (IDom) for each node
/// 
/// Immediate dominator definition: the closest dominator in the dominator tree
/// that is not the node itself
/// 
/// Properties:
/// - Each non-entry node has exactly one immediate dominator
/// - Immediate dominator relationship forms a tree (dominator tree)
/// 
/// In the set of strict dominators, find the node that dominates all other strict dominators
/// That is: idom(n) = the "largest" element in the set of strict dominators
/// 快速计算直接支配者（基于 RPO + intersect）
pub fn compute_idom_fast(cfg: &Cfg) -> Vec<Option<usize>> {
    let n = cfg.size();
    let entry = cfg.entry;

    let postorder = compute_postorder(cfg);
    let mut po_idx = vec![0; n];
    for (i, &node) in postorder.iter().enumerate() {
        po_idx[node] = i;
    }

    let rpo = compute_rpo(cfg);
    let mut idom: Vec<Option<usize>> = vec![None; n];
    idom[entry] = Some(entry);

    let mut changed = true;
    while changed {
        changed = false;
        for &b in &rpo {
            if b == entry { continue; }
            let preds: Vec<usize> = (0..n)
                .filter(|&p| cfg.blocks[p].successors.contains(&b))
                .collect();

            let mut new_idom = None;
            for &p in &preds {
                if idom[p].is_some() { new_idom = Some(p); break; }
            }
            if let Some(mut curr_idom) = new_idom {
                for &p in &preds {
                    if Some(p) == new_idom { continue; }
                    if idom[p].is_some() {
                        curr_idom = intersect(curr_idom, p, &idom, &po_idx);
                    }
                }
                if idom[b] != Some(curr_idom) {
                    idom[b] = Some(curr_idom);
                    changed = true;
                }
            }
        }
    }
    idom
}

/// 从 idom 构建完整支配集
pub fn compute_dominators_fast(cfg: &Cfg, idom: &[Option<usize>]) -> Vec<BTreeSet<usize>> {
    let n = cfg.size();
    let entry = cfg.entry;
    let mut dom = vec![BTreeSet::new(); n];
    dom[entry].insert(entry);

    for i in 0..n {
        if i == entry || idom[i].is_none() { continue; }
        dom[i].insert(i);
        let mut curr = i;
        while curr != entry {
            if let Some(p) = idom[curr] {
                if p == curr { break; }
                dom[i].insert(p);
                curr = p;
            } else { break; }
        }
    }

    let all: BTreeSet<usize> = (0..n).collect();
    for i in 0..n {
        if i != entry && idom[i].is_none() {
            dom[i] = all.clone();
        }
    }
    dom
}


/// compute dominance frontier for each node
///
/// DF[n] = set of nodes x where n dominates a predecessor of x
///         but n does NOT strictly dominate x.
///
/// Equivalently: x ∈ DF[n] iff ∃p ∈ preds(x) such that n dominates p
///               but n does not dominate x (except possibly x = n).
pub fn compute_dominance_frontier(cfg: &Cfg, dom: &[BTreeSet<usize>]) -> Vec<BTreeSet<usize>> {
    let n = cfg.size();
    let mut df = vec![BTreeSet::new(); n];

    for x in 0..n {
        let preds: Vec<usize> = (0..n)
            .filter(|&p| cfg.blocks[p].successors.contains(&x))
            .collect();
        if preds.len() < 2 {
            continue;
        }
        for &p in &preds {
            let mut runner = p;
            while !dom[runner].contains(&x) || runner == x {
                if runner == cfg.entry {
                    break;
                }
                // runner is not in dom[x], so x is in DF[runner]
                df[runner].insert(x);
                // find idom of runner (the one strict dominator that dominates all others)
                let strict: BTreeSet<usize> =
                    dom[runner].iter().filter(|&&d| d != runner).copied().collect();
                let prev = runner;
                runner = strict
                    .iter()
                    .find(|&&d| strict.iter().all(|&o| o == d || dom[o].contains(&d)))
                    .copied()
                    .unwrap_or(cfg.entry);
                if runner == prev {
                    break;
                }
            }
        }
    }

    df
}

/// compute iterated dominance frontier
/// IDF(S) = least fixpoint of DF(S ∪ IDF(S))
pub fn compute_iterated_dominance_frontier(
    cfg: &Cfg,
    dom: &[BTreeSet<usize>],
    initial: &BTreeSet<usize>,
) -> BTreeSet<usize> {
    let df = compute_dominance_frontier(cfg, dom);
    let mut result = initial.clone();
    let mut worklist: Vec<usize> = initial.iter().copied().collect();

    while let Some(b) = worklist.pop() {
        for &f in &df[b] {
            if result.insert(f) {
                worklist.push(f);
            }
        }
    }

    result
}

/// build dominator tree children from immediate dominators
pub fn compute_dom_tree_children(cfg: &Cfg, idom: &[Option<usize>]) -> Vec<Vec<usize>> {
    let n = cfg.size();
    let mut children = vec![Vec::new(); n];
    for i in 0..n {
        if i == cfg.entry {
            continue;
        }
        if let Some(parent) = idom[i] {
            children[parent].push(i);
        }
    }
    children
}
