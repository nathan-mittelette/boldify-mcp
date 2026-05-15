use std::{
    fs,
    path::{Path, PathBuf},
};

use service::ContentService;

#[derive(Debug)]
struct IntegrationCase {
    name: String,
    syntax: &'static str,
    input_path: PathBuf,
    expected_path: PathBuf,
    input: String,
    expected_output: String,
}

fn integrations_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/integrations")
}

fn load_cases() -> Vec<IntegrationCase> {
    let mut case_dirs = fs::read_dir(integrations_root())
        .expect("cannot read tests/integrations")
        .map(|entry| entry.expect("invalid entry"))
        .filter(|entry| entry.file_type().expect("invalid file type").is_dir())
        .collect::<Vec<_>>();

    case_dirs.sort_by_key(|entry| entry.file_name());

    case_dirs
        .into_iter()
        .map(|entry| load_case(&entry.path()))
        .collect()
}

fn load_case(case_dir: &Path) -> IntegrationCase {
    let name = case_dir
        .file_name()
        .and_then(|name| name.to_str())
        .expect("invalid directory name")
        .to_string();

    let markdown_input = case_dir.join("input.md");
    let html_input = case_dir.join("input.html");
    let output_path = case_dir.join("output.txt");

    let (syntax, input_path) = match (markdown_input.exists(), html_input.exists()) {
        (true, false) => ("markdown", markdown_input),
        (false, true) => ("html", html_input),
        (true, true) => panic!("{name}: only one input.* file is allowed"),
        (false, false) => panic!("{name}: missing input.md or input.html file"),
    };

    IntegrationCase {
        name: name.clone(),
        syntax,
        input: fs::read_to_string(&input_path)
            .unwrap_or_else(|_| panic!("cannot read {}", input_path.display())),
        input_path,
        expected_output: fs::read_to_string(&output_path)
            .unwrap_or_else(|_| panic!("{name}: cannot read output.txt")),
        expected_path: output_path,
    }
}

#[test]
fn integrations_assets_match_expected_outputs() {
    let cases = load_cases();
    assert!(
        !cases.is_empty(),
        "no test directories found in tests/integrations"
    );

    let service = ContentService::new();
    let mut failures = Vec::new();

    for case in cases {
        let result = service.convert(case.syntax, &case.input);

        match result {
            Ok(actual) if actual == case.expected_output => {}
            Ok(actual) => failures.push(format!(
                "{}\n  input: {}\n  expected: {}\n  unexpected output\n  expected: {:?}\n  got: {:?}",
                case.name,
                case.input_path.display(),
                case.expected_path.display(),
                case.expected_output,
                actual
            )),
            Err(error) => failures.push(format!(
                "{}\n  input: {}\n  expected: {}\n  conversion error: {}",
                case.name,
                case.input_path.display(),
                case.expected_path.display(),
                error
            )),
        }
    }

    assert!(
        failures.is_empty(),
        "some assets failed:\n\n{}",
        failures.join("\n\n")
    );
}
