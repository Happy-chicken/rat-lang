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
use midend::ast_optimizer;
use midend::ir_emitter::IrEmitter;
use midend::ir_optimizer::passes::{self, PassManager as LlvmPassManager};

use crate::frontend::ast::printer::AstPrint;

fn main() {
    let src = r#"
def main() -> int {
    let a: int = 10;
    let b: int = 20;
    let c:int;
    if a < b {
        c = a;
    }
    else {
        c = b;
    }
    return c;
}
    "#;
    let file = SourceFile::new("main.rat".to_string(), src.to_string());
    let mut diag_ctxt = DiagCtxt::new();
    diag_ctxt.add_file(file.clone());

    let lexer = Lexer::new(&file.src);
    let mut parser = Parser::new(lexer, &mut diag_ctxt);
    let mut ast = parser.parse_program();
    // let mut out = String::new();
    // ast.print("", true, &mut out).unwrap();
    // println!("{}", out);

    let mut analysis_pipeline = AnalysisPipeline::standard();
    let sema_ctx = analysis_pipeline.run(&ast, &mut diag_ctxt);

    let ast_pm = ast_optimizer::PassManager::standard();
    let changes = ast_pm.run(&mut ast);
    println!("\nAST optimizations: {} changes", changes);

    // out.clear();
    // ast.print("", true, &mut out).unwrap();
    // println!("After AST opt:\n{}", out);

    let mut pm = LlvmPassManager::new();
    pm.add_pass(Box::new(passes::mem2reg::Mem2Reg));
    // pm.add_pass(Box::new(passes::cse::CommonSubexpressionElimination));
    // pm.add_pass(Box::new(passes::const_fold::ConstantFolding));

    
    let context = Context::create();
    let mut emitter = IrEmitter::new(&context, "main", &mut diag_ctxt);
    emitter.generate(&ast, &sema_ctx);

    println!("=== Raw LLVM IR ===");
    emitter.dump_module();

    if !emitter.has_errors() {
        let cfg = midend::analyzer::dataflow::build_dummy_cfg();
        let dom = midend::analyzer::dominator::compute_dominators_fast(&cfg);
        for block in &cfg.blocks {
            if block.id == cfg.exit { continue; }
            println!(
                "  block {}: successors={:?}, dominators={:?}",
                block.id, block.successors, dom[block.id]
            );
        }
        

        let changes = pm.run_until_fixed_point(emitter.module(), 1);
        println!("=== After opt ({} changes) ===", changes);
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
