pub mod const_fold;
pub mod cse;
pub mod dce;
pub mod mem2reg;

use inkwell::module::Module;

pub trait Pass {
    fn name(&self) -> &'static str;

    fn description(&self) -> &'static str {
        ""
    }

    fn run(&self, module: &Module) -> bool;
}

pub struct PassManager {
    passes: Vec<Box<dyn Pass>>,
}

impl PassManager {
    pub fn new() -> Self {
        Self { passes: Vec::new() }
    }

    pub fn add_pass(&mut self, pass: Box<dyn Pass>) {
        self.passes.push(pass);
    }

    pub fn run_all(&self, module: &Module) -> usize {
        let mut total_changes = 0;
        for pass in &self.passes {
            let changed = pass.run(module);
            if changed {
                total_changes += 1;
            }
        }
        total_changes
    }

    pub fn run_until_fixed_point(&self, module: &Module, max_iter: usize) -> usize {
        let mut total_changes = 0;
        for _ in 0..max_iter {
            let changes = self.run_all(module);
            if changes == 0 {
                break;
            }
            total_changes += changes;
        }
        total_changes
    }
}

impl Default for PassManager {
    fn default() -> Self {
        Self::new()
    }
}
