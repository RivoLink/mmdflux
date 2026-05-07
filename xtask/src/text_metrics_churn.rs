use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use mmdflux::graph::GeometryLevel;
use mmdflux::graph::measure::{
    COMPATIBILITY_TEXT_METRICS_PROFILE_ID, RECORDED_SANS_TEXT_METRICS_PROFILE_ID,
};
use mmdflux::{OutputFormat, RenderConfig, render_diagram};
use serde::Serialize;
use serde_json::Value;

const COMMAND: &str = "text-metrics-churn";
const DEFAULT_OUTPUT: &str = "target/text-metrics/default-profile-churn.json";
const GRAPH_FIXTURE_DIRS: &[&str] = &["flowchart", "class", "state"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextMetricsChurnOptions {
    output: Option<PathBuf>,
}

pub(crate) fn parse_text_metrics_churn_args<I, S>(args: I) -> Result<TextMetricsChurnOptions>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut args = args.into_iter();
    match args.next().map(|arg| arg.as_ref().to_string()) {
        Some(command) if command == COMMAND => {}
        Some(other) => bail!("expected `cargo xtask {COMMAND}`, got `cargo xtask {other}`"),
        None => bail!("missing `cargo xtask {COMMAND}` invocation"),
    }

    let mut output = None;
    while let Some(arg) = args.next().map(|arg| arg.as_ref().to_string()) {
        match arg.as_str() {
            "--output" => {
                let value = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("missing value for `--output`"))?;
                output = Some(PathBuf::from(value.as_ref()));
            }
            _ if arg.starts_with("--output=") => {
                let value = arg
                    .strip_prefix("--output=")
                    .expect("prefix checked")
                    .to_string();
                output = Some(PathBuf::from(value));
            }
            other => bail!("unknown `cargo xtask {COMMAND}` argument `{other}`"),
        }
    }

    Ok(TextMetricsChurnOptions { output })
}

pub(crate) fn run(options: TextMetricsChurnOptions) -> Result<()> {
    let repo_root = crate::repo_root();
    let output = options
        .output
        .unwrap_or_else(|| repo_root.join(DEFAULT_OUTPUT));
    let report = build_report(&repo_root)?;
    let json = serde_json::to_string_pretty(&report)?;

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create `{}`", parent.display()))?;
    }
    fs::write(&output, format!("{json}\n"))
        .with_context(|| format!("failed to write `{}`", output.display()))?;

    println!("wrote {}", display_path(&repo_root, &output));
    println!(
        "compared {} fixtures; changed SVG: {}; changed MMDS: {}; unexpected structural: {}",
        report.summary.fixtures_compared,
        report.summary.changed_svg_fixtures,
        report.summary.changed_mmds_fixtures,
        report.summary.unexpected_structural_changes,
    );

    Ok(())
}

pub(crate) fn help_text() -> &'static str {
    "\
cargo xtask text-metrics-churn [options]

Compare explicit compatibility-profile output against explicit recorded-profile
output across graph-family fixtures.

Options:
    --output <path>    Write JSON report to path

Default output:
    target/text-metrics/default-profile-churn.json"
}

fn build_report(repo_root: &Path) -> Result<ChurnReport> {
    let mut fixtures = Vec::new();
    for family in GRAPH_FIXTURE_DIRS {
        let dir = repo_root.join("tests").join("fixtures").join(family);
        for path in fixture_paths(&dir)? {
            fixtures.push(compare_fixture(repo_root, family, &path)?);
        }
    }

    let mut summary = ChurnSummary {
        fixtures_compared: fixtures.len(),
        ..ChurnSummary::default()
    };
    for fixture in &fixtures {
        if fixture.svg_changed {
            summary.changed_svg_fixtures += 1;
        }
        if fixture.mmds_changed {
            summary.changed_mmds_fixtures += 1;
        }
        if fixture
            .categories
            .iter()
            .any(|category| category == CATEGORY_BOUNDS_OR_LABEL)
        {
            summary.bounds_or_label_changes += 1;
        }
        if fixture
            .categories
            .iter()
            .any(|category| category == CATEGORY_ROUTE_COORDINATES)
        {
            summary.route_coordinate_changes += 1;
        }
        if fixture
            .categories
            .iter()
            .any(|category| category == CATEGORY_PROFILE_METADATA)
        {
            summary.profile_metadata_changes += 1;
        }
        if fixture
            .categories
            .iter()
            .any(|category| category == CATEGORY_UNEXPECTED_STRUCTURAL)
        {
            summary.unexpected_structural_changes += 1;
        }
        if fixture.render_error.is_some() {
            summary.render_errors += 1;
        }
    }

    Ok(ChurnReport {
        profiles: ChurnProfiles {
            compatibility: COMPATIBILITY_TEXT_METRICS_PROFILE_ID,
            recorded: RECORDED_SANS_TEXT_METRICS_PROFILE_ID,
        },
        summary,
        fixtures,
    })
}

fn fixture_paths(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("failed to read `{}`", dir.display()))? {
        let entry =
            entry.with_context(|| format!("failed to read entry in `{}`", dir.display()))?;
        let path = entry.path();
        if path.extension().is_some_and(|extension| extension == "mmd") {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn compare_fixture(repo_root: &Path, family: &str, path: &Path) -> Result<FixtureChurn> {
    let input =
        fs::read_to_string(path).with_context(|| format!("failed to read `{}`", path.display()))?;
    let fixture = display_path(repo_root, path);

    let compatibility = render_outputs(&input, COMPATIBILITY_TEXT_METRICS_PROFILE_ID);
    let recorded = render_outputs(&input, RECORDED_SANS_TEXT_METRICS_PROFILE_ID);

    let mut categories = BTreeSet::new();
    let mut render_errors = Vec::new();
    let mut unexpected_paths = BTreeSet::new();

    if let Err(error) = &compatibility.svg {
        render_errors.push(format!("compatibility SVG: {error}"));
    }
    if let Err(error) = &compatibility.mmds {
        render_errors.push(format!("compatibility MMDS: {error}"));
    }
    if let Err(error) = &recorded.svg {
        render_errors.push(format!("recorded SVG: {error}"));
    }
    if let Err(error) = &recorded.mmds {
        render_errors.push(format!("recorded MMDS: {error}"));
    }

    let svg_changed = match (&compatibility.svg, &recorded.svg) {
        (Ok(left), Ok(right)) => left != right,
        _ => {
            categories.insert(CATEGORY_UNEXPECTED_STRUCTURAL.to_string());
            unexpected_paths.insert("svg_render_error".to_string());
            false
        }
    };

    let mmds_changed = match (&compatibility.mmds, &recorded.mmds) {
        (Ok(left), Ok(right)) => {
            if left == right {
                false
            } else {
                let classification = classify_mmds_changes(left, right);
                categories.extend(classification.categories());
                unexpected_paths.extend(classification.unexpected_paths);
                true
            }
        }
        _ => {
            categories.insert(CATEGORY_UNEXPECTED_STRUCTURAL.to_string());
            unexpected_paths.insert("mmds_render_error".to_string());
            false
        }
    };

    if svg_changed && categories.is_empty() {
        categories.insert(CATEGORY_SVG_BYTES.to_string());
    }

    Ok(FixtureChurn {
        fixture,
        diagram_family: family.to_string(),
        svg_changed,
        mmds_changed,
        categories: categories.into_iter().collect(),
        unexpected_paths: unexpected_paths.into_iter().collect(),
        render_error: if render_errors.is_empty() {
            None
        } else {
            Some(render_errors.join("; "))
        },
    })
}

#[derive(Debug)]
struct RenderOutputs {
    svg: Result<String, String>,
    mmds: Result<String, String>,
}

fn render_outputs(input: &str, profile_id: &str) -> RenderOutputs {
    let base_config = RenderConfig {
        font_metrics_profile: Some(profile_id.to_string()),
        ..RenderConfig::default()
    };
    let routed_mmds_config = RenderConfig {
        geometry_level: GeometryLevel::Routed,
        ..base_config.clone()
    };

    RenderOutputs {
        svg: render_diagram(input, OutputFormat::Svg, &base_config).map_err(|error| error.message),
        mmds: render_diagram(input, OutputFormat::Mmds, &routed_mmds_config)
            .map_err(|error| error.message),
    }
}

const CATEGORY_BOUNDS_OR_LABEL: &str = "bounds_or_label";
const CATEGORY_PROFILE_METADATA: &str = "profile_metadata";
const CATEGORY_ROUTE_COORDINATES: &str = "route_coordinates";
const CATEGORY_SVG_BYTES: &str = "svg_bytes";
const CATEGORY_UNEXPECTED_STRUCTURAL: &str = "unexpected_structural";

#[derive(Debug, Default)]
struct MmdsClassification {
    bounds_or_label: bool,
    profile_metadata: bool,
    route_coordinates: bool,
    unexpected_structural: bool,
    unexpected_paths: BTreeSet<String>,
}

impl MmdsClassification {
    fn categories(&self) -> Vec<String> {
        let mut categories = Vec::new();
        if self.bounds_or_label {
            categories.push(CATEGORY_BOUNDS_OR_LABEL.to_string());
        }
        if self.route_coordinates {
            categories.push(CATEGORY_ROUTE_COORDINATES.to_string());
        }
        if self.profile_metadata {
            categories.push(CATEGORY_PROFILE_METADATA.to_string());
        }
        if self.unexpected_structural {
            categories.push(CATEGORY_UNEXPECTED_STRUCTURAL.to_string());
        }
        categories
    }
}

fn classify_mmds_changes(left: &str, right: &str) -> MmdsClassification {
    let mut classification = MmdsClassification::default();
    let (Ok(left), Ok(right)) = (
        serde_json::from_str::<Value>(left),
        serde_json::from_str::<Value>(right),
    ) else {
        classification.unexpected_structural = true;
        return classification;
    };

    let mut path = Vec::new();
    classify_value(&left, &right, &mut path, &mut classification);
    classification
}

fn classify_value(
    left: &Value,
    right: &Value,
    path: &mut Vec<String>,
    classification: &mut MmdsClassification,
) {
    match (left, right) {
        (Value::Object(left), Value::Object(right)) => {
            let left_keys = left.keys().collect::<BTreeSet<_>>();
            let right_keys = right.keys().collect::<BTreeSet<_>>();
            if left_keys != right_keys {
                classify_changed_path(path, classification);
            }
            for key in left_keys.intersection(&right_keys) {
                path.push((*key).clone());
                classify_value(&left[*key], &right[*key], path, classification);
                path.pop();
            }
        }
        (Value::Array(left), Value::Array(right)) => {
            if left.len() != right.len() {
                classify_changed_path(path, classification);
            }
            for (index, (left, right)) in left.iter().zip(right).enumerate() {
                path.push(index.to_string());
                classify_value(left, right, path, classification);
                path.pop();
            }
        }
        _ if left == right => {}
        _ => classify_changed_path(path, classification),
    }
}

fn classify_changed_path(path: &[String], classification: &mut MmdsClassification) {
    let joined = path.join(".");
    if joined.contains("org.mmdflux.text-metrics.v1")
        || joined == "profiles"
        || joined.starts_with("profiles.")
    {
        classification.profile_metadata = true;
        return;
    }

    if joined.contains("org.mmdflux.render.text.v1.projection") {
        classification.route_coordinates = true;
        return;
    }

    if path.iter().any(|part| part == "edges")
        && path.iter().any(|part| {
            matches!(
                part.as_str(),
                "path" | "label_position" | "label_rect" | "source_port" | "target_port"
            )
        })
    {
        classification.route_coordinates = true;
        return;
    }

    if path.iter().any(|part| {
        matches!(
            part.as_str(),
            "bounds"
                | "height"
                | "label_rect"
                | "metadata"
                | "nodes"
                | "position"
                | "size"
                | "subgraphs"
                | "width"
                | "x"
                | "y"
        )
    }) {
        classification.bounds_or_label = true;
        return;
    }

    classification.unexpected_structural = true;
    classification.unexpected_paths.insert(joined);
}

fn display_path(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[derive(Debug, Serialize)]
struct ChurnReport {
    profiles: ChurnProfiles,
    summary: ChurnSummary,
    fixtures: Vec<FixtureChurn>,
}

#[derive(Debug, Serialize)]
struct ChurnProfiles {
    compatibility: &'static str,
    recorded: &'static str,
}

#[derive(Debug, Default, Serialize)]
struct ChurnSummary {
    fixtures_compared: usize,
    changed_svg_fixtures: usize,
    changed_mmds_fixtures: usize,
    bounds_or_label_changes: usize,
    route_coordinate_changes: usize,
    profile_metadata_changes: usize,
    unexpected_structural_changes: usize,
    render_errors: usize,
}

#[derive(Debug, Serialize)]
struct FixtureChurn {
    fixture: String,
    diagram_family: String,
    svg_changed: bool,
    mmds_changed: bool,
    categories: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    unexpected_paths: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    render_error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_defaults_output_to_none() {
        let options = parse_text_metrics_churn_args(["text-metrics-churn"]).unwrap();

        assert_eq!(options.output, None);
    }

    #[test]
    fn parse_explicit_output_path() {
        let options =
            parse_text_metrics_churn_args(["text-metrics-churn", "--output", "target/report.json"])
                .unwrap();

        assert_eq!(options.output, Some(PathBuf::from("target/report.json")));
    }
}
