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
class Circle {
    let name: str = "c";
    let r: int = 0;
    
}

class Rect {
    let name: str = "s";
    let w: int = 0;
    let h: int = 0;   
}

trait Computable {
    decl area(factor: int) -> int;
    decl say() -> str;
}

impl Computable for Rect {
    def area(self: Rect, factor: int) -> int {
        return self.w * self.h * factor;
    }

    def say(self: Rect) -> str {
        return self.name;
    }
}

impl Computable for Circle {
    def area(self: Circle, factor: int) -> int {
        return self.r * self.r * factor;
    }

    def say(self: Circle) -> str {
        return self.name;
    }
}

def fib(n: int) -> int {
    if n < 2 {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

def main() -> int {
    let r: Rect = Rect("aa", 10, 20);
    let a1: int = r.area(1);
    let c: Circle = Circle("bb", 10);
    let a2: int = c.area(3);

    let nums: list<int> = [1, 2, 3, 4, 5];
    let s: int = 0;
    let i: int = 0;
    while i < 5 {
        s = s + nums[i];
        i = i + 1;
    }

    let f: int = fib(6);

    let x: int = 99;
    let p: ptr<int> = &x;
    let v: int = *p;

    return a1 + a2 + s + f + v;
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
