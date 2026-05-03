use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

const DEFAULT_SOURCE: &str = "docs/assets/readme/at-a-glance.mmd";
const DEFAULT_OUT_DIR: &str = "docs/assets/readme";
const DEFAULT_NAME: &str = "at-a-glance";
const LAYOUT_ENGINE: &str = "flux-layered";
const EDGE_PRESET: &str = "smooth-step";
const GEOMETRY_LEVEL: &str = "routed";
const PATH_SIMPLIFICATION: &str = "none";

const SOURCE_MARKER: &str = "source";
const TEXT_MARKER: &str = "text";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReadmeUpdate {
    Auto,
    Yes,
    No,
}

#[derive(Debug)]
pub(crate) struct ReadmeAssetsOptions {
    source: PathBuf,
    out_dir: PathBuf,
    name: String,
    mmdflux_bin: Option<PathBuf>,
    readme: ReadmeUpdate,
    check: bool,
}

struct ResolvedOptions {
    repo_root: PathBuf,
    source: PathBuf,
    out_dir: PathBuf,
    name: String,
    mmdflux_bin: Option<PathBuf>,
    readme: ReadmeUpdate,
    check: bool,
}

struct AssetPaths {
    mmd: PathBuf,
    text: PathBuf,
    svg: PathBuf,
    svg_light: PathBuf,
    svg_dark: PathBuf,
    mmds: PathBuf,
}

pub(crate) fn parse_readme_assets_args<I, S>(args: I) -> Result<ReadmeAssetsOptions>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut args = args.into_iter();
    match args.next() {
        Some(first) if first.as_ref() == "readme-assets" => {}
        Some(first) => bail!(
            "expected `cargo xtask readme-assets`, got `cargo xtask {}`",
            first.as_ref()
        ),
        None => bail!("missing `cargo xtask readme-assets` invocation"),
    }

    let mut options = ReadmeAssetsOptions {
        source: PathBuf::from(DEFAULT_SOURCE),
        out_dir: PathBuf::from(DEFAULT_OUT_DIR),
        name: DEFAULT_NAME.to_string(),
        mmdflux_bin: env_mmdflux_bin(),
        readme: ReadmeUpdate::Auto,
        check: false,
    };

    while let Some(arg) = args.next() {
        match arg.as_ref() {
            "-s" | "--source" => {
                options.source = PathBuf::from(next_arg(&mut args, arg.as_ref())?);
            }
            "-o" | "--out-dir" => {
                options.out_dir = PathBuf::from(next_arg(&mut args, arg.as_ref())?);
            }
            "-n" | "--name" => {
                options.name = next_arg(&mut args, arg.as_ref())?;
            }
            "--mmdflux-bin" => {
                options.mmdflux_bin = Some(PathBuf::from(next_arg(&mut args, arg.as_ref())?));
            }
            "--readme" => options.readme = ReadmeUpdate::Yes,
            "--no-readme" => options.readme = ReadmeUpdate::No,
            "--check" => options.check = true,
            other => bail!("unknown `cargo xtask readme-assets` argument `{other}`"),
        }
    }

    Ok(options)
}

fn next_arg<I, S>(args: &mut I, flag: &str) -> Result<String>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    args.next()
        .map(|value| value.as_ref().to_string())
        .ok_or_else(|| anyhow::anyhow!("missing value for `{flag}`"))
}

fn env_mmdflux_bin() -> Option<PathBuf> {
    std::env::var_os("MMDFLUX_BIN")
        .filter(|value| !value.as_os_str().is_empty())
        .map(PathBuf::from)
}

pub(crate) fn run(options: ReadmeAssetsOptions) -> Result<()> {
    let options = options.resolve()?;
    let paths = AssetPaths::new(&options.out_dir, &options.name);
    let mut changed = Vec::new();

    if !options.check {
        fs::create_dir_all(&options.out_dir).with_context(|| {
            format!(
                "failed to create README asset output directory `{}`",
                options.out_dir.display()
            )
        })?;
    }

    println!("Refreshing README assets:");
    println!(
        "  source: {}",
        display_path(&options.repo_root, &options.source)
    );
    println!(
        "  output: {}",
        display_path(&options.repo_root, &options.out_dir)
    );
    if options.check {
        println!("  mode: check");
    }
    println!();

    let source = fs::read(&options.source)
        .with_context(|| format!("failed to read source file `{}`", options.source.display()))?;

    if options.source != paths.mmd {
        write_or_check(
            &options.repo_root,
            &paths.mmd,
            &source,
            options.check,
            true,
            &mut changed,
        )?;
    }

    let text = run_mmdflux_capture(
        &options,
        "text output",
        [
            "--format".into(),
            "text".into(),
            options.source.as_os_str().to_os_string(),
        ],
    )?;
    write_or_check(
        &options.repo_root,
        &paths.text,
        &text,
        options.check,
        true,
        &mut changed,
    )?;

    let mmds = run_mmdflux_capture(
        &options,
        "MMDS output",
        [
            "--format".into(),
            "mmds".into(),
            "--layout-engine".into(),
            LAYOUT_ENGINE.into(),
            "--geometry-level".into(),
            GEOMETRY_LEVEL.into(),
            "--path-simplification".into(),
            PATH_SIMPLIFICATION.into(),
            options.source.as_os_str().to_os_string(),
        ],
    )?;
    write_or_check(
        &options.repo_root,
        &paths.mmds,
        &mmds,
        options.check,
        true,
        &mut changed,
    )?;

    let svg_raw = run_mmdflux_capture(
        &options,
        "SVG output",
        [
            "--format".into(),
            "svg".into(),
            "--layout-engine".into(),
            LAYOUT_ENGINE.into(),
            "--edge-preset".into(),
            EDGE_PRESET.into(),
            options.source.as_os_str().to_os_string(),
        ],
    )?;
    let svg_raw = String::from_utf8(svg_raw).context("mmdflux emitted non-UTF-8 SVG output")?;
    let svg_light = svg_raw.replace(
        "background-color: transparent;",
        "background-color: #ffffff;",
    );
    let svg_dark = svg_raw
        .replace(
            "background-color: transparent;",
            "background-color: #0d1117;",
        )
        .replace("background-color: #ffffff;", "background-color: #0d1117;")
        .replace("fill=\"white\"", "fill=\"#161b22\"")
        .replace("fill=\"#333\"", "fill=\"#e6edf3\"")
        .replace("stroke=\"#333\"", "stroke=\"#8b949e\"");

    write_or_check(
        &options.repo_root,
        &paths.svg_light,
        svg_light.as_bytes(),
        options.check,
        true,
        &mut changed,
    )?;
    write_or_check(
        &options.repo_root,
        &paths.svg_dark,
        svg_dark.as_bytes(),
        options.check,
        true,
        &mut changed,
    )?;
    write_or_check(
        &options.repo_root,
        &paths.svg,
        svg_light.as_bytes(),
        options.check,
        true,
        &mut changed,
    )?;

    if should_update_readme(options.readme, &options.name) {
        sync_readme(&options, &source, &text, &mut changed)?;
    }

    if options.check {
        if changed.is_empty() {
            println!("README assets are up to date.");
            return Ok(());
        }

        for path in &changed {
            println!("  stale: {path}");
        }
        bail!("README assets are out of date; run `cargo xtask readme-assets`");
    }

    println!("Wrote:");
    for path in [
        &paths.mmd,
        &paths.text,
        &paths.svg,
        &paths.svg_light,
        &paths.svg_dark,
        &paths.mmds,
    ] {
        print_written_file(&options.repo_root, path)?;
    }
    if should_update_readme(options.readme, &options.name) {
        print_written_file(&options.repo_root, &options.repo_root.join("README.md"))?;
    }

    Ok(())
}

impl ReadmeAssetsOptions {
    fn resolve(self) -> Result<ResolvedOptions> {
        let repo_root = crate::repo_root();
        let source = resolve_path(&repo_root, &self.source);
        let out_dir = resolve_path(&repo_root, &self.out_dir);
        let mmdflux_bin = self.mmdflux_bin.map(|path| resolve_path(&repo_root, &path));

        if !source.is_file() {
            bail!("source file not found: {}", source.display());
        }

        if let Some(bin) = &mmdflux_bin {
            validate_mmdflux_bin(bin)?;
        }

        Ok(ResolvedOptions {
            repo_root,
            source,
            out_dir,
            name: self.name,
            mmdflux_bin,
            readme: self.readme,
            check: self.check,
        })
    }
}

impl AssetPaths {
    fn new(out_dir: &Path, name: &str) -> Self {
        Self {
            mmd: out_dir.join(format!("{name}.mmd")),
            text: out_dir.join(format!("{name}.txt")),
            svg: out_dir.join(format!("{name}.svg")),
            svg_light: out_dir.join(format!("{name}-light.svg")),
            svg_dark: out_dir.join(format!("{name}-dark.svg")),
            mmds: out_dir.join(format!("{name}.mmds.json")),
        }
    }
}

pub(crate) fn help_text() -> &'static str {
    "\
cargo xtask readme-assets [options]

Refresh README showcase assets from a Mermaid source diagram.

Options:
    -s, --source <path>       Mermaid input file (.mmd)
    -o, --out-dir <path>      Output directory (default: docs/assets/readme)
    -n, --name <name>         Output basename (default: at-a-glance)
        --mmdflux-bin <path>  Use a prebuilt mmdflux binary instead of cargo run
        --readme              Also update marked blocks in README.md
        --no-readme           Skip updating README.md
        --check               Check generated assets and README without writing

Environment:
    MMDFLUX_BIN               Same as --mmdflux-bin"
}

fn resolve_path(repo_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    }
}

fn validate_mmdflux_bin(path: &Path) -> Result<()> {
    if !path.is_file() {
        bail!("mmdflux binary not found: {}", path.display());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mode = fs::metadata(path)
            .with_context(|| format!("failed to inspect `{}`", path.display()))?
            .permissions()
            .mode();
        if mode & 0o111 == 0 {
            bail!("mmdflux binary is not executable: {}", path.display());
        }
    }

    Ok(())
}

fn run_mmdflux_capture<I>(options: &ResolvedOptions, label: &str, args: I) -> Result<Vec<u8>>
where
    I: IntoIterator<Item = OsString>,
{
    let mut command = mmdflux_command_for_capture(options);
    command.args(args);

    let output = command
        .output()
        .with_context(|| format!("failed to run mmdflux for {label}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("mmdflux failed while generating {label}: {stderr}");
    }

    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(output.stdout)
}

fn mmdflux_command_for_capture(options: &ResolvedOptions) -> Command {
    if let Some(bin) = &options.mmdflux_bin {
        Command::new(bin)
    } else {
        let mut command = Command::new("cargo");
        command.current_dir(&options.repo_root);
        command.args(["run", "--quiet", "--bin", "mmdflux", "--"]);
        command
    }
}

fn should_update_readme(mode: ReadmeUpdate, name: &str) -> bool {
    match mode {
        ReadmeUpdate::Yes => true,
        ReadmeUpdate::No => false,
        ReadmeUpdate::Auto => name == DEFAULT_NAME,
    }
}

fn sync_readme(
    options: &ResolvedOptions,
    source: &[u8],
    text: &[u8],
    changed: &mut Vec<String>,
) -> Result<()> {
    let readme_path = options.repo_root.join("README.md");
    let readme = fs::read_to_string(&readme_path)
        .with_context(|| format!("failed to read `{}`", readme_path.display()))?;
    let source = std::str::from_utf8(source).context("Mermaid source is not UTF-8")?;
    let text = std::str::from_utf8(text).context("text output is not UTF-8")?;

    let source_block = fenced_block("", source);
    let text_block = fenced_block("text", text);
    let readme = replace_marked_region(&readme, SOURCE_MARKER, &source_block)?;
    let readme = replace_marked_region(&readme, TEXT_MARKER, &text_block)?;

    write_or_check(
        &options.repo_root,
        &readme_path,
        readme.as_bytes(),
        options.check,
        false,
        changed,
    )
}

fn fenced_block(language: &str, contents: &str) -> String {
    let contents = trim_trailing_newlines(contents);
    format!("```{language}\n{contents}\n```")
}

fn trim_trailing_newlines(value: &str) -> &str {
    value.trim_end_matches(['\n', '\r'])
}

fn replace_marked_region(readme: &str, name: &str, replacement: &str) -> Result<String> {
    let begin = marker(name, "begin");
    let end = marker(name, "end");

    ensure_unique_marker(readme, &begin)?;
    ensure_unique_marker(readme, &end)?;

    let begin_start = readme.find(&begin).expect("marker uniqueness checked");
    let begin_line_end = readme[begin_start..]
        .find('\n')
        .map(|offset| begin_start + offset + 1)
        .ok_or_else(|| anyhow::anyhow!("README marker `{begin}` must be on its own line"))?;
    let end_start = readme[begin_line_end..]
        .find(&end)
        .map(|offset| begin_line_end + offset)
        .ok_or_else(|| anyhow::anyhow!("README marker `{end}` must follow `{begin}`"))?;

    if !readme[..end_start].ends_with('\n') {
        bail!("README marker `{end}` must be on its own line");
    }

    let mut updated = String::with_capacity(readme.len() + replacement.len());
    updated.push_str(&readme[..begin_line_end]);
    updated.push_str(replacement);
    updated.push('\n');
    updated.push_str(&readme[end_start..]);
    Ok(updated)
}

fn marker(name: &str, position: &str) -> String {
    format!("<!-- mmdflux-readme-assets:{name} {position} -->")
}

fn ensure_unique_marker(readme: &str, marker: &str) -> Result<()> {
    let count = readme.match_indices(marker).count();
    match count {
        1 => Ok(()),
        0 => bail!("README marker `{marker}` not found"),
        _ => bail!("README marker `{marker}` appears {count} times"),
    }
}

fn write_or_check(
    repo_root: &Path,
    path: &Path,
    contents: &[u8],
    check: bool,
    generated_asset: bool,
    changed: &mut Vec<String>,
) -> Result<()> {
    let relative_path = display_path(repo_root, path);
    if fs::read(path).is_ok_and(|current| current == contents) {
        return Ok(());
    }

    changed.push(relative_path);
    if check {
        return Ok(());
    }

    let tmp = temp_path_near(path);
    fs::write(&tmp, contents)
        .with_context(|| format!("failed to write temporary file `{}`", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| {
        format!(
            "failed to replace `{}` with `{}`",
            path.display(),
            tmp.display()
        )
    })?;

    if generated_asset {
        set_generated_file_permissions(path)?;
    }

    Ok(())
}

fn temp_path_near(path: &Path) -> PathBuf {
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("readme-assets");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    path.with_file_name(format!(".{filename}.tmp.{}.{nonce}", std::process::id()))
}

#[cfg(unix)]
fn set_generated_file_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)
        .with_context(|| format!("failed to inspect `{}`", path.display()))?
        .permissions();
    permissions.set_mode(0o644);
    fs::set_permissions(path, permissions)
        .with_context(|| format!("failed to chmod `{}`", path.display()))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_generated_file_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

fn print_written_file(repo_root: &Path, path: &Path) -> Result<()> {
    let size = fs::metadata(path)
        .with_context(|| format!("failed to inspect `{}`", path.display()))?
        .len();
    println!(
        "  {:>8} {}",
        human_size(size),
        display_path(repo_root, path)
    );
    Ok(())
}

fn human_size(size: u64) -> String {
    if size < 1024 {
        return format!("{size}B");
    }

    let kib = size as f64 / 1024.0;
    if kib < 1024.0 {
        return format!("{kib:.1}K");
    }

    let mib = kib / 1024.0;
    format!("{mib:.1}M")
}

fn display_path(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    #[test]
    fn replace_marked_region_replaces_only_marked_content() {
        let readme = "\
before
<!-- mmdflux-readme-assets:text begin -->
old
<!-- mmdflux-readme-assets:text end -->
after
";

        let updated = replace_marked_region(readme, "text", "new").unwrap();

        assert_eq!(
            updated,
            "\
before
<!-- mmdflux-readme-assets:text begin -->
new
<!-- mmdflux-readme-assets:text end -->
after
"
        );
    }

    #[test]
    fn replace_marked_region_rejects_missing_markers() {
        let err = replace_marked_region("README", "text", "new").unwrap_err();

        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn replace_marked_region_rejects_duplicate_markers() {
        let readme = "\
<!-- mmdflux-readme-assets:text begin -->
old
<!-- mmdflux-readme-assets:text end -->
<!-- mmdflux-readme-assets:text begin -->
old
<!-- mmdflux-readme-assets:text end -->
";

        let err = replace_marked_region(readme, "text", "new").unwrap_err();

        assert!(err.to_string().contains("appears 2 times"));
    }

    #[test]
    fn fenced_block_trims_extra_trailing_newlines() {
        assert_eq!(fenced_block("text", "a\n\n"), "```text\na\n```");
    }

    #[test]
    fn tracing_mmdflux_log_is_left_for_readme_assets_child() {
        let options = ResolvedOptions {
            repo_root: PathBuf::from("/tmp/mmdflux"),
            source: PathBuf::from("/tmp/mmdflux/input.mmd"),
            out_dir: PathBuf::from("/tmp/mmdflux/out"),
            name: "fixture".to_string(),
            mmdflux_bin: Some(PathBuf::from("/bin/echo")),
            readme: ReadmeUpdate::No,
            check: true,
        };

        let command = mmdflux_command_for_capture(&options);
        let removes_mmdflux_log = command
            .get_envs()
            .any(|(key, value)| key == OsStr::new("MMDFLUX_LOG") && value.is_none());

        assert!(!removes_mmdflux_log);
    }
}
