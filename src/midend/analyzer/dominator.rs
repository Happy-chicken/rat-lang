use std::collections::{BTreeSet, HashMap};

use super::dataflow::Cfg;

pub fn compute_postorder(cfg: &Cfg) -> Vec<usize> {
    let n = cfg.blocks.len();
    let mut visited = vec![false; n];
    let mut order = Vec::with_capacity(n);

    fn dfs(node: usize, cfg: &Cfg, visited: &mut [bool], order: &mut Vec<usize>) {
        visited[node] = true;
        for &succ in &cfg.blocks[node].successors {
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

/// compute dominator set of each node
/// 
/// Dominator definition: node d dominates node n if every path from entry to n must go through d
/// 
/// - Each node dominates itself (reflexivity)
/// - Entry node dominates all nodes
/// - Dominator relationship forms a tree (dominator tree)
/// 
///   Dom(n) = {n} ∪ (⋂_{p ∈ preds(n)} Dom(p))
///   That is: the dominators of n = n itself + the intersection of all predecessors' dominators
/// 
/// Identifying loop headers, computing dominator trees, performing control flow optimizations
pub fn compute_dominators(cfg: &Cfg) -> Vec<BTreeSet<usize>> {
    let n = cfg.blocks.len();
    let all: BTreeSet<usize> = (0..n).collect();

    let mut dom: Vec<BTreeSet<usize>> = vec![all.clone(); n];
    let mut entry_set = BTreeSet::new();
    entry_set.insert(cfg.entry);
    dom[cfg.entry] = entry_set;

    let mut changed = true;
    while changed {
        changed = false;
        for i in 0..n {
            if i == cfg.entry {
                continue;
            }

            let preds: Vec<usize> = (0..n)
                .filter(|&p| cfg.blocks[p].successors.contains(&i))
                .collect();

            let intersection: BTreeSet<usize> = if preds.is_empty() {
                all.clone()
            } else {
                let mut result = dom[preds[0]].clone();
                for &p in &preds[1..] {
                    result = result.intersection(&dom[p]).copied().collect();
                }
                result
            };

            let mut new_dom = intersection;
            new_dom.insert(i);

            if new_dom != dom[i] {
                dom[i] = new_dom;
                changed = true;
            }
        }
    }

    dom
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
pub fn compute_idom(cfg: &Cfg, dom: &[BTreeSet<usize>]) -> Vec<Option<usize>> {
    let n = cfg.blocks.len();
    let mut idom = vec![None; n];

    for i in 0..n {
        if i == cfg.entry {
            continue;
        }
        let strict_dom: BTreeSet<usize> = dom[i].iter().filter(|&&d| d != i).copied().collect();

        for &d in &strict_dom {
            let dominates_all: bool = strict_dom
                .iter()
                .all(|&other| other == d || dom[other].contains(&d));
            if dominates_all {
                idom[i] = Some(d);
                break;
            }
        }
    }

    idom
}
