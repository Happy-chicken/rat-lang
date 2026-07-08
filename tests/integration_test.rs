use ratlang::common::DiagCtxt;
use ratlang::common::location::SourceFile;
use ratlang::frontend::lexer::Lexer;
use ratlang::frontend::parser::Parser;
use ratlang::frontend::sema_checker::AnalysisPipeline;

fn run_pipeline(src: &str) -> DiagCtxt {
    let file = SourceFile::new("test.rat".into(), src.to_string());
    let mut diag = DiagCtxt::new();
    diag.add_file(file.clone());

    let lexer = Lexer::new(&file.src);
    let mut parser = Parser::new(lexer, &mut diag);
    let ast = parser.parse_program();

    let mut pipeline = AnalysisPipeline::standard();
    let _ = pipeline.run(&ast, &mut diag);
    diag
}

fn run_parser_only(src: &str) -> DiagCtxt {
    let file = SourceFile::new("test.rat".into(), src.to_string());
    let mut diag = DiagCtxt::new();
    diag.add_file(file.clone());

    let lexer = Lexer::new(&file.src);
    let mut parser = Parser::new(lexer, &mut diag);
    let _ = parser.parse_program();
    diag
}

fn run_resolver_only(src: &str) -> DiagCtxt {
    let file = SourceFile::new("test.rat".into(), src.to_string());
    let mut diag = DiagCtxt::new();
    diag.add_file(file.clone());

    let lexer = Lexer::new(&file.src);
    let mut parser = Parser::new(lexer, &mut diag);
    let ast = parser.parse_program();

    use ratlang::frontend::sema_checker::{resolver::Resolver, pass::Pass, sema_ctx::SemaCtxt};
    let mut ctx = SemaCtxt::new();
    let mut resolver = Resolver::new();
    let _ = resolver.run(&ast, &mut ctx, &mut diag);
    diag
}

fn diag_has_error_containing(diag: &DiagCtxt, text: &str) -> bool {
    let mut buf = Vec::new();
    diag.print_all(&mut buf).ok();
    let output = String::from_utf8_lossy(&buf);
    output.contains(text)
}

fn diag_error_count(diag: &DiagCtxt) -> usize {
    let mut buf = Vec::new();
    diag.print_all(&mut buf).ok();
    let output = String::from_utf8_lossy(&buf);
    output.matches("error:").count()
}

// ============================================================
//  Parser Tests
// ============================================================

#[test]
fn test_parse_empty_program() {
    let diag = run_parser_only("");
    assert!(!diag.has_errors());
}

#[test]
fn test_parse_function_def_no_params() {
    let diag = run_parser_only("def main() { return 0; }");
    assert!(!diag.has_errors());
}

#[test]
fn test_parse_function_def_with_params() {
    let diag = run_parser_only("def add(x: int, y: int) -> int { return x + y; }");
    assert!(!diag.has_errors());
}

#[test]
fn test_parse_function_decl() {
    let diag = run_parser_only("decl abs(x: float) -> float;");
    assert!(!diag.has_errors());
}

#[test]
fn test_parse_class() {
    let diag = run_parser_only("class Foo { let a: int; let b: bool; }");
    assert!(!diag.has_errors());
}

#[test]
fn test_parse_trait() {
    let diag = run_parser_only("trait Drawable { decl draw() -> none; decl size() -> int; }");
    assert!(!diag.has_errors());
}

#[test]
fn test_parse_impl_for_class() {
    let diag = run_parser_only("impl Foo { def new() -> Foo { return 0; } }");
    assert!(!diag.has_errors());
}

#[test]
fn test_parse_impl_trait_for_class() {
    let diag = run_parser_only("impl Draw for Circle { def draw() { return; } }");
    assert!(!diag.has_errors());
}

#[test]
fn test_parse_var_def() {
    let diag = run_parser_only("def main() { let x: int = 42; return x; }");
    assert!(!diag.has_errors());
}

#[test]
fn test_parse_if_elif_else() {
    let diag = run_parser_only(
        "def main() { if x > 0 { return 1; } elif x == 0 { return 0; } else { return -1; } }",
    );
    assert!(!diag.has_errors());
}

#[test]
fn test_parse_while_loop() {
    let diag = run_parser_only("def main() { while x < 10 { x = x + 1; } return; }");
    assert!(!diag.has_errors());
}

#[test]
fn test_parse_break_continue() {
    let diag = run_parser_only(
        "def main() { while true { if x > 5 { break; } continue; } return; }",
    );
    assert!(!diag.has_errors());
}

#[test]
fn test_parse_call_expression() {
    let diag = run_parser_only("def main() { foo(x, 1 + 2); return; }");
    assert!(!diag.has_errors());
}

#[test]
fn test_parse_member_access() {
    let diag = run_parser_only("def main() { let v: int = obj.field; return v; }");
    assert!(!diag.has_errors());
}

#[test]
fn test_parse_index_expression() {
    let diag = run_parser_only("def main() { let v: int = arr[0]; return v; }");
    assert!(!diag.has_errors());
}

// ============================================================
//  Resolver Tests (top-level name collection, struct recursion)
// ============================================================

#[test]
fn test_resolver_duplicate_function() {
    let diag = run_resolver_only("def f() { return; } def f() { return; }");
    assert!(diag.has_errors());
    assert!(diag_has_error_containing(&diag, "defined multiple times"));
}

#[test]
fn test_resolver_duplicate_class() {
    let diag = run_resolver_only("class Foo { let a: int; } class Foo { let b: int; }");
    assert!(diag.has_errors());
    assert!(diag_has_error_containing(&diag, "defined multiple times"));
}

#[test]
fn test_resolver_duplicate_trait() {
    let diag =
        run_resolver_only("trait T { decl m(); } trait T { decl n(); }");
    assert!(diag.has_errors());
    assert!(diag_has_error_containing(&diag, "defined multiple times"));
}

#[test]
fn test_resolver_dup_function_and_class_same_name() {
    let diag = run_resolver_only("class Foo { let a: int; } def Foo() { return; }");
    assert!(diag.has_errors());
}

#[test]
fn test_resolver_duplicate_field() {
    let diag = run_resolver_only("class Foo { let a: int; let a: bool; }");
    assert!(diag.has_errors());
    assert!(diag_has_error_containing(&diag, "duplicate field name"));
}

#[test]
fn test_resolver_duplicate_parameter() {
    let diag = run_resolver_only("def f(x: int, x: int) { return; }");
    assert!(diag.has_errors());
    assert!(diag_has_error_containing(&diag, "duplicate parameter name"));
}

#[test]
fn test_resolver_duplicate_method_in_trait() {
    let diag = run_resolver_only(
        "trait T { decl m() -> int; decl m() -> float; }",
    );
    assert!(diag.has_errors());
    assert!(diag_has_error_containing(&diag, "duplicate method name"));
}

#[test]
fn test_resolver_struct_recursion_direct() {
    let diag = run_resolver_only("class A { let b: B; } class B { let a: A; }");
    assert!(diag.has_errors());
    assert!(diag_has_error_containing(&diag, "recursive type definition"));
}

#[test]
fn test_resolver_struct_recursion_indirect() {
    let diag = run_resolver_only(
        "class A { let b: B; } class B { let c: C; } class C { let a: A; }",
    );
    assert!(diag.has_errors());
    assert!(diag_has_error_containing(&diag, "recursive type definition"));
}

#[test]
fn test_resolver_no_struct_recursion() {
    let diag = run_resolver_only(
        "class A { let x: int; } class B { let y: float; }",
    );
    assert!(!diag.has_errors());
}

#[test]
fn test_resolver_valid_program() {
    let diag = run_resolver_only(
        "class Point { let x: int; let y: int; }
         trait Movable { decl move() -> none; }
         def distance(p: Point) -> float { return 0.0; }
         decl origin() -> Point;",
    );
    assert!(!diag.has_errors());
}

// ============================================================
//  Checker Tests (scope-based checks)
// ============================================================

#[test]
fn test_checker_undefined_variable() {
    let diag = run_pipeline("def main() { return x; }");
    assert!(diag.has_errors());
    assert!(diag_has_error_containing(&diag, "cannot find value `x`"));
}

#[test]
fn test_checker_variable_resolved() {
    let diag = run_pipeline("def main() { let x: int = 1; return x; }");
    assert!(!diag.has_errors());
}

#[test]
fn test_checker_duplicate_variable_same_scope() {
    let diag = run_pipeline(
        "def main() { let x: int = 1; let x: int = 2; return x; }",
    );
    assert!(diag.has_errors());
    assert!(diag_has_error_containing(&diag, "already declared"));
}

#[test]
fn test_checker_variable_shadowing() {
    let diag = run_pipeline(
        "def main() {
            let x: int = 1;
            if true {
                let x: int = 2;
                return x;
            }
            return x;
        }",
    );
    assert!(!diag.has_errors());
}

#[test]
fn test_checker_return_outside_function() {
    let diag = run_pipeline("return 0;");
    assert!(diag.has_errors());
    assert!(diag_has_error_containing(&diag, "return statement outside"));
}

#[test]
fn test_checker_break_outside_loop() {
    let diag = run_pipeline("def main() { break; return; }");
    assert!(diag.has_errors());
    assert!(diag_has_error_containing(&diag, "break statement outside"));
}

#[test]
fn test_checker_continue_outside_loop() {
    let diag = run_pipeline("def main() { continue; return; }");
    assert!(diag.has_errors());
    assert!(diag_has_error_containing(&diag, "continue statement outside"));
}

#[test]
fn test_checker_break_inside_loop() {
    let diag = run_pipeline(
        "def main() { while true { if x > 3 { break; } } return; }",
    );
    assert!(!diag_has_error_containing(&diag, "break statement outside"));
}

#[test]
fn test_checker_continue_inside_loop() {
    let diag = run_pipeline(
        "def main() { while true { if x > 3 { continue; } } return; }",
    );
    assert!(!diag_has_error_containing(&diag, "continue statement outside"));
}

#[test]
fn test_checker_call_non_function() {
    let diag = run_pipeline(
        "def main() { let x: int = 1; x(); return; }",
    );
    assert!(diag.has_errors());
    assert!(diag_has_error_containing(&diag, "not callable"));
}

#[test]
fn test_checker_call_valid_function() {
    let diag = run_pipeline(
        "decl f() -> int;
         def main() { f(); return; }",
    );
    assert!(!diag.has_errors());
}

#[test]
fn test_checker_impl_class_not_found() {
    let diag = run_pipeline("impl NoSuchClass { def m() { return; } }");
    assert!(diag.has_errors());
    assert!(diag_has_error_containing(&diag, "class `NoSuchClass` not found"));
}

#[test]
fn test_checker_impl_trait_not_found() {
    let diag = run_pipeline(
        "class Foo { let a: int; } impl NoSuchTrait for Foo { def m() { return; } }",
    );
    assert!(diag.has_errors());
    assert!(diag_has_error_containing(&diag, "trait `NoSuchTrait` not found"));
}

#[test]
fn test_checker_impl_valid() {
    let diag = run_pipeline(
        "class Foo { let a: int; } impl Foo { def m() { return; } }",
    );
    assert!(!diag.has_errors());
}

#[test]
fn test_checker_nested_scopes() {
    let diag = run_pipeline(
        "def main() {
            let a: int = 1;
            if true {
                let b: int = 2;
                return a + b;
            }
            return a;
        }",
    );
    assert!(!diag.has_errors());
}

#[test]
fn test_checker_variable_not_visible_outside_block() {
    let diag = run_pipeline(
        "def main() {
            if true {
                let b: int = 2;
            }
            return b;
        }",
    );
    assert!(diag.has_errors());
    assert!(diag_has_error_containing(&diag, "cannot find value `b`"));
}

// ============================================================
//  Full Pipeline Tests
// ============================================================

#[test]
fn test_full_pipeline_no_errors() {
    let src = "class Point { let x: int; let y: int; }
               trait Movable { decl move() -> none; }
               def distance(a: Point, b: Point) -> float {
                   let dx: float = 0.0;
                   let dy: float = 0.0;
                   return dx + dy;
               }
               def main() {
                   let p: Point = Point;
                   return;
               }";
    let diag = run_pipeline(src);
    assert!(diag_error_count(&diag) == 0);
}

#[test]
fn test_multiple_errors_reported() {
    let diag = run_pipeline(
        "def main() {
            return x;
            return y;
        }
        def main() { return; }",
    );
    assert!(diag_error_count(&diag) >= 3); // x not found, y not found, main redefined
}
