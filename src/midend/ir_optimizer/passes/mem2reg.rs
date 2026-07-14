use inkwell::module::Module;
use inkwell::values::{AnyValue, AsValueRef, BasicValue, BasicValueEnum, FunctionValue, InstructionOpcode, InstructionValue, Operand};
use crate::midend::analyzer::context::build_analysis_context;
use crate::midend::analyzer::dominator::{compute_dominators_fast, compute_dom_tree_children, compute_idom_fast, compute_dominance_frontier};

use super::Pass;
use std::collections::{BTreeSet, HashMap};
pub struct Mem2Reg;

impl Pass for Mem2Reg {
    fn name(&self) -> &'static str {
        "mem2reg"
    }

    fn description(&self) -> &'static str {
        "LLVM mem2reg: promotes alloca/load/store to SSA register values"
    }

    // fn run(&self, module: &Module) -> bool {
    //     crate::midend::ir_optimizer::init_native_target();
    //     match crate::midend::ir_optimizer::run_llvm_optimizations(module, "mem2reg") {
    //         Ok(changed) => changed,
    //         Err(e) => {
    //             eprintln!("[mem2reg] failed: {}", e);
    //             false
    //         }
    //     }
    // }
    fn run(&self, module: &Module) -> bool {
        let mut changed = false;

        // Step 1: Compute dominators for the CFG
        let ana_ctx = build_analysis_context(module);
        let cfgs = ana_ctx.cfgs;
        for function in module.get_functions() {
            let cfg = match cfgs.get(function.get_name().to_str().unwrap()) {
                Some(cfg) => cfg,
                None => continue, // Skip functions without CFG
            };
            
            let idom = compute_idom_fast(&cfg);
            let dom = compute_dominators_fast(&cfg, &idom);
            let dom_tree = compute_dom_tree_children(&cfg, &idom);
            let dom_frontier = compute_dominance_frontier(&cfg, &dom);
        }
        changed
    }
}

fn collect_promotable_allocas<'a>(func: &'a FunctionValue) -> Vec<InstructionValue<'a>> {
    let mut allocas = Vec::new();
    for bb in func.get_basic_blocks() {
        for instr in bb.get_instructions() {
            if instr.get_opcode() != InstructionOpcode::Alloca {
                continue;
            }
            // 检查 alloca 是否只被 load/store 使用（且地址未逃逸）
            if is_alloca_promotable(&instr) {
                allocas.push(instr);
            }
        }
    }
    allocas
}

fn is_alloca_promotable(alloca: &InstructionValue) -> bool {
    if alloca.get_opcode() != InstructionOpcode::Alloca {
        return false;
    }
    let use_ = alloca.get_first_use();
    while let Some(u) = use_ {
        let user = u.get_user().as_any_value_enum().into_instruction_value();
        match user.get_opcode() {
            InstructionOpcode::Load => {},
            InstructionOpcode::Store => {
                // 检查 store 的目标是否是该 alloca
                let ptr = user.get_operand(1);
                match ptr {
                    Some(Operand::Value(v)) => {
                        if v != alloca.as_any_value_enum().into_pointer_value() {
                            return false; // store 的目标不是该 alloca
                        }
                    },
                    _ => return false,
                }
            },
            _ => return false, // 其他指令使用 alloca，不能提升
        }
    };
    true
}
