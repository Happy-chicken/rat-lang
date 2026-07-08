mod common;
mod frontend;
use common::DiagCtxt;
use common::location::SourceFile;
use frontend::ast::printer::AstPrint;
use frontend::lexer::Lexer;
use frontend::parser::Parser;
use frontend::sema_checker::AnalysisPipeline;

fn main() {
    let src = r#"
    let x:int = 1;
    def main(a:ptr<int>)->int {
    let x:int = 1;
    {
        let local_var:int = 2;
    }
    let l:list<int> = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
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

    // let mut sema_ctx = SemaCtxt::new();
    let mut analysis_pipeline = AnalysisPipeline::standard();
    let sema_ctx = analysis_pipeline.run(&ast, &mut diag_ctxt);
    sema_ctx.symbol_table.dump();
    diag_ctxt.print_all(&mut std::io::stdout()).expect("");
    // print!("{:#?}", ast);
}
