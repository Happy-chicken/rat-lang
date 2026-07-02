mod common;
mod frontend;
use common::DiagCtxt;
use common::location::SourceFile;
use frontend::lexer::Lexer;
use frontend::parser::Parser;
use crate::frontend::ast::printer::AstPrint;
fn main() {
    let src = r#" 
class Eye {
}
class Ear {
}
class Dog {
    var eye:Eye;
    var ear:Ear;
}
trait Speakable {
    decl speak() -> str;
}
impl Speakable for Dog {
    def speak() -> str {
        return "Woof!";
    }
}
impl Dog {
    def new() -> str {
        return "Dog created!";
    }
}
decl sub(a:int, b:int)->int;
def add(a:int, b:int)->int {
    if a > b and a < 10 or !a {
        // a = a + b;
    } else {
        a = - a - b;
    }
    return a;
}
def main() { 
    var b:bool = 1 < 2 > 3;
    var x:int = 42 + (1 + 2) * 3;
    var y:float = 3.14;
    var z:str = "Hello, world!";
    var mylist:list<list<int>> = [1, 2, 3, 4, 5];
    var myarray:array<3, array<3, int>> = [1, 2, 3];
    var myptr:ptr<int> = &x;
    var xx:int = *myptr;
    var mycall:int = add(1, 2);
    var dog:Dog;
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
    diag_ctxt.print_all(&mut std::io::stdout()).expect("");
    // print!("{:#?}", ast);
}
    