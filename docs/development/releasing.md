# Releasing

All releases are managed through [cocogitto](https://docs.cocogitto.io/) (`cog bump --package`). Each package gets a tag in the format `{package}-v{version}` (e.g., `mmdflux-v2.1.0`, `mmds-core-v0.2.0`). Tag pushes trigger the corresponding CI workflows.

This project publishes:

- `mmdflux` crate to crates.io
- Binary release assets to GitHub Releases
- `@mmds/wasm` npm package
- `@mmds/core`, `@mmds/excalidraw`, `@mmds/tldraw` npm packages
- Homebrew formula updates in `kevinswiber/homebrew-mmdflux`

## Tag Format

All tags follow the pattern `{package}-v{version}`:

| Package              | Tag example              | CI workflow                                                |
| -------------------- | ------------------------ | ---------------------------------------------------------- |
| mmdflux (root crate) | `mmdflux-v2.1.0`         | Release + Crate Release + WASM Release + Playground Deploy |
| @mmds/core           | `mmds-core-v0.2.0`       | Packages Release                                           |
| @mmds/excalidraw     | `mmds-excalidraw-v0.2.0` | Packages Release                                           |
| @mmds/tldraw         | `mmds-tldraw-v0.2.0`     | Packages Release                                           |

## Changelogs

Cocogitto generates changelogs automatically during `cog bump`. Each package has its own `CHANGELOG.md`:

| Package          | Changelog path                          |
| ---------------- | --------------------------------------- |
| mmdflux          | `CHANGELOG.md` (repo root)              |
| @mmds/core       | `packages/mmds-core/CHANGELOG.md`       |
| @mmds/excalidraw | `packages/mmds-excalidraw/CHANGELOG.md` |
| @mmds/tldraw     | `packages/mmds-tldraw/CHANGELOG.md`     |

Cocogitto inserts new release entries after the first `- - -` separator in each file, preserving any older hand-written entries below. The root `CHANGELOG.md` contains hand-written entries for versions prior to v2.0.1; these are kept intact beneath the auto-generated sections.

### Previewing a changelog before release

Use `cog changelog` with a range to preview what will be generated:

```bash
# Preview unreleased changes since last mmdflux tag
cog changelog "$(git describe --tags --match 'mmdflux-v*' --abbrev=0)..HEAD"

# Preview changes for a specific adapter package tag
cog changelog "$(git describe --tags --match 'mmds-core-v*' --abbrev=0)..HEAD"
```

This prints to stdout only — no files are modified.

## Root Crate Release (mmdflux + @mmds/wasm)

The root crate and `@mmds/wasm` are version-locked and release together.

### Checklist

1. Ensure `main` is green in CI.

2. Ensure the working tree is clean (no uncommitted or untracked changes). `cog bump` will refuse to run otherwise.

3. Preview the changelog and decide the bump level:

```bash
cog changelog "$(git describe --tags --match 'mmdflux-v*' --abbrev=0)..HEAD"
```

4. Bump with cocogitto:

```bash
# Automatic version based on conventional commits since last tag
cog bump --package mmdflux --auto

# Or specify the bump level
cog bump --package mmdflux --patch   # bug fixes only
cog bump --package mmdflux --minor   # new features
cog bump --package mmdflux --major   # breaking changes
```

This will:

- Run `just lint` and `just test`
- Update version in `Cargo.toml`, `crates/mmdflux-wasm/Cargo.toml`, and `xtask/Cargo.toml`
- Generate the changelog in `CHANGELOG.md`
- Commit, tag (`mmdflux-v{version}`), and push

5. Confirm the four release workflows complete:
   - **Release** — builds binaries and publishes GitHub Release assets
   - **Crate Release** — publishes `mmdflux` to crates.io
   - **WASM Release** — publishes `@mmds/wasm` to npm
   - **Playground Deploy** — deploys web playground to Cloudflare Pages

6. Update the Homebrew formula (see below).

## npm Adapter Packages

The adapter packages (`@mmds/core`, `@mmds/excalidraw`, `@mmds/tldraw`) version independently from the root crate.

### Bumping a Package

```bash
# Automatic version based on scoped conventional commits
cog bump --package mmds-core --auto

# Or specify the bump level
cog bump --package mmds-core --patch
cog bump --package mmds-excalidraw --minor
cog bump --package mmds-tldraw --patch
```

This will:

1. Run `npm version` to update `package.json`
2. Generate a package-specific changelog in the package directory
3. Commit, tag (e.g., `mmds-core-v0.2.0`), and push

The tag push triggers the **Packages Release** workflow, which builds and publishes the package to npm.

### Coordinating @mmds/core Updates

When `@mmds/core` has a breaking change that affects the adapters:

1. Bump `@mmds/core` first: `cog bump --package mmds-core --minor`
2. Update the `@mmds/core` dependency version in `mmds-excalidraw` and `mmds-tldraw` package.json files
3. Run `npm install` in `packages/` to update the lockfile
4. Bump each adapter: `cog bump --package mmds-excalidraw --minor`, etc.

For compatible (patch) changes, the caret range (`^0.x.y`) handles resolution automatically — no adapter bump is needed.

### Trusted Publishing Setup

Each npm package must be configured on [npmjs.com](https://www.npmjs.com/) to trust the GitHub Actions workflow:

- **Repository:** `kevinswiber/mmdflux`
- **Workflow:** `packages-release.yml` (for adapter packages) or `wasm-release.yml` (for `@mmds/wasm`)
- **Environment:** (default, no named environment)

This must be done once per package in the npm package settings under "Publishing access".

## GitHub Release Assets

The release workflow publishes these artifacts:

- `mmdflux-vX.Y.Z-darwin-arm64.tar.gz`
- `mmdflux-vX.Y.Z-darwin-x86_64.tar.gz`
- `mmdflux-vX.Y.Z-linux-x86_64.tar.gz`
- `mmdflux-vX.Y.Z-windows-x86_64.zip`
- `checksums.txt`

## Homebrew Tap

Tap repository:

- [kevinswiber/homebrew-mmdflux](https://github.com/kevinswiber/homebrew-mmdflux)

Install command for users:

```bash
brew tap kevinswiber/mmdflux
brew install mmdflux
```

### Updating Homebrew Formula for a New Release

1. Clone/update tap repo:

```bash
git clone git@github.com:kevinswiber/homebrew-mmdflux.git
cd homebrew-mmdflux
```

2. Pull release checksums:

```bash
gh release download mmdflux-vX.Y.Z --repo kevinswiber/mmdflux --pattern checksums.txt
cat checksums.txt
```

3. Update `Formula/mmdflux.rb` with new version, URLs, and SHA256 values.
4. Commit and push the formula update.
