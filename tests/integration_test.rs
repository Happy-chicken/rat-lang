use std::fs;
use std::path::Path;
use inkwell::context::Context;
use ratlang::backend::jit::JitRunner;
use ratlang::common::DiagCtxt;
use ratlang::common::location::SourceFile;
use ratlang::frontend::lexer::Lexer;
use ratlang::frontend::parser::Parser;
use ratlang::frontend::sema_checker::{AnalysisPipeline, sema_ctx::SemaCtxt};
use ratlang::midend::ir_emitter::IrEmitter;

fn run_pipeline(src: &str) -> (DiagCtxt, SemaCtxt) {
    let file = SourceFile::new("test.rat".into(), src.to_string());
    let mut diag = DiagCtxt::new();
    diag.add_file(file.clone());

    let lexer = Lexer::new(&file.src);
    let mut parser = Parser::new(lexer, &mut diag);
    let ast = parser.parse_program();

    let mut pipeline = AnalysisPipeline::standard();
    let sema_ctx = pipeline.run(&ast, &mut diag);
    (diag, sema_ctx)
}

fn diag_output(diag: &DiagCtxt) -> String {
    let mut buf = Vec::new();
    diag.print_all(&mut buf).ok();
    String::from_utf8_lossy(&buf).to_string()
}

struct CaseDirectives {
    should_pass: bool,
    should_fail: bool,
    expected_errors: Vec<String>,
    expected_warnings: Vec<String>,
    jit_expect: Option<i64>,
}

fn parse_directives(src: &str) -> CaseDirectives {
    CaseDirectives {
        should_pass: src.contains("// @pass"),
        should_fail: src.contains("// @fail"),
        expected_errors: src
            .lines()
            .filter(|l| l.starts_with("// @error:"))
            .map(|l| l.trim_start_matches("// @error:").trim().to_string())
            .collect(),
        expected_warnings: src
            .lines()
            .filter(|l| l.starts_with("// @warning:"))
            .map(|l| l.trim_start_matches("// @warning:").trim().to_string())
            .collect(),
        jit_expect: src
            .lines()
            .filter(|l| l.starts_with("// @jit:"))
            .filter_map(|l| l.trim_start_matches("// @jit:").trim().parse::<i64>().ok())
            .next(),
    }
}

fn run_cat_case(path: &Path, display_name: &str) {
    let src = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e));

    let directives = parse_directives(&src);
    let (diag, sema_ctx) = run_pipeline(&src);
    let output = diag_output(&diag);

    // check pass/fail
    if directives.should_pass {
        assert!(
            !diag.has_errors(),
            "{}: expected no errors but got:\n{}",
            display_name,
            output
        );
    }
    if directives.should_fail {
        assert!(
            diag.has_errors(),
            "{}: expected errors but got none",
            display_name
        );
    }

    // check specific error/warning messages
    for msg in &directives.expected_errors {
        assert!(
            output.contains(msg.as_str()),
            "{}: expected error containing '{}', but output was:\n{}",
            display_name, msg, output
        );
    }
    for msg in &directives.expected_warnings {
        assert!(
            output.contains(msg.as_str()),
            "{}: expected warning containing '{}', but output was:\n{}",
            display_name, msg, output
        );
    }

    // JIT
    if let Some(expected) = directives.jit_expect {
        assert!(
            !diag.has_errors(),
            "{}: @jit expected but pipeline has errors:\n{}",
            display_name, output
        );

        let jit_src = src.clone();
        let file = SourceFile::new("test.rat".into(), jit_src);
        let lexer_src = file.src.clone();
        let lexer = Lexer::new(&lexer_src);
        let mut fresh_diag = DiagCtxt::new();
        fresh_diag.add_file(file);
        let mut parser = Parser::new(lexer, &mut fresh_diag);
        let ast = parser.parse_program();

        let context = Context::create();
        let mut emitter = IrEmitter::new(&context, "test", &mut fresh_diag);
        emitter.generate(&ast, &sema_ctx);

        match JitRunner::new(emitter.module()) {
            Ok(runner) => unsafe {
                match runner.call_main() {
                    Ok(result) => {
                        assert_eq!(
                            result, expected,
                            "{}: JIT returned {}, expected {}",
                            display_name, result, expected
                        );
                    }
                    Err(e) => {
                        panic!("{}: JIT call failed: {}", display_name, e);
                    }
                }
            },
            Err(e) => {
                panic!("{}: JIT init failed: {}", display_name, e);
            }
        }
    }
}

fn collect_cat_files(dir: &Path) -> Vec<(String, std::path::PathBuf)> {
    let mut result = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                result.extend(collect_cat_files(&path));
            } else if path.extension().map_or(false, |ext| ext == "cat") {
                let parent = path.parent().unwrap().file_name().unwrap().to_string_lossy();
                let fname = path.file_name().unwrap().to_string_lossy();
                let display = format!("{}/{}", parent, fname);
                result.push((display, path));
            }
        }
    }
    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}

#[test]
fn all_cat_cases() {
    let cases_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("cases");
    let entries = collect_cat_files(&cases_dir);

    assert!(!entries.is_empty(), "no .cat test cases found in {}", cases_dir.display());

    let mut failures = vec![];
    for (display_name, path) in &entries {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_cat_case(path, display_name)
        })) {
            Ok(()) => {}
            Err(e) => {
                let msg = e
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_else(|| "unknown panic".to_string());
                failures.push(format!("  FAIL {}: {}", display_name, msg));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "\n{}/{} test cases failed:\n{}",
            failures.len(),
            entries.len(),
            failures.join("\n")
        );
    }
}
