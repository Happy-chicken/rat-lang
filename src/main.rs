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
use midend::analyzer::{available_expression, context, live_variable, reaching_definition};
use midend::ir_emitter::IrEmitter;

fn main() {
    let src = r#"
def func(a: ptr<int>) {
    *a = 10;
}
def main() -> int {
    let x: int = 99;
    let p: ptr<int> = &x;
    func(p);
    return x;
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

    println!("\n=== LLVM IR ===");
    let context = Context::create();
    let mut emitter = IrEmitter::new(&context, "main", &mut diag_ctxt);
    emitter.generate(&ast, &sema_ctx);
    emitter.dump_module();

    let ctx = context::build_analysis_context(emitter.module());
    if !emitter.has_errors() {
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

        match emitter.optimize_llvm() {
            Ok(true) => println!("LLVM passes applied successfully"),
            Ok(false) => println!("LLVM passes completed (no changes)"),
            Err(e) => println!("LLVM optimization failed: {}", e),
        }

        match JitRunner::new(emitter.module()) {
            Ok(runner) => unsafe {
                match runner.call_main() {
                    Ok(result) => println!("\n>> main() returned: {}", result),
                    Err(e) => eprintln!("JIT call failed: {}", e),
                }
            },
            Err(e) => eprintln!("JIT init failed: {}", e),
        }
    };

    live_variable::detect_unused_variables(&ctx.cfgs, &mut diag_ctxt);
    diag_ctxt.print_all(&mut std::io::stdout()).expect("");
}
