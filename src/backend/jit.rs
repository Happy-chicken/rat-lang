use inkwell::execution_engine::ExecutionEngine;
use inkwell::module::Module;
use inkwell::OptimizationLevel;

pub struct JitRunner<'ctx> {
    engine: ExecutionEngine<'ctx>,
}

impl<'ctx> JitRunner<'ctx> {
    pub fn new(module: &Module<'ctx>) -> Result<Self, String> {
        let engine = module
            .create_jit_execution_engine(OptimizationLevel::None)
            .map_err(|e| format!("failed to create JIT engine: {:?}", e))?;
        Ok(JitRunner { engine })
    }

    pub unsafe fn call_main(&self) -> Result<i64, String> {
        let main_fn = unsafe {
            self.engine
                .get_function::<unsafe extern "C" fn() -> i64>("main")
        }
        .map_err(|e| format!("failed to find main function: {:?}", e))?;
        Ok(unsafe { main_fn.call() })
    }
}
