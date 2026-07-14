use std::collections::HashMap;

use inkwell::module::Module;
use inkwell::values::{InstructionOpcode, Operand};

use super::Pass;
use crate::midend::analyzer::dataflow;
use crate::midend::analyzer::dominator;

fn operand_key(op: &Operand) -> Option<String> {
    match op {
        Operand::Value(v) => match *v {
            inkwell::values::BasicValueEnum::IntValue(iv) => {
                if let Some(c) = iv.get_zero_extended_constant() {
                    return Some(format!("{}", c as i64));
                }
                let name = iv.get_name().to_str().unwrap_or("");
                if !name.is_empty() {
                    return Some(name.to_string());
                }
                Some(format!("{}", iv))
            }
            inkwell::values::BasicValueEnum::FloatValue(fv) => {
                if let Some((val, _)) = fv.get_constant() {
                    return Some(format!("{:.6}", val));
                }
                let name = fv.get_name().to_str().unwrap_or("");
                if !name.is_empty() {
                    return Some(name.to_string());
                }
                Some(format!("{}", fv))
            }
            _ => None,
        },
        _ => None,
    }
}

fn opcode_name(opcode: InstructionOpcode) -> Option<&'static str> {
    match opcode {
        InstructionOpcode::Add => Some("add"),
        InstructionOpcode::Sub => Some("sub"),
        InstructionOpcode::Mul => Some("mul"),
        InstructionOpcode::SDiv => Some("sdiv"),
        InstructionOpcode::FAdd => Some("fadd"),
        InstructionOpcode::FSub => Some("fsub"),
        InstructionOpcode::FMul => Some("fmul"),
        InstructionOpcode::FDiv => Some("fdiv"),
        InstructionOpcode::And => Some("and"),
        InstructionOpcode::Or => Some("or"),
        InstructionOpcode::Xor => Some("xor"),
        _ => None,
    }
}

fn make_expr_key(instr: &inkwell::values::InstructionValue) -> Option<String> {
    let op_str = opcode_name(instr.get_opcode())?;
    let lhs = operand_key(&instr.get_operand(0)?)?;
    let rhs = operand_key(&instr.get_operand(1)?)?;
    Some(format!("{}({},{})", op_str, lhs, rhs))
}

fn build_ssa_cfg(func: &inkwell::values::FunctionValue) -> (dataflow::Cfg, usize) {
    use std::collections::BTreeSet;
    use inkwell::basic_block::BasicBlock;

    let basic_blocks: Vec<BasicBlock> = func.get_basic_blocks();
    let n = basic_blocks.len();
    let exit_idx = n;

    let bb_to_idx: HashMap<BasicBlock, usize> = basic_blocks
        .iter()
        .enumerate()
        .map(|(i, bb)| (*bb, i))
        .collect();

    let mut blocks = Vec::with_capacity(n + 1);

    for (i, bb) in basic_blocks.iter().enumerate() {
        let mut def = BTreeSet::new();
        for instr in bb.get_instructions() {
            if let Some(name) = instr.get_name() {
                if let Ok(n) = name.to_str() {
                    if !n.is_empty() {
                        def.insert(n.to_string());
                    }
                }
            }
        }

        let successors = if let Some(terminator) = bb.get_terminator() {
            let mut succs = Vec::new();
            for i in 0..terminator.get_num_operands() {
                if let Some(Operand::Block(succ_bb)) = terminator.get_operand(i) {
                    if let Some(&idx) = bb_to_idx.get(&succ_bb) {
                        succs.push(idx);
                    }
                }
            }
            if succs.is_empty() {
                vec![exit_idx]
            } else {
                succs
            }
        } else {
            vec![exit_idx]
        };

        blocks.push(dataflow::BlockInfo {
            id: i,
            def,
            r#use: BTreeSet::new(),
            successors,
        });
    }

    blocks.push(dataflow::BlockInfo {
        id: exit_idx,
        def: BTreeSet::new(),
        r#use: BTreeSet::new(),
        successors: vec![],
    });

    let cfg = dataflow::Cfg {
        blocks,
        entry: 0,
        exit: exit_idx,
    };

    (cfg, n)
}

pub struct CommonSubexpressionElimination;

impl Pass for CommonSubexpressionElimination {
    fn name(&self) -> &'static str {
        "cse"
    }

    fn description(&self) -> &'static str {
        "global CSE on SSA: dominator-tree value propagation"
    }

    fn run(&self, module: &Module) -> bool {
        let mut changed = false;

        for func in module.get_functions() {
            let fn_name = match func.get_name().to_str() {
                Ok(s) => s.to_string(),
                Err(_) => continue,
            };
            if fn_name.starts_with("llvm.") {
                continue;
            }

            let (cfg, num_real_blocks) = build_ssa_cfg(&func);
            // let dom = dominator::compute_dominators(&cfg);
            let idom = dominator::compute_idom_fast(&cfg);
            let rpo = dominator::compute_rpo(&cfg);

            let bb_list = func.get_basic_blocks();
            let mut block_tables: Vec<HashMap<String, inkwell::values::IntValue>> =
                vec![HashMap::new(); cfg.blocks.len()];

            for &b in &rpo {
                if b == cfg.exit || b >= num_real_blocks {
                    continue;
                }

                let mut table: HashMap<String, inkwell::values::IntValue> = if let Some(id) = idom[b]
                {
                    block_tables[id].clone()
                } else {
                    HashMap::new()
                };

                if let Some(bb) = bb_list.get(b) {
                    let instructions: Vec<_> = bb.get_instructions().collect();
                    for instr in &instructions {
                        let key = match make_expr_key(instr) {
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
