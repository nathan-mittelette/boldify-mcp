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
        .expect("impossible de lire tests/integrations")
        .map(|entry| entry.expect("entrée invalide"))
        .filter(|entry| entry.file_type().expect("type invalide").is_dir())
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
        .expect("nom de dossier invalide")
        .to_string();

    let markdown_input = case_dir.join("input.md");
    let html_input = case_dir.join("input.html");
    let output_path = case_dir.join("output.txt");

    let (syntax, input_path) = match (markdown_input.exists(), html_input.exists()) {
        (true, false) => ("markdown", markdown_input),
        (false, true) => ("html", html_input),
        (true, true) => panic!("{name}: un seul fichier input.* est autorisé"),
        (false, false) => panic!("{name}: fichier input.md ou input.html manquant"),
    };

    IntegrationCase {
        name: name.clone(),
        syntax,
        input: fs::read_to_string(&input_path)
            .unwrap_or_else(|_| panic!("impossible de lire {}", input_path.display())),
        input_path,
        expected_output: fs::read_to_string(&output_path)
            .unwrap_or_else(|_| panic!("{name}: impossible de lire output.txt")),
        expected_path: output_path,
    }
}

#[test]
fn integrations_assets_correspondent_aux_outputs_attendus() {
    let cases = load_cases();
    assert!(
        !cases.is_empty(),
        "aucun dossier de test trouvé dans tests/integrations"
    );

    let service = ContentService::new();
    let mut failures = Vec::new();

    for case in cases {
        let result = service.convert(case.syntax, &case.input);

        match result {
            Ok(actual) if actual == case.expected_output => {}
            Ok(actual) => failures.push(format!(
                "{}\n  input: {}\n  expected: {}\n  sortie inattendue\n  attendu: {:?}\n  obtenu : {:?}",
                case.name,
                case.input_path.display(),
                case.expected_path.display(),
                case.expected_output,
                actual
            )),
            Err(error) => failures.push(format!(
                "{}\n  input: {}\n  expected: {}\n  conversion en erreur: {}",
                case.name,
                case.input_path.display(),
                case.expected_path.display(),
                error
            )),
        }
    }

    assert!(
        failures.is_empty(),
        "certains assets ont échoué:\n\n{}",
        failures.join("\n\n")
    );
}
