use inkwell::module::Module;
use inkwell::values::{BasicValueEnum, InstructionOpcode, InstructionValue, Operand};

use super::Pass;

fn get_const_i64(op: &Operand) -> Option<i64> {
    match op {
        Operand::Value(v) => match *v {
            BasicValueEnum::IntValue(iv) => iv.get_zero_extended_constant().map(|c| c as i64),
            _ => None,
        },
        _ => None,
    }
}

fn get_const_f64(op: &Operand) -> Option<f64> {
    match op {
        Operand::Value(v) => match *v {
            BasicValueEnum::FloatValue(fv) => fv.get_constant().map(|(val, _)| val),
            _ => None,
        },
        _ => None,
    }
}

fn make_int_const<'ctx>(instr: &InstructionValue<'ctx>, val: i64) -> BasicValueEnum<'ctx> {
    instr.get_type().into_int_type().const_int(val as u64, true).into()
}

fn make_float_const<'ctx>(instr: &InstructionValue<'ctx>, val: f64) -> BasicValueEnum<'ctx> {
    instr.get_type().into_float_type().const_float(val).into()
}

fn make_bool_const<'ctx>(instr: &InstructionValue<'ctx>, val: bool) -> BasicValueEnum<'ctx> {
    instr.get_type().into_int_type().const_int(if val { 1 } else { 0 }, false).into()
}

fn try_fold_int(instr: &InstructionValue<'_>) -> Option<i64> {
    let lhs = get_const_i64(&instr.get_operand(0)?)?;
    let rhs = get_const_i64(&instr.get_operand(1)?)?;
    match instr.get_opcode() {
        InstructionOpcode::Add => Some(lhs.wrapping_add(rhs)),
        InstructionOpcode::Sub => Some(lhs.wrapping_sub(rhs)),
        InstructionOpcode::Mul => Some(lhs.wrapping_mul(rhs)),
        InstructionOpcode::SDiv if rhs != 0 => Some(lhs / rhs),
        InstructionOpcode::And => Some(lhs & rhs),
        InstructionOpcode::Or => Some(lhs | rhs),
        InstructionOpcode::Xor => Some(lhs ^ rhs),
        _ => None,
    }
}

fn try_fold_float(instr: &InstructionValue<'_>) -> Option<f64> {
    let lhs = get_const_f64(&instr.get_operand(0)?)?;
    let rhs = get_const_f64(&instr.get_operand(1)?)?;
    match instr.get_opcode() {
        InstructionOpcode::FAdd => Some(lhs + rhs),
        InstructionOpcode::FSub => Some(lhs - rhs),
        InstructionOpcode::FMul => Some(lhs * rhs),
        InstructionOpcode::FDiv if rhs != 0.0 => Some(lhs / rhs),
        _ => None,
    }
}

fn try_fold_icmp(instr: &InstructionValue<'_>) -> Option<bool> {
    let lhs = get_const_i64(&instr.get_operand(0)?)?;
    let rhs = get_const_i64(&instr.get_operand(1)?)?;
    match instr.get_opcode() {
        InstructionOpcode::ICmp => {
            let pred = instr.get_icmp_predicate()?;
            Some(match pred {
                inkwell::IntPredicate::EQ => lhs == rhs,
                inkwell::IntPredicate::NE => lhs != rhs,
                inkwell::IntPredicate::SLT => lhs < rhs,
                inkwell::IntPredicate::SGT => lhs > rhs,
                inkwell::IntPredicate::SLE => lhs <= rhs,
                inkwell::IntPredicate::SGE => lhs >= rhs,
                _ => return None,
            })
        }
        _ => None,
    }
}

fn try_fold_fcmp(instr: &InstructionValue<'_>) -> Option<bool> {
    let lhs = get_const_f64(&instr.get_operand(0)?)?;
    let rhs = get_const_f64(&instr.get_operand(1)?)?;
    match instr.get_opcode() {
        InstructionOpcode::FCmp => {
            let pred = instr.get_fcmp_predicate()?;
            Some(match pred {
                inkwell::FloatPredicate::OEQ | inkwell::FloatPredicate::UEQ => lhs == rhs,
                inkwell::FloatPredicate::ONE | inkwell::FloatPredicate::UNE => lhs != rhs,
                inkwell::FloatPredicate::OLT | inkwell::FloatPredicate::ULT => lhs < rhs,
                inkwell::FloatPredicate::OGT | inkwell::FloatPredicate::UGT => lhs > rhs,
                inkwell::FloatPredicate::OLE | inkwell::FloatPredicate::ULE => lhs <= rhs,
                inkwell::FloatPredicate::OGE | inkwell::FloatPredicate::UGE => lhs >= rhs,
                _ => return None,
            })
        }
        _ => None,
    }
}

fn do_replace<'ctx>(instr: &InstructionValue<'ctx>, const_val: BasicValueEnum<'ctx>) -> bool {
    use inkwell::values::IntValue;
    use inkwell::values::FloatValue;

    match const_val {
        BasicValueEnum::IntValue(cv) => {
            if let Ok(iv) = IntValue::try_from(*instr) {
                iv.replace_all_uses_with(cv);
                return true;
            }
        }
        BasicValueEnum::FloatValue(cv) => {
            if let Ok(fv) = FloatValue::try_from(*instr) {
                fv.replace_all_uses_with(cv);
                return true;
            }
        }
        _ => {}
    }
    false
}

pub struct ConstantFolding;

impl Pass for ConstantFolding {
    fn name(&self) -> &'static str {
        "const-fold"
    }

    fn description(&self) -> &'static str {
        "evaluates constant int/float/bool expressions at compile time"
    }

    fn run(&self, module: &Module) -> bool {
        let mut changed = false;

        for func in module.get_functions() {
            for bb in func.get_basic_blocks() {
                let instructions: Vec<_> = bb.get_instructions().collect();
                for instr in &instructions {
                    let fold_result = try_fold_int(instr).map(|v| make_int_const(instr, v))
                        .or_else(|| try_fold_float(instr).map(|v| make_float_const(instr, v)))
                        .or_else(|| try_fold_icmp(instr).map(|v| make_bool_const(instr, v)))
                        .or_else(|| try_fold_fcmp(instr).map(|v| make_bool_const(instr, v)));

                    if let Some(const_val) = fold_result {
                        if do_replace(instr, const_val) {
                            changed = true;
                        }
                    }
                }
            }
        }

        changed
    }
}
