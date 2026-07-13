pub mod passes;
use inkwell::module::Module;
use inkwell::passes::PassBuilderOptions;
use inkwell::targets::{CodeModel, InitializationConfig, RelocMode, Target, TargetMachine};

pub fn init_native_target() {
    let config = InitializationConfig {
        asm_parser: false,
        asm_printer: false,
        base: true,
        disassembler: false,
        info: true,
        machine_code: false,
    };
    let _ = Target::initialize_native(&config);
}

pub fn create_host_target_machine() -> Option<TargetMachine> {
    let triple = TargetMachine::get_default_triple();
    let target = Target::from_triple(&triple).ok()?;
    let cpu = TargetMachine::get_host_cpu_name().to_string();
    let features = TargetMachine::get_host_cpu_features().to_string();
    target.create_target_machine(
        &triple,
        &cpu,
        &features,
        inkwell::OptimizationLevel::Default,
        RelocMode::Default,
        CodeModel::JITDefault,
    )
}

pub fn run_llvm_optimizations(module: &Module, passes: &str) -> Result<bool, String> {
    let machine = create_host_target_machine()
        .ok_or_else(|| "failed to create target machine".to_string())?;
    let options = PassBuilderOptions::create();
    options.set_verify_each(false);
    module
        .run_passes(passes, &machine, options)
        .map(|()| true)
        .map_err(|e| format!("LLVM pass pipeline failed: {}", e))
}
