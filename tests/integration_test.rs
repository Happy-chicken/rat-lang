use std::fs;
use std::path::Path;
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

fn diag_output(diag: &DiagCtxt) -> String {
    let mut buf = Vec::new();
    diag.print_all(&mut buf).ok();
    String::from_utf8_lossy(&buf).to_string()
}

fn run_cat_case(path: &Path) {
    let src = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e));

    let should_pass = src.contains("// @pass");
    let should_fail = src.contains("// @fail");

    let expected_errors: Vec<&str> = src
        .lines()
        .filter(|l| l.starts_with("// @error:"))
        .map(|l| l.trim_start_matches("// @error:").trim())
        .collect();
    let expected_warnings: Vec<&str> = src
        .lines()
        .filter(|l| l.starts_with("// @warning:"))
        .map(|l| l.trim_start_matches("// @warning:").trim())
        .collect();

    let diag = run_pipeline(&src);
    let output = diag_output(&diag);
    let name = path.file_name().unwrap().to_string_lossy();

    if should_pass {
        assert!(
            !diag.has_errors(),
            "{}: expected no errors but got:\n{}",
            name,
            output
        );
    }

    if should_fail {
        assert!(
            diag.has_errors(),
            "{}: expected errors but got none",
            name
        );
    }

    for msg in &expected_errors {
        assert!(
            output.contains(msg),
            "{}: expected error containing '{}', but output was:\n{}",
            name,
            msg,
            output
        );
    }

    for msg in &expected_warnings {
        assert!(
            output.contains(msg),
            "{}: expected warning containing '{}', but output was:\n{}",
            name,
            msg,
            output
        );
    }
}

#[test]
fn all_cat_cases() {
    let cases_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("cases");
    let mut entries: Vec<_> = fs::read_dir(&cases_dir)
        .expect("cases directory not found")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "cat"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    assert!(!entries.is_empty(), "no .cat test cases found in {}", cases_dir.display());

    let mut failures = vec![];
    for entry in &entries {
        let path = entry.path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        print!("Running test case: {}", name);
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_cat_case(&path))) {
            Ok(()) => {println!("{} - OK", name);},
            Err(e) => {
                let msg = e
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_else(|| "unknown panic".to_string());
                failures.push(format!("  FAIL {}: {}", name, msg));
            }
        }
    }

    if !failures.is_empty() {
        panic!("\n{}/{} test cases failed:\n{}", failures.len(), entries.len(), failures.join("\n"));
    }
}
