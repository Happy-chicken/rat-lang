mod common;
mod frontend;

use common::location::SourceFile;
use common::DiagCtxt;
use frontend::lexer::Lexer;

fn main() {
    let src = r#"
        def main() {
            var x = 42;
            var y = 3.14;
            var z = "Hello, world!";
            return x;
        }
    "#;
    let file = SourceFile::new("main.rs".to_string(), src.to_string());
    let mut diag_ctxt = DiagCtxt::new();
    diag_ctxt.add_file(file.clone());

    let lexer = Lexer::new(&file.src);
    for token in lexer {
        println!("{:?}", token);
        if token.kind == frontend::lexer::token::TokenKind::Error {
            let diag = diag_ctxt
                .struct_span_err(token.span, "unexpected character")
                .note("this token is invalid")
                .build();
            diag_ctxt.emit(diag);
        }
    }

    diag_ctxt.print_all(&mut std::io::stderr()).unwrap();
}
