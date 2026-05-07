use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn default_profile_churn_report_writes_machine_checkable_summary() {
    let repo_root = repo_root();
    let report_path = repo_root
        .join("target")
        .join("text-metrics")
        .join("default-profile-churn.json");
    let _ = fs::remove_file(&report_path);

    let run = run_xtask(&repo_root, &["text-metrics-churn"]);

    assert_eq!(run.status_code, 0, "{}", run.output);
    assert!(
        run.output
            .contains("target/text-metrics/default-profile-churn.json"),
        "{}",
        run.output
    );
    assert!(
        report_path.exists(),
        "missing report at {}",
        report_path.display()
    );

    let report: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&report_path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", report_path.display())),
    )
    .expect("report should be valid JSON");

    assert_eq!(
        report["profiles"]["compatibility"],
        "mmdflux-heuristic-proportional-v1"
    );
    assert_eq!(report["profiles"]["recorded"], "mmdflux-sans-v1");
    assert!(
        report["summary"]["fixtures_compared"]
            .as_u64()
            .expect("fixtures_compared should be a number")
            > 0,
        "{report:#}"
    );
    assert!(
        report["summary"]["changed_svg_fixtures"]
            .as_u64()
            .expect("changed_svg_fixtures should be a number")
            > 0,
        "{report:#}"
    );
    assert!(
        report["fixtures"]
            .as_array()
            .expect("fixtures should be an array")
            .iter()
            .any(|fixture| fixture["fixture"]
                .as_str()
                .is_some_and(|path| path.ends_with("labeled_edges.mmd"))),
        "{report:#}"
    );
}

#[derive(Debug)]
struct CommandRun {
    status_code: i32,
    output: String,
}

fn run_xtask(repo_root: &Path, args: &[&str]) -> CommandRun {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .current_dir(repo_root)
        .env("MMDFLUX_REPO_ROOT", repo_root)
        .output()
        .unwrap_or_else(|error| panic!("failed to run xtask {:?}: {error}", args));

    CommandRun {
        status_code: output.status.code().unwrap_or(-1),
        output: format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask crate should live under the repository root")
        .to_path_buf()
}
