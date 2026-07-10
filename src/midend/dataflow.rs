use std::collections::{BTreeSet, HashMap, VecDeque};

use crate::frontend::ast::expr::{Expr, ExprNode};
use crate::frontend::ast::item::Item;
use crate::frontend::ast::stmt::{Stmt, StmtNode};
use crate::frontend::ast::Program;

pub type VarSet = BTreeSet<String>;

#[derive(Debug, Clone)]
pub struct BlockInfo {
    pub id: usize,
    pub def: VarSet,
    pub r#use: VarSet,
    pub successors: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct Cfg {
    pub blocks: Vec<BlockInfo>,
    pub entry: usize,
    pub exit: usize,
}

pub enum Direction {
    Forward,
    Backward,
}

pub trait DataflowAnalysis {
    type State: Clone + Eq + std::fmt::Debug;

    fn direction(&self) -> Direction;
    fn initial_state(&self) -> Self::State;
    fn boundary_state(&self) -> Self::State;
    fn transfer(&self, block: &BlockInfo, input: &Self::State) -> Self::State;
    fn meet(&self, states: &[Self::State]) -> Self::State;
}

pub struct DataflowSolver<'a, A: DataflowAnalysis> {
    cfg: &'a Cfg,
    analysis: &'a A,
}

impl<'a, A: DataflowAnalysis> DataflowSolver<'a, A> {
    pub fn new(cfg: &'a Cfg, analysis: &'a A) -> Self {
        Self { cfg, analysis }
    }

    pub fn solve(&self) -> Vec<A::State> {
        let n = self.cfg.blocks.len();
        let mut in_states: Vec<A::State> = vec![self.analysis.initial_state(); n];
        let mut out_states: Vec<A::State> = vec![self.analysis.initial_state(); n];
        let boundary = self.analysis.boundary_state();

        match self.analysis.direction() {
            Direction::Forward => {
                in_states[self.cfg.entry] = boundary.clone();
                out_states[self.cfg.entry] =
                    self.analysis
                        .transfer(&self.cfg.blocks[self.cfg.entry], &boundary);
            }
            Direction::Backward => {
                out_states[self.cfg.exit] = boundary.clone();
                in_states[self.cfg.exit] =
                    self.analysis
                        .transfer(&self.cfg.blocks[self.cfg.exit], &boundary);
            }
        }

        let mut worklist: VecDeque<usize> = VecDeque::new();
        for i in 0..n {
            worklist.push_back(i);
        }

        while let Some(block_id) = worklist.pop_front() {
            let block = &self.cfg.blocks[block_id];

            match self.analysis.direction() {
                Direction::Forward => {
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
                        continue;
                    }
                    let new_in = self.analysis.meet(&pred_states);
                    if new_in != in_states[block_id] {
                        in_states[block_id] = new_in.clone();
                        let new_out = self.analysis.transfer(block, &new_in);
                        if new_out != out_states[block_id] {
                            out_states[block_id] = new_out;
                            for &succ in &block.successors {
                                if !worklist.contains(&succ) {
                                    worklist.push_back(succ);
                                }
                            }
                        }
                    }
                }
                Direction::Backward => {
                    let succ_states: Vec<A::State> = block
                        .successors
                        .iter()
                        .map(|&s| in_states[s].clone())
                        .chain(
                            if block_id == self.cfg.exit {
                                vec![boundary.clone()]
                            } else {
                                vec![]
                            },
                        )
                        .collect();
                    if succ_states.is_empty() {
                        continue;
                    }
                    let new_out = self.analysis.meet(&succ_states);
                    if new_out != out_states[block_id] {
                        out_states[block_id] = new_out.clone();
                        let new_in = self.analysis.transfer(block, &new_out);
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
        }

        match self.analysis.direction() {
            Direction::Forward => out_states,
            Direction::Backward => in_states,
        }
    }
}

fn collect_vars(expr: &Expr) -> VarSet {
    let mut vars = BTreeSet::new();
    match expr {
        Expr::Variable(name) => {
            vars.insert(name.clone());
        }
        Expr::Binary { lhs, rhs, .. } => {
            vars.extend(collect_vars(&lhs.expr));
            vars.extend(collect_vars(&rhs.expr));
        }
        Expr::Unary { expr, .. } => {
            vars.extend(collect_vars(&expr.expr));
        }
        Expr::Call { callee, args } => {
            vars.extend(collect_vars(&callee.expr));
            for arg in args {
                vars.extend(collect_vars(&arg.expr));
            }
        }
        Expr::Member { object, .. } => {
            vars.extend(collect_vars(&object.expr));
        }
        Expr::Assign { target, value } => {
            if let Expr::Variable(name) = &target.expr {
                vars.insert(name.clone());
            }
            vars.extend(collect_vars(&value.expr));
        }
        Expr::List { elements } => {
            for e in elements {
                vars.extend(collect_vars(&e.expr));
            }
        }
        Expr::Index { object, index } => {
            vars.extend(collect_vars(&object.expr));
            vars.extend(collect_vars(&index.expr));
        }
        _ => {}
    }
    vars
}

fn analyse_stmt(stmt: &StmtNode, def: &mut VarSet, r#use: &mut VarSet) {
    match &stmt.stmt {
        Stmt::VarDef { name, init, .. } => {
            if let Some(init_expr) = init {
                analyse_expr(init_expr, def, r#use);
            }
            def.insert(name.clone());
        }
        Stmt::Return(Some(expr)) => {
            analyse_expr(expr, def, r#use);
        }
        Stmt::Return(None) => {}
        Stmt::ExprStmt(expr) => {
            analyse_expr(expr, def, r#use);
        }
        Stmt::BlockStmt(block) => {
            for s in &block.stmts {
                analyse_stmt(s, def, r#use);
            }
        }
        Stmt::If {
            condition,
            then_branch,
            elif_branch,
            else_branch,
        } => {
            analyse_expr(condition, def, r#use);
            for stmt in &then_branch.stmts {
                analyse_stmt(stmt, def, r#use);
            }
            for (cond, branch) in elif_branch {
                analyse_expr(cond, def, r#use);
                for stmt in &branch.stmts {
                    analyse_stmt(stmt, def, r#use);
                }
            }
            for stmt in &else_branch.stmts {
                analyse_stmt(stmt, def, r#use);
            }
        }
        Stmt::Loop { condition, body } => {
            analyse_expr(condition, def, r#use);
            for stmt in &body.stmts {
                analyse_stmt(stmt, def, r#use);
            }
        }
        Stmt::Break | Stmt::Continue => {}
    }
}

fn analyse_expr(expr: &ExprNode, def: &mut VarSet, r#use: &mut VarSet) {
    match &expr.expr {
        Expr::Assign { target, value } => {
            r#use.extend(collect_vars(&value.expr));
            if let Expr::Variable(name) = &target.expr {
                def.insert(name.clone());
            } else {
                r#use.extend(collect_vars(&target.expr));
            }
        }
        _ => {
            r#use.extend(collect_vars(&expr.expr));
        }
    }
}

pub fn build_cfg(program: &Program) -> HashMap<String, Cfg> {
    let mut cfgs = HashMap::new();

    for item_node in &program.items {
        if let Item::FunctionDef(func) = &item_node.item {
            let mut def = BTreeSet::new();
            let mut r#use = BTreeSet::new();

            for param in &func.function_header.params {
                def.insert(param.name.clone());
            }

            for stmt in &func.body.stmts {
                analyse_stmt(stmt, &mut def, &mut r#use);
            }

            let block = BlockInfo {
                id: 0,
                def,
                r#use,
                successors: vec![],
            };

            cfgs.insert(
                func.function_header.name.clone(),
                Cfg {
                    blocks: vec![block],
                    entry: 0,
                    exit: 0,
                },
            );
        }
    }

    cfgs
}

pub struct LiveVariableAnalysis;

impl DataflowAnalysis for LiveVariableAnalysis {
    type State = VarSet;

    fn direction(&self) -> Direction {
        Direction::Backward
    }

    fn initial_state(&self) -> Self::State {
        BTreeSet::new()
    }

    fn boundary_state(&self) -> Self::State {
        BTreeSet::new()
    }

    fn transfer(&self, block: &BlockInfo, output: &Self::State) -> Self::State {
        let mut input = output.clone();
        for v in &block.def {
            input.remove(v);
        }
        for v in &block.r#use {
            input.insert(v.clone());
        }
        input
    }

    fn meet(&self, states: &[Self::State]) -> Self::State {
        let mut result = BTreeSet::new();
        for s in states {
            result.extend(s.iter().cloned());
        }
        result
    }
}

pub fn compute_live_variables(cfg: &Cfg) -> Vec<VarSet> {
    let analysis = LiveVariableAnalysis;
    let solver = DataflowSolver::new(cfg, &analysis);
    solver.solve()
}
