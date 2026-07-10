mod backend;
mod common;
mod frontend;
mod midend;
use backend::jit::JitRunner;
use common::DiagCtxt;
use common::location::SourceFile;
use frontend::ast::printer::AstPrint;
use frontend::lexer::Lexer;
use frontend::parser::Parser;
use frontend::sema_checker::AnalysisPipeline;
use inkwell::context::Context;
use midend::dataflow;
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

    let mut output = String::new();
    ast.print("", true, &mut output).unwrap();
    println!("{}", output);

    let mut analysis_pipeline = AnalysisPipeline::standard();
    let sema_ctx = analysis_pipeline.run(&ast, &mut diag_ctxt);
    sema_ctx.symbol_table.dump();
    

    println!("\n=== LLVM IR ===");
    let context = Context::create();
    let mut emitter = IrEmitter::new(&context, "main", &mut diag_ctxt);
    emitter.generate(&ast, &sema_ctx);
    emitter.dump_module();

    if !emitter.has_errors() {
        println!("\n=== Dataflow Analysis (Live Variables) ===");
        let cfgs = dataflow::build_cfg(&ast);
        for (fn_name, cfg) in &cfgs {
            let live = dataflow::compute_live_variables(cfg);
            println!(
                "fn {}: def={:?} use={:?} live_in={:?}",
                fn_name, cfg.blocks[0].def, cfg.blocks[0].r#use, live[0]
            );
        }

        println!("\n=== LLVM Optimization ===");
        match emitter.optimize_llvm() {
            Ok(true) => println!("LLVM passes applied successfully"),
            Ok(false) => println!("LLVM passes completed (no changes)"),
            Err(e) => println!("LLVM optimization failed: {}", e),
        }
        println!("=== Optimized LLVM IR ===");
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
    diag_ctxt.print_all(&mut std::io::stdout()).expect("");
}
