use std::collections::HashMap;

use inkwell::module::Module;
use inkwell::values::{InstructionOpcode, Operand};

use super::Pass;
use crate::midend::analyzer::available_expression::{self, ExprSet};
use crate::midend::analyzer::context::build_analysis_context;
use crate::midend::analyzer::dataflow::DataflowSolver;
use crate::midend::analyzer::dominator;

fn resolve_to_alloca_name(
    operand: &Operand,
    load_map: &HashMap<String, String>,
) -> Option<String> {
    match operand {
        Operand::Value(v) => {
            let name = v.get_name().to_str().unwrap_or("");
            if name.is_empty() {
                return None;
            }
            load_map.get(name).cloned()
        }
        _ => None,
    }
}

fn make_expr_key(
    instr: &inkwell::values::InstructionValue,
    load_map: &HashMap<String, String>,
) -> Option<String> {
    let op_str = match instr.get_opcode() {
        InstructionOpcode::Add => "add",
        InstructionOpcode::Sub => "sub",
        InstructionOpcode::Mul => "mul",
        InstructionOpcode::SDiv => "sdiv",
        InstructionOpcode::And => "and",
        InstructionOpcode::Or => "or",
        InstructionOpcode::Xor => "xor",
        _ => return None,
    };
    let lhs = resolve_to_alloca_name(&instr.get_operand(0)?, load_map)?;
    let rhs = resolve_to_alloca_name(&instr.get_operand(1)?, load_map)?;
    Some(format!("{}({},{})", op_str, lhs, rhs))
}

fn compute_avail_in(cfg: &crate::midend::analyzer::dataflow::Cfg, avail_out: &[ExprSet]) -> Vec<ExprSet> {
    let n = cfg.blocks.len();
    let mut avail_in = vec![ExprSet::new(); n];

    for b in 0..n {
        if b == cfg.entry {
            continue;
        }
        let pred_out: Vec<&ExprSet> = (0..n)
            .filter(|&p| cfg.blocks[p].successors.contains(&b))
            .map(|p| &avail_out[p])
            .collect();
        if pred_out.is_empty() {
            continue;
        }
        let mut result = pred_out[0].clone();
        for s in &pred_out[1..] {
            result = result.intersection(s).cloned().collect();
        }
        avail_in[b] = result;
    }

    avail_in
}

pub struct CommonSubexpressionElimination;

impl Pass for CommonSubexpressionElimination {
    fn name(&self) -> &'static str {
        "cse"
    }

    fn description(&self) -> &'static str {
        "global CSE: dominator-tree propagation filtered by available expressions"
    }

    fn run(&self, module: &Module) -> bool {
        let analysis_ctx = build_analysis_context(module);
        let mut changed = false;

        for func in module.get_functions() {
            let fn_name = match func.get_name().to_str() {
                Ok(s) => s.to_string(),
                Err(_) => continue,
            };
            if fn_name.starts_with("llvm.") {
                continue;
            }
            let cfg = match analysis_ctx.cfgs.get(&fn_name) {
                Some(c) => c,
                None => continue,
            };
            let fdata = match analysis_ctx.func_data.get(&fn_name) {
                Some(d) => d,
                None => continue,
            };

            let avail = available_expression::AvailableExpressionAnalysis::from_context(cfg, fdata);
            let avail_out = DataflowSolver::new(cfg, &avail).solve();
            let avail_in = compute_avail_in(cfg, &avail_out);
            let dom = dominator::compute_dominators(cfg);
            let idom = dominator::compute_idom(cfg, &dom);
            let rpo = dominator::compute_rpo(cfg);

            let bb_list = func.get_basic_blocks();
            let mut block_tables: Vec<HashMap<String, inkwell::values::IntValue>> =
                vec![HashMap::new(); cfg.blocks.len()];

            for &b in &rpo {
                if b == cfg.exit || b >= bb_list.len() {
                    continue;
                }

                let mut table: HashMap<String, inkwell::values::IntValue> = if let Some(id) = idom[b]
                {
                    block_tables[id].clone()
                } else {
                    HashMap::new()
                };

                table.retain(|expr, _| b < avail_in.len() && avail_in[b].contains(expr));

                if let Some(bb) = bb_list.get(b) {
                    let instructions: Vec<_> = bb.get_instructions().collect();
                    for instr in &instructions {
                        let key = match make_expr_key(instr, &fdata.load_to_alloca) {
                            Some(k) => k,
                            None => continue,
                        };

                        if let Some(&prev) = table.get(&key) {
                            if let Ok(iv) = inkwell::values::IntValue::try_from(*instr) {
                                iv.replace_all_uses_with(prev);
                                changed = true;
                            }
                        } else if let Ok(iv) = inkwell::values::IntValue::try_from(*instr) {
                            table.insert(key, iv);
                        }
                    }
                }

                block_tables[b] = table;
            }
        }

        changed
    }
}
