use inkwell::module::Module;

use super::Pass;

pub struct Mem2Reg;

impl Pass for Mem2Reg {
    fn name(&self) -> &'static str {
        "mem2reg"
    }

    fn description(&self) -> &'static str {
        "LLVM mem2reg: promotes alloca/load/store to SSA register values"
    }

    fn run(&self, module: &Module) -> bool {
        crate::midend::ir_optimizer::init_native_target();
        match crate::midend::ir_optimizer::run_llvm_optimizations(module, "mem2reg") {
            Ok(changed) => changed,
            Err(e) => {
                eprintln!("[mem2reg] failed: {}", e);
                false
            }
        }
    }
}
