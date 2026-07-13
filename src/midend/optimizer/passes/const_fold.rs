use inkwell::module::Module;
use inkwell::values::{BasicValueEnum, InstructionOpcode, Operand};

use super::Pass;

fn get_const_i64(operand: &Operand) -> Option<i64> {
    match operand {
        Operand::Value(v) => match *v {
            BasicValueEnum::IntValue(iv) => iv.get_zero_extended_constant().map(|c| c as i64),
            _ => None,
        },
        _ => None,
    }
}

fn try_fold_int(instr: &inkwell::values::InstructionValue) -> Option<i64> {
    let lhs = get_const_i64(&instr.get_operand(0)?)?;
    let rhs = get_const_i64(&instr.get_operand(1)?)?;

    let result = match instr.get_opcode() {
        InstructionOpcode::Add => lhs.wrapping_add(rhs),
        InstructionOpcode::Sub => lhs.wrapping_sub(rhs),
        InstructionOpcode::Mul => lhs.wrapping_mul(rhs),
        InstructionOpcode::SDiv if rhs != 0 => lhs / rhs,
        InstructionOpcode::And => lhs & rhs,
        InstructionOpcode::Or  => lhs | rhs,
        InstructionOpcode::Xor => lhs ^ rhs,
        _ => return None,
    };

    Some(result)
}

pub struct ConstantFolding;

impl Pass for ConstantFolding {
    fn name(&self) -> &'static str {
        "const-fold"
    }

    fn description(&self) -> &'static str {
        "evaluates constant expressions at compile time"
    }

    fn run(&self, module: &Module) -> bool {
        let mut changed = false;

        for func in module.get_functions() {
            for bb in func.get_basic_blocks() {
                let instructions: Vec<_> = bb.get_instructions().collect();
                for instr in &instructions {
                    if let Some(value) = try_fold_int(instr) {
                        let int_type = instr.get_type().into_int_type();
                        let const_val = int_type.const_int(
                            value as u64,
                            true,
                        );
                        if let Ok(iv) = inkwell::values::IntValue::try_from(*instr) {
                            iv.replace_all_uses_with(const_val);
                            instr.erase_from_basic_block();
                            changed = true;
                        }
                    }
                }
            }
        }

        changed
    }
}
