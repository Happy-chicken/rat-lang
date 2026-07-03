mod common;
mod frontend;
use crate::frontend::ast::printer::AstPrint;
use common::DiagCtxt;
use common::location::SourceFile;
use frontend::lexer::Lexer;
use frontend::parser::Parser;
use frontend::sema_checker::resolver::Resolver;
use frontend::sema_checker::symbol_table::SymbolTable;
fn main() {
    let src = r#"def main() { 
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
    let mut symbol_table = SymbolTable::new();
    let mut resolver = Resolver::new(&mut symbol_table, &mut diag_ctxt);
    resolver.resolve(&ast);
    diag_ctxt.print_all(&mut std::io::stdout()).expect("");
    // print!("{:#?}", ast);
}

