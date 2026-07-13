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
use midend::ir_emitter::IrEmitter;

fn main() {
    let src = r#"
def main() -> int {
    let a: int = 10;
    let b: int = 20;
    let x: int = a + b;
    let y: int = x * 2;
    let gt: bool = y > 50;

    let f: float = 1.5;
    let g: float = 2.5;
    let fadd: float = f + g;
    let fmul: float = f * 2.0;
    let flt: bool = f < g;
    let fneg: float = -f;

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

    {
        let context = Context::create();
        let mut emitter = IrEmitter::new(&context, "main", &mut diag_ctxt);
        emitter.generate(&ast, &sema_ctx);

        println!("=== LLVM IR ===");
        emitter.dump_module();

        if !emitter.has_errors() {
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
