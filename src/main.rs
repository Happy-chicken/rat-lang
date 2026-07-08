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
use midend::ir_emitter::IrEmitter;

fn main() {
    let src = r#"
    def add(a:int, b:int)->int {
        return a + b;
    }
    def main()->int {
        let x:int = 10;
        let y:int = 20;
        return add(x, y);
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
    diag_ctxt.print_all(&mut std::io::stdout()).expect("");

    println!("\n=== LLVM IR ===");
    let context = Context::create();
    let mut emitter = IrEmitter::new(&context, "main", &mut diag_ctxt);
    emitter.generate(&ast, &sema_ctx);

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
