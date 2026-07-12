use std::collections::{BTreeSet, HashMap};

use inkwell::basic_block::BasicBlock;
use inkwell::module::Module;
use inkwell::values::{InstructionOpcode, Operand};

use super::dataflow::{BlockInfo, Cfg};

fn is_valid_var_name(name: &str) -> bool {
    !name.is_empty() && !name.contains('.') && !name.contains("tmp")
}

fn operand_ptr_name(operand: &Operand) -> Option<String> {
    match operand {
        Operand::Value(val) => {
            let name = val.get_name().to_str().unwrap_or("");
            if is_valid_var_name(name) {
                Some(name.to_string())
            } else {
                None
            }
        }
        Operand::Block(_) => None,
    }
}

fn opcode_to_str(opcode: InstructionOpcode) -> &'static str {
    match opcode {
        InstructionOpcode::Add => "add",
        InstructionOpcode::Sub => "sub",
        InstructionOpcode::Mul => "mul",
        InstructionOpcode::SDiv => "sdiv",
        InstructionOpcode::UDiv => "udiv",
        InstructionOpcode::SRem => "srem",
        InstructionOpcode::And => "and",
        InstructionOpcode::Or => "or",
        InstructionOpcode::Xor => "xor",
        InstructionOpcode::Shl => "shl",
        InstructionOpcode::LShr => "lshr",
        InstructionOpcode::AShr => "ashr",
        InstructionOpcode::ICmp => "icmp",
        InstructionOpcode::FCmp => "fcmp",
        _ => "",
    }
}

fn is_expression_opcode(opcode: InstructionOpcode) -> bool {
    !opcode_to_str(opcode).is_empty()
}

fn build_load_to_alloca_map<'ctx>(bb: &BasicBlock<'ctx>) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for instr in bb.get_instructions() {
        if instr.get_opcode() != InstructionOpcode::Load {
            continue;
        }
        if let Some(load_name_cstr) = instr.get_name() {
            let load_name = load_name_cstr.to_str().unwrap_or("");
            if load_name.is_empty() {
                continue;
            }
            if let Some(ptr_op) = instr.get_operand(0) {
                if let Some(alloca_name) = operand_ptr_name(&ptr_op) {
                    map.insert(load_name.to_string(), alloca_name);
                }
            }
        }
    }
    map
}

fn collect_alloca_names<'ctx>(bb: &BasicBlock<'ctx>) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for instr in bb.get_instructions() {
        if instr.get_opcode() == InstructionOpcode::Alloca {
            if let Some(cstr) = instr.get_name() {
                if let Ok(name) = cstr.to_str() {
                    if is_valid_var_name(name) {
                        names.insert(name.to_string());
                    }
                }
            }
        }
    }
    names
}

fn collect_block_expressions<'ctx>(
    bb: &BasicBlock<'ctx>,
    load_to_alloca: &HashMap<String, String>,
    alloca_names: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut exprs = BTreeSet::new();
    for instr in bb.get_instructions() {
        let opcode = instr.get_opcode();
        if !is_expression_opcode(opcode) {
            continue;
        }
        let name0 = instr.get_operand(0)
            .and_then(|o| resolve_to_alloca(&o, load_to_alloca, alloca_names));
        let name1 = instr.get_operand(1)
            .and_then(|o| resolve_to_alloca(&o, load_to_alloca, alloca_names));
        if let (Some(v0), Some(v1)) = (&name0, &name1) {
            let op_str = opcode_to_str(opcode);
            exprs.insert(format!("{}({},{})", op_str, v0, v1));
        }
    }
    exprs
}

fn resolve_to_alloca(
    operand: &Operand,
    load_to_alloca: &HashMap<String, String>,
    alloca_names: &BTreeSet<String>,
) -> Option<String> {
    match operand {
        Operand::Value(val) => {
            let name = val.get_name().to_str().unwrap_or("");
            if alloca_names.contains(name) {
                return Some(name.to_string());
            }
            load_to_alloca.get(name).cloned()
        }
        Operand::Block(_) => None,
    }
}

/// Per-function analysis data extracted from LLVM IR
pub struct FunctionAnalysisData {
    pub alloca_names: BTreeSet<String>,
    pub load_to_alloca: HashMap<String, String>,
    pub block_expressions: Vec<BTreeSet<String>>,
    pub all_expressions: BTreeSet<String>,
}

/// Shared analysis context built from a single LLVM IR scan.
/// Provides all data needed by live variable, reaching definition,
/// and available expression analyses without re-scanning.
pub struct AnalysisContext {
    pub cfgs: HashMap<String, Cfg>,
    pub func_data: HashMap<String, FunctionAnalysisData>,
}

/// Builds the analysis context by scanning the LLVM IR module once.
/// Extracts CFGs (def/use/successors), alloca names, load-to-alloca mappings,
/// and expression data for every function.
pub fn build_analysis_context<'ctx>(module: &Module<'ctx>) -> AnalysisContext {
    let mut cfgs = HashMap::new();
    let mut func_data = HashMap::new();

    for function in module.get_functions() {
        let fn_name = match function.get_name().to_str() {
            Ok(s) => s.to_string(),
            Err(_) => continue,
        };
        if fn_name.starts_with("llvm.") {
            continue;
        }

        let basic_blocks: Vec<BasicBlock> = function.get_basic_blocks();
        if basic_blocks.is_empty() {
            continue;
        }

        let alloca_names: BTreeSet<String> = basic_blocks
            .iter()
            .flat_map(|bb| collect_alloca_names(bb))
            .collect();

        let load_to_alloca: HashMap<String, String> = basic_blocks
            .iter()
            .flat_map(|bb| build_load_to_alloca_map(bb))
            .collect();

        let block_expressions: Vec<BTreeSet<String>> = basic_blocks
            .iter()
            .map(|bb| collect_block_expressions(bb, &load_to_alloca, &alloca_names))
            .collect();

        let all_expressions: BTreeSet<String> = block_expressions
            .iter()
            .flat_map(|e| e.iter().cloned())
            .collect();

        let bb_to_idx: HashMap<BasicBlock, usize> = basic_blocks
            .iter()
            .enumerate()
            .map(|(i, bb)| (*bb, i))
            .collect();

        let entry_idx = 0;
        let exit_idx = basic_blocks.len();

        let mut blocks = Vec::with_capacity(basic_blocks.len() + 1);

        for (i, bb) in basic_blocks.iter().enumerate() {
            let (def, r#use) = extract_block_def_use(bb, &alloca_names);

            let successors = if let Some(terminator) = bb.get_terminator() {
                get_successor_indices(&terminator, &bb_to_idx)
            } else {
                Vec::new()
            };
            let successors = if successors.is_empty() {
                vec![exit_idx]
            } else {
                successors
            };

            blocks.push(BlockInfo {
                id: i,
                def,
                r#use,
                successors,
            });
        }

        blocks.push(BlockInfo {
            id: exit_idx,
            def: BTreeSet::new(),
            r#use: BTreeSet::new(),
            successors: vec![],
        });

        cfgs.insert(
            fn_name.clone(),
            Cfg {
                blocks,
                entry: entry_idx,
                exit: exit_idx,
            },
        );

        func_data.insert(
            fn_name,
            FunctionAnalysisData {
                alloca_names,
                load_to_alloca,
                block_expressions,
                all_expressions,
            },
        );
    }

    AnalysisContext { cfgs, func_data }
}

// ── helpers duplicated from dataflow.rs (could be shared but kept self-contained) ──

fn extract_block_def_use<'ctx>(
    bb: &BasicBlock<'ctx>,
    alloca_names: &BTreeSet<String>,
) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut def = BTreeSet::new();
    let mut r#use = BTreeSet::new();
    for instr in bb.get_instructions() {
        match instr.get_opcode() {
            InstructionOpcode::Alloca => {
                if let Some(cstr) = instr.get_name() {
                    if let Ok(name) = cstr.to_str() {
                        if is_valid_var_name(name) {
                            def.insert(name.to_string());
                        }
                    }
                }
            }
            InstructionOpcode::Store => {
                if let Some(operand) = instr.get_operand(1) {
                    if let Some(name) = operand_ptr_name(&operand) {
                        if alloca_names.contains(&name) {
                            def.insert(name);
                        }
                    }
                }
                for i in 0..instr.get_num_operands() {
                    if i == 1 {
                        continue;
                    }
                    if let Some(operand) = instr.get_operand(i) {
                        if let Some(name) = operand_ptr_name(&operand) {
                            if alloca_names.contains(&name) {
                                r#use.insert(name);
                            }
                        }
                    }
                }
            }
            _ => {
                for i in 0..instr.get_num_operands() {
                    if let Some(operand) = instr.get_operand(i) {
                        if let Some(name) = operand_ptr_name(&operand) {
                            if alloca_names.contains(&name) {
                                r#use.insert(name);
                            }
                        }
                    }
                }
            }
        }
    }
    (def, r#use)
}

fn get_successor_indices<'ctx>(
    terminator: &inkwell::values::InstructionValue<'ctx>,
    bb_to_idx: &HashMap<BasicBlock<'ctx>, usize>,
) -> Vec<usize> {
    let mut successors = Vec::new();
    for i in 0..terminator.get_num_operands() {
        if let Some(operand) = terminator.get_operand(i) {
            if let Operand::Block(bb) = operand {
                if let Some(&idx) = bb_to_idx.get(&bb) {
                    successors.push(idx);
                }
            }
        }
    }
    successors
}
