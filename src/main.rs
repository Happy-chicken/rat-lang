mod backend;
mod common;
mod frontend;
mod midend;
use backend::jit::JitRunner;
use common::DiagCtxt;
use common::location::SourceFile;
use frontend::lexer::Lexer;
use frontend::parser::Parser;
use frontend::sema_checker::AnalysisPipeline;
use inkwell::context::Context;
use midend::analyzer::{available_expression, context, live_variable, reaching_definition, very_busy_expression};
use midend::ir_emitter::IrEmitter;
use midend::optimizer::passes::{self, PassManager};

fn main() {
    let src = r#"
def test(a: int, b: int) -> int {
    let s: int = a + b;
    let r: int;
    if s > 10 {
        r = 1;
    } else {
        r = 2;
    }
    let t: int = a + b;
    return t + r;
}

def main() -> int {
    let dead: int = 99;
    return test(10, 20);
}
    "#;
    let file = SourceFile::new("main.rat".to_string(), src.to_string());
    let mut diag_ctxt = DiagCtxt::new();
    diag_ctxt.add_file(file.clone());

    let lexer = Lexer::new(&file.src);
    let mut parser = Parser::new(lexer, &mut diag_ctxt);
    let ast = parser.parse_program();

    let mut analysis_pipeline = AnalysisPipeline::standard();
    let sema_ctx = analysis_pipeline.run(&ast, &mut diag_ctxt);

    let mut pm = PassManager::new();
    pm.add_pass(Box::new(passes::dce::DeadCodeElimination));
    pm.add_pass(Box::new(passes::const_fold::ConstantFolding));
    pm.add_pass(Box::new(passes::cse::CommonSubexpressionElimination));

    {
        let context = Context::create();
        let mut emitter = IrEmitter::new(&context, "main", &mut diag_ctxt);
        emitter.generate(&ast, &sema_ctx);

        println!("=== Raw LLVM IR ===");
        emitter.dump_module();

        if !emitter.has_errors() {
            let ctx = context::build_analysis_context(emitter.module());
            println!("\n=== Live Variables ===");
            for (fn_name, cfg) in &ctx.cfgs {
                let live = live_variable::compute_live_variables(cfg);
                println!("fn {}:", fn_name);
                for block in &cfg.blocks {
                    if block.id == cfg.exit { continue; }
                    println!(
                        "  block {}: def={:?} use={:?} live_in={:?} succ={:?}",
                        block.id, block.def, block.r#use, live[block.id], block.successors
                    );
                }
            }

            println!("\n=== Reaching Definitions ===");
            for (fn_name, cfg) in &ctx.cfgs {
                let reaching = reaching_definition::compute_reaching_definitions(cfg);
                println!("fn {}:", fn_name);
                for block in &cfg.blocks {
                    if block.id == cfg.exit { continue; }
                    println!("  block {} out: {:?}", block.id, reaching[block.id]);
                }
            }

            println!("\n=== Available Expressions ===");
            for (fn_name, cfg) in &ctx.cfgs {
                if let Some(fdata) = ctx.func_data.get(fn_name) {
                    let analysis = available_expression::AvailableExpressionAnalysis::from_context(cfg, fdata);
                    let avail = available_expression::compute_available_expressions(cfg, &analysis);
                    println!("fn {}:", fn_name);
                    for block in &cfg.blocks {
                        if block.id == cfg.exit { continue; }
                        println!("  block {} out: {:?}", block.id, avail[block.id]);
                    }
                }
            }
            
            println!("\n=== Very Busy Expressions ===");
            for (fn_name, cfg) in &ctx.cfgs {
                if let Some(fdata) = ctx.func_data.get(fn_name) {
                    let analysis = very_busy_expression::VeryBusyExpressionAnalysis::from_context(cfg, fdata);
                    let avail = very_busy_expression::compute_very_busy_expressions(cfg, &analysis);
                    println!("fn {}:", fn_name);
                    for block in &cfg.blocks {
                        if block.id == cfg.exit { continue; }
                        println!("  block {} out: {:?}", block.id, avail[block.id]);
                    }
                }
            }

            let changes = pm.run_until_fixed_point(emitter.module(), 1);
            println!("=== After Custom Passes ({} changes) ===", changes);
            emitter.dump_module();

            match JitRunner::new(emitter.module()) {
                Ok(runner) => unsafe {
                    match runner.call_main() {
                        Ok(result) => println!("\n>> main() returned: {}", result),
                        Err(e) => eprintln!("JIT call failed: {}", e),
                    }
                },
                Err(e) => eprintln!("JIT init failed: {}", e),
            }
        }
    }

    diag_ctxt.print_all(&mut std::io::stdout()).expect("");
}
