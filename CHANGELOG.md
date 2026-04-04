# Changelog

- - -
## [mmdflux-v2.1.0](https://github.com/kevinswiber/mmdflux/compare/f68ce6bc8119ee229b129ecf7961f21d775f39d4..mmdflux-v2.1.0) - 2026-04-04
#### Features
- (**mmds-tldraw**) add state diagram support - ([7bf1fa8](https://github.com/kevinswiber/mmdflux/commit/7bf1fa8eace5f2b64f8b985552951a5cfb42cde4)) - [@kevinswiber](https://github.com/kevinswiber)
- add svg theme compatibility - ([be39160](https://github.com/kevinswiber/mmdflux/commit/be39160dc093ac7a12ad6ed0cc9326b76a8dfd42)) - [@kevinswiber](https://github.com/kevinswiber)
- add sequence diagram MMDS output and tldraw adapter - ([1a94180](https://github.com/kevinswiber/mmdflux/commit/1a94180f40d461ffc077f484db453f67576f8ac7)) - [@kevinswiber](https://github.com/kevinswiber)
- add sequence participant lifecycle - ([ebd44cb](https://github.com/kevinswiber/mmdflux/commit/ebd44cb44c68473d126d8681ce4de71c7cf6fe1f)) - [@kevinswiber](https://github.com/kevinswiber)
- add dashed edge style for note constraint edges - ([ad8193c](https://github.com/kevinswiber/mmdflux/commit/ad8193c65769be0358cd198439c1a2731edc4a77)) - [@kevinswiber](https://github.com/kevinswiber)
- add note support for state diagrams - ([ebacae8](https://github.com/kevinswiber/mmdflux/commit/ebacae807a9a1d1576e7db2a67452505387ac094)) - [@kevinswiber](https://github.com/kevinswiber)
- add sequence title and autonumber controls - ([1ba1987](https://github.com/kevinswiber/mmdflux/commit/1ba19872eb080051b74ccf4f84445bd336db8b6e)) - [@kevinswiber](https://github.com/kevinswiber)
- add sequence participant box grouping - ([8db886c](https://github.com/kevinswiber/mmdflux/commit/8db886c2fa27d6db9061e5fc3810857cf312608e)) - [@kevinswiber](https://github.com/kevinswiber)
- support multi-line state descriptions with two-section box rendering - ([6374733](https://github.com/kevinswiber/mmdflux/commit/6374733045e00cbe30f872a4b71a5e7177bbfc24)) - [@kevinswiber](https://github.com/kevinswiber)
- support bracket stereotype notation and v1 header for state diagrams - ([fbbe87d](https://github.com/kevinswiber/mmdflux/commit/fbbe87dbae4ac5829ce45f891e2f829674d8fba2)) - [@kevinswiber](https://github.com/kevinswiber)
- add more sequence block operators - ([e32b0d3](https://github.com/kevinswiber/mmdflux/commit/e32b0d3b1f07fd8c6494a598e050ad424777a9b4)) - [@kevinswiber](https://github.com/kevinswiber)
- add sequence interaction operators - ([36110dd](https://github.com/kevinswiber/mmdflux/commit/36110dd7728b109285d56d87882d211c737b26b0)) - [@kevinswiber](https://github.com/kevinswiber)
- add SVG output for sequence diagrams - ([4ea04e2](https://github.com/kevinswiber/mmdflux/commit/4ea04e21d5cc835649a778d7d243d1c27d68304e)) - [@kevinswiber](https://github.com/kevinswiber)
- add state diagram support (stateDiagram-v2) (#110) - ([8eda729](https://github.com/kevinswiber/mmdflux/commit/8eda7294a1cf639087b80dc73864ea1705cdbccd)) - [@kevinswiber](https://github.com/kevinswiber)
- add activation support to sequence diagrams - ([425ed43](https://github.com/kevinswiber/mmdflux/commit/425ed433f0e5a63954884fed8e4b4330a204e1f0)) - [@kevinswiber](https://github.com/kevinswiber)
- merge sequence note positioning (#106) - ([0080632](https://github.com/kevinswiber/mmdflux/commit/0080632e7bc71b182e0b9cdc4edb346d7a33161a)) - [@kevinswiber](https://github.com/kevinswiber)
- add note left/right/spanning positioning to sequence diagrams - ([9326d65](https://github.com/kevinswiber/mmdflux/commit/9326d65346c88ab9632c4b4e25912f263697b3a2)) - [@kevinswiber](https://github.com/kevinswiber)
#### Bug Fixes
- accept unicode characters in flowchart identifiers - ([4706687](https://github.com/kevinswiber/mmdflux/commit/47066879d0670c7bec7a3a430869d70b5e164113)) - [@kevinswiber](https://github.com/kevinswiber)
- skip cross-boundary nodes in subgraph bounds computation - ([5090f3a](https://github.com/kevinswiber/mmdflux/commit/5090f3abc2cd4f7f5b71983168ead0e1e2c2b2a9)) - [@kevinswiber](https://github.com/kevinswiber)
- skip compound exit constraints when target re-enters subgraph - ([99181b3](https://github.com/kevinswiber/mmdflux/commit/99181b30d89cdb30589c8889a278e3e6259f9e0e)) - [@kevinswiber](https://github.com/kevinswiber)
- detect rank-reversed edges in compound graphs as backward - ([f969967](https://github.com/kevinswiber/mmdflux/commit/f969967393dcbd9e44b297bc55c7e2d7787bf76e)) - [@kevinswiber](https://github.com/kevinswiber)
- align sequence arrowhead semantics with mermaid - ([15d0c46](https://github.com/kevinswiber/mmdflux/commit/15d0c46fcf351312ca3681b93b0164622618a4e4)) - [@kevinswiber](https://github.com/kevinswiber)
- create note edges inline during statement processing - ([ccd2944](https://github.com/kevinswiber/mmdflux/commit/ccd29443f25ea8f09d41732836a16fa3efc577b0)) - [@kevinswiber](https://github.com/kevinswiber)
- match Mermaid note positioning semantics - ([0c44f66](https://github.com/kevinswiber/mmdflux/commit/0c44f66980bcd1f859250dc56b8cfbd7c1ee4305)) - [@kevinswiber](https://github.com/kevinswiber)
- skip reversed edges in compound exit rank constraints - ([7533dae](https://github.com/kevinswiber/mmdflux/commit/7533daebb221bb55b39717c4a00c61383bfc32b7)) - [@kevinswiber](https://github.com/kevinswiber)
- keep sequence svg fragments inside viewbox - ([e038496](https://github.com/kevinswiber/mmdflux/commit/e038496483dbf45fe9081c16f73e658699991e91)) - [@kevinswiber](https://github.com/kevinswiber)
- increase sequence fragment svg padding - ([4b3b6ab](https://github.com/kevinswiber/mmdflux/commit/4b3b6abf05e3dc6230931b1e9f6ed0562e1fb27c)) - [@kevinswiber](https://github.com/kevinswiber)
- refine sequence fragment svg headers - ([8b78aea](https://github.com/kevinswiber/mmdflux/commit/8b78aea5df98a48906c96e8b6c64439287eeefd6)) - [@kevinswiber](https://github.com/kevinswiber)
- deterministic node ordering in direction-override sublayouts - ([916190d](https://github.com/kevinswiber/mmdflux/commit/916190dfc0568f5d5df90f595483fcb556db31ff)) - [@kevinswiber](https://github.com/kevinswiber)
- align glyph nodes with edge connectors in text rendering - ([8833969](https://github.com/kevinswiber/mmdflux/commit/88339698d6ed423b368b2e6e7b9f7f51852ddbe9)) - [@kevinswiber](https://github.com/kevinswiber)
- use rendered path for label bounds on re-routed edges - ([e7bab90](https://github.com/kevinswiber/mmdflux/commit/e7bab90f9bf6ffad37fade6351203f05fb11d7a2)) - [@kevinswiber](https://github.com/kevinswiber)
- constrain backward edge routing to parent subgraph bounds - ([14d15cc](https://github.com/kevinswiber/mmdflux/commit/14d15cc0370917be8f336bf6ad1a39241068ee1f)) - [@kevinswiber](https://github.com/kevinswiber)
- accept TD as direction synonym for TB in state diagrams - ([468fffb](https://github.com/kevinswiber/mmdflux/commit/468fffb0bbc598e9827776e5b690610d4671832b)) - [@kevinswiber](https://github.com/kevinswiber)
- enable waypoint routing for subgraph-as-node edges (#127) - ([ae7fc34](https://github.com/kevinswiber/mmdflux/commit/ae7fc3438e951ea3132c0b40b00087b50f31b1af)) - [@kevinswiber](https://github.com/kevinswiber)
- exclude nesting edges from LayoutResult edge output - ([d6329c8](https://github.com/kevinswiber/mmdflux/commit/d6329c8e629f7e457255183e017bfe0c9c90aab1)) - [@kevinswiber](https://github.com/kevinswiber)
- run subgraph centering after bound expansion - ([f6f5485](https://github.com/kevinswiber/mmdflux/commit/f6f5485e8679aa07510377d1bdcbfa1b6ddab738)) - [@kevinswiber](https://github.com/kevinswiber)
- add exit rank constraints for compound graph subgraphs (#123) - ([3efcd7a](https://github.com/kevinswiber/mmdflux/commit/3efcd7ac6fb4566806a22bc992197dab7fa37e0f)) - [@kevinswiber](https://github.com/kevinswiber)
- center external nodes independently for subgraph-as-node (#120) - ([3b9f1ac](https://github.com/kevinswiber/mmdflux/commit/3b9f1ac498aa00823e2da0e685fe332f1f6246d6)) - [@kevinswiber](https://github.com/kevinswiber)
- skip mermaid direction alternation for state diagrams (#118) - ([7e3de6b](https://github.com/kevinswiber/mmdflux/commit/7e3de6b77400ba2363ec81e28e21bf567d205db2)) - [@kevinswiber](https://github.com/kevinswiber)
- architecture host worktree support (#115) - ([974078f](https://github.com/kevinswiber/mmdflux/commit/974078f18131567e9140b51120e1d62a4c62a5ff)) - [@kevinswiber](https://github.com/kevinswiber)
- sequence parser drops common arrow types (-> --> -x -) variants) - ([e56d430](https://github.com/kevinswiber/mmdflux/commit/e56d43084ef353873706eab5be3104b13c1aeaa1)) - [@kevinswiber](https://github.com/kevinswiber)
- overflow saturated left-face attachments to adjacent faces in LR text routing - ([58e2de3](https://github.com/kevinswiber/mmdflux/commit/58e2de3d25cb0344dde0bfb1216d96b18c1edf4d)) - [@kevinswiber](https://github.com/kevinswiber)
- clean up lr svg orthogonal jogs and overlap artifacts - ([38d5aa0](https://github.com/kevinswiber/mmdflux/commit/38d5aa0246795003a037f7308fad9422e8ebda98)) - [@kevinswiber](https://github.com/kevinswiber)
#### Documentation
- add built-in SVG theme list and beautiful-mermaid attribution - ([c072ed4](https://github.com/kevinswiber/mmdflux/commit/c072ed41e962197529fe16ae3088481c32eb9a7e)) - [@kevinswiber](https://github.com/kevinswiber)
- fix v2.0.2 changelog to use remote template with GitHub links - ([7c8a903](https://github.com/kevinswiber/mmdflux/commit/7c8a903b1db2342a25f5ce772c3f527bfb1db7c7)) - [@kevinswiber](https://github.com/kevinswiber)
- add CLI commands and prerequisites to release and setup guides - ([f68ce6b](https://github.com/kevinswiber/mmdflux/commit/f68ce6bc8119ee229b129ecf7961f21d775f39d4)) - [@kevinswiber](https://github.com/kevinswiber)
#### Refactoring
- change the default curve style to be linear-rounded (smooth-step) in RenderConfig - ([6f1170f](https://github.com/kevinswiber/mmdflux/commit/6f1170f828a907bc26f587ad7018b16d4f8cb333)) - [@kevinswiber](https://github.com/kevinswiber)

- - -

## [mmdflux-v2.0.2](https://github.com/kevinswiber/mmdflux/compare/mmdflux-v2.0.1..mmdflux-v2.0.2) - 2026-03-26
#### Bug Fixes
- render class diagram endpoint cardinality labels - ([430eac9](https://github.com/kevinswiber/mmdflux/commit/430eac96e7f8b3af714abff75ac4fe5e13ea4e51)) - [@kevinswiber](https://github.com/kevinswiber)
- anchor lollipop labels to markers and use hollow circle symbol - ([d22d688](https://github.com/kevinswiber/mmdflux/commit/d22d688003dacc27c0aa301a34c11b169f2cb812)) - [@kevinswiber](https://github.com/kevinswiber)
- render class diagram display labels instead of internal identifiers - ([d002c6b](https://github.com/kevinswiber/mmdflux/commit/d002c6bb38782c8ccb0b521b0c2be0d4321e9186)) - [@kevinswiber](https://github.com/kevinswiber)
- widen SVG fan-in entry point spread on angular targets - ([9fabc70](https://github.com/kevinswiber/mmdflux/commit/9fabc70698bb476f08f2e862f535e27b76b52c04)) - [@kevinswiber](https://github.com/kevinswiber)
- close stacked-document shape bottom border with corner character - ([ed5e468](https://github.com/kevinswiber/mmdflux/commit/ed5e468a0ead906cd7b1920836419c6c16574dd9)) - [@kevinswiber](https://github.com/kevinswiber)
- layout nested mixed-direction override subgraphs with proper spacing - ([97a1f0e](https://github.com/kevinswiber/mmdflux/commit/97a1f0e78f046f81b7facd7d97fd0fc2f5cf626d)) - [@kevinswiber](https://github.com/kevinswiber)
- center multi-line node labels within box - ([aa43df9](https://github.com/kevinswiber/mmdflux/commit/aa43df9cc90e98de0cc7fd3a5b9c3354d022b350)) - [@kevinswiber](https://github.com/kevinswiber)
- separate LR criss-cross draw paths at the grid derivation level - ([97730e9](https://github.com/kevinswiber/mmdflux/commit/97730e9f812bb939ae2660f272dbe5930e53d521)) - [@kevinswiber](https://github.com/kevinswiber)
- separate overlapping parallel vertical segments and cap left/right face extension - ([332eb33](https://github.com/kevinswiber/mmdflux/commit/332eb330e399f9e6bf0cde4e46de8d84bcc73b95)) - [@kevinswiber](https://github.com/kevinswiber)
- simplify terminal dip patterns to prevent shared edge segments - ([af3e17d](https://github.com/kevinswiber/mmdflux/commit/af3e17d57652f4de4676f3f259f8964da223c954)) - [@kevinswiber](https://github.com/kevinswiber)
- prevent backward corridor detour for forward edges with face-rejected draw paths - ([8f10d54](https://github.com/kevinswiber/mmdflux/commit/8f10d54761aa9ce68ee757e41754c94b289e322f)) - [@kevinswiber](https://github.com/kevinswiber)
- spread dense fan-in arrowheads beyond narrow node faces - ([d232cdb](https://github.com/kevinswiber/mmdflux/commit/d232cdb9fa5080a120d2539b25b1ccce26697bf5)) - [@kevinswiber](https://github.com/kevinswiber)
- ignore short perpendicular draw-path terminal steps for face inference - ([ed97dd3](https://github.com/kevinswiber/mmdflux/commit/ed97dd38f850806b6f73970a9b5ed1215b16bb06)) - [@kevinswiber](https://github.com/kevinswiber)
- connect terminal segments to arrows via L-shaped diversion - ([3aa0d79](https://github.com/kevinswiber/mmdflux/commit/3aa0d798519fa25c0463ac99dfb40225a20d82f8)) - [@kevinswiber](https://github.com/kevinswiber)
- derive entry direction from target face instead of segment geometry - ([e379fa2](https://github.com/kevinswiber/mmdflux/commit/e379fa2b7353d774564f2d0f04e692401b900dbf)) - [@kevinswiber](https://github.com/kevinswiber)
- preserve edge lanes while clearing border collisions - ([c344d74](https://github.com/kevinswiber/mmdflux/commit/c344d749189b874ba13edacb3bcf1d02f11893e6)) - [@kevinswiber](https://github.com/kevinswiber)
#### Documentation
- configure cocogitto changelog generation and update release process - ([25d6cc2](https://github.com/kevinswiber/mmdflux/commit/25d6cc2284b0f41842b6f4ac4006c2673336773e)) - [@kevinswiber](https://github.com/kevinswiber)
- add class and sequence snapshot regeneration commands to AGENTS.md - ([d443d44](https://github.com/kevinswiber/mmdflux/commit/d443d444908a11f624ae3064813bc07d9a0c6595)) - [@kevinswiber](https://github.com/kevinswiber)

- - -


## v2.0.1

### Fixed

- Fixed LR/RL forward orthogonal edges routing through unrelated node interiors
  on dense architecture-style graphs. A new general-purpose scan-and-reroute
  pass (`avoid_forward_node_intrusions`) detours edges around non-endpoint
  blockers for any direction and path length.
- Fixed LR/RL terminal face-normal support being invalidated by forward
  reroutes. A post-reroute enforcement step re-applies the terminal stem
  contract when the approach direction is wrong, scoped to LR/RL to avoid
  TD/BT regressions.
- Fixed forward orthogonal edges overshooting past their target and hairpinning
  back (primary-axis reversal), including paths that exited the SVG viewBox
  (e.g. `diagrams → errors` reaching `y < 0` on the architecture graph).
  A reversal-collapse pass removes the overshoot loop post-construction.

## v2.0.0

### Breaking

- Complete restructuring of the crate's module layout and public API.
  All import paths have changed. The public surface is now a curated
  three-tier contract: runtime facade (`render_diagram`, `detect_diagram`,
  `validate_diagram`), low-level API (`builtins`, `registry`,
  `prepared`, `mmds`), and internal implementation modules.
- `src/parser/` moved to `src/mermaid/`.
- `src/layered/` moved to `src/engines/graph/algorithms/layered/`.
- `src/diagram.rs` split into `src/config.rs`, `src/format.rs`,
  `src/errors.rs`, `src/family.rs`, `src/diagnostics.rs`.
- Rendering code moved from `src/diagrams/flowchart/render/` to
  `src/render/graph/` (shared) and `src/render/diagram/` (family-local).
- `src/mmds.rs` expanded to `src/mmds/` directory module.
- `src/lint.rs` removed; validation logic moved to
  `src/diagrams/flowchart/validation.rs` via `DiagramInstance::validation_warnings`.

### Added

- `DiagramInstance::validation_warnings` trait method for diagram-type-specific
  validation through the registry pipeline.
- `PreparedDiagram` contract as the seam between diagram compilation and
  rendering dispatch.
- Architecture guard tests (`tests/architecture_guards.rs`) enforcing module
  boundaries in both production and test code.
- `docs/architecture/dependency-rules.md` with 18 ownership and dependency rules.
- `docs/architecture/deferred-friction.md` tracking 5 monitored items.

## v1.4.0

### Fixed

- Fixed MMDS node positions using top-left origin instead of center-point
  coordinates. MMDS output now emits node centers per the spec, and hydration
  correctly converts centers back to the internal top-left rectangle origin.
- Fixed supported flowchart `style` statements still being reported as
  "parsed but ignored" in lint and WASM validation; only unsupported style
  properties now emit warnings.
- Fixed ANSI text fill backgrounds bleeding onto right node borders during
  transitions back to stroke-only border cells.

- Fixed backward edges detaching from source nodes in SVG when the backward
  path's first horizontal segment fell exactly on the source node's bottom
  boundary.
- Fixed tiny sub-pixel cross-axis jogs on forward edges caused by
  `collapse_tiny_cross_axis_jog` misidentifying short orthogonal segments in
  the SVG orthogonal router.
- Fixed backward edges in LR layouts entering the target node's east face
  instead of the south face when `align_backward_outer_lane_to_hint` pulled
  the outer lane inside node boundaries using layout hint waypoints that pass
  through node centers.
- Fixed `render_svg()` (library/test path) producing different layouts than the
  CLI by replacing hardcoded flux flags with calls to the canonical
  `flux_layout_profile()` and `adapt_flux_profile_for_reversed_chain_crowding()`
  from the engine module.
- Fixed `render_svg()` ignoring `routing_style` when deriving `edge_routing`,
  causing basis and straight preset snapshots to use orthogonal routing paths
  instead of polyline and direct routing respectively.
- Fixed post-quantization text waypoint collisions by repairing whole
  orthogonal segments after snapping, eliminating issue-21-style corridor
  clips that point-only waypoint nudging could miss.
- Fixed text routing parity gaps in LR/RL backward edges and forward long-skip
  edges by reusing validated shared routed paths only when they are nontrivial
  and collision-free, while keeping short backward loops on the text-specific
  fallback path.
- Fixed orthogonal flowchart criss-cross routing in text and SVG so overlapping
  crossings separate more clearly while preserving compact source elbows and
  target-facing terminal arrowhead support.
- Fixed `mermaid-layered` rank assignment diverging from Dagre/Mermaid on
  feedback-cycle graphs by restoring Dagre-compatible network simplex feasible
  tree growth and entering-edge cut selection.

### Added

- Added Mermaid flowchart node `style` support for `fill`, `stroke`, and
  `color`, including parser/builder ingestion, ANSI-capable text/ASCII
  rendering, SVG rendering, and regression fixtures/snapshots for styled
  output.
- Added MMDS node-style round-tripping via the `mmdflux-node-style-v1`
  profile and the `org.mmdflux.node-style.v1` extension namespace.
- Added `NO_COLOR` support for default text/ASCII color suppression; explicit
  `--color` still overrides `NO_COLOR` for per-invocation control.
- Added web playground text preview modes (`Plain`, `Styled`, `ANSI`) with
  copy actions and share/local-state persistence for the selected preview
  mode.
- Added `scripts/svg-gallery-diff` for side-by-side before/after HTML gallery
  of changed SVG snapshots versus a base ref.
- Added Dagre parity fixtures and regression coverage for the
  `callgraph_feedback_cycle` feedback-cycle layout, along with text and SVG
  snapshots for the new flowchart fixture.
- Added `criss_cross.mmd` flowchart fixture coverage, including text and SVG
  regression snapshots for the orthogonal-routing overlap case.

## v1.3.1

### Added

- Added `--version` flag to the CLI.

## v1.3.0

### Breaking

- Removed edge preset token `bezier`; use `basis` (`--edge-preset basis`).
- SVG curve control is now a clean-break contract via
  `--curve basis|linear|linear-sharp|linear-rounded`.
- Removed legacy CLI flags `--interpolation-style` and `--corner-style`.
- Removed legacy WASM/web config fields `interpolationStyle` and `cornerStyle`;
  use `curve`.

### Added

- Implemented plan-0088 model-order tie-breaking across layered ordering paths
  to preserve source insertion order deterministically.
- Implemented plan-0089 greedy-switch two-sided post-pass crossing reduction,
  plus crossing baselines and quality regression checks.
- Implemented plan-0090 per-gap rank-separation overrides for `flux-layered`
  based on gap edge density and crossing pressure.
- Implemented plan-0091 per-edge label spacing features, including label dummy
  insertion, label side selection, label-layer switching, thickness offset, and
  HEAD/TAIL label support.
- Expanded layout and routing non-regression coverage (ordering, spacing,
  routing topology, and engine behavior).

### Fixed

- Fixed multiple backward-edge routing regressions in text and SVG, including
  corridor-aware channeling, face attachment consistency, and subgraph override
  cases.
- Fixed SVG edge rendering regressions around arrowhead visibility, reciprocal
  two-point curve separation, and shape-border lane attachment.
- Fixed label/spacing regressions in layered layout, including restored
  unlabeled-edge rank separation and corrected label-gap accounting.
- Fixed reversed long-edge chain accounting leakage into forward-edge density
  metrics.

### Changed

- Implemented plan-0092 curve taxonomy clean break and removed transitional
  interpolation bridge behavior in favor of `Curve`.
- Renamed SVG snapshot bucket `flowchart-bezier` to `flowchart-basis`.
- Updated web playground preset vocabulary from `bezier` to `basis`.
- Updated `scripts/svg-gallery` and `scripts/view` defaults/examples to use
  `basis`; `svg-gallery` now also exports fixture source copies.
- Removed web CSS `!important` cursor overrides and rely on panzoom cursor
  config and normal cascade precedence.

## v1.2.0

### Added

- `mermaid-layered` engine now ignores subgraph `direction` overrides when the
  subgraph has cross-boundary edges, matching Mermaid.js/dagre behavior.
  `flux-layered` continues to always respect direction overrides.

### Fixed

- Sibling subgraph bounds no longer overlap after sublayout reconciliation.
- Added margin between adjacent subgraph borders for visual breathing room.
- Text backward-edge routing now reuses shared routed paths for long TD/BT
  backward edges while preserving text-specific fallback heuristics for short
  cycles, fixing wrong-facing arrowheads and attachment/segment artifacts
  (for example in `complex.mmd` and `multiple_cycles.mmd`).
- SVG polyline rendering no longer injects tiny synthetic jogs on
  axis-to-diagonal turns (for example `ampersand.mmd`) in both
  `flux-layered` and `mermaid-layered`.
- Self-loop tail regression coverage now validates loop-lane drift without
  assuming a fixed elbow index, preventing false failures when valid polyline
  cleanup reduces intermediate points.

### Changed

- Routing semantics: `--edge-preset straight` now means direct routing
  (`Direct + Linear + Sharp`). Use `--edge-preset polyline` for prior straight semantics.
- Direct routing now uses a collision-aware fallback: when a single direct segment
  would cross node interiors, mmdflux preserves node-avoidance geometry.

### Refactor

- Renamed broad `dagre` terminology to `layered` across APIs, internals, and docs
  (plan-0082), including layout/routing config names and layered hint types.
- Reorganized `src/diagrams/flowchart/render/` to clearly separate text, SVG, and
  shared modules ([#13](https://github.com/kevinswiber/mmdflux/pull/13)):
  extracted shared layout building (`layout_building.rs`) and subgraph ops
  (`layout_subgraph_ops.rs`), moved text types to `text_types.rs`, renamed
  `layout.rs` to `text_layout.rs`, and added `text_` prefix to all text-only
  modules for naming symmetry with `svg_*`. Renamed `LayoutConfig` to
  `GridLayoutConfig`.
- `mermaid-layered` engine now only supports SVG and MMDS output, matching
  Mermaid.js which only renders to SVG ([#14](https://github.com/kevinswiber/mmdflux/pull/14)).
  Text/ASCII output uses `flux-layered` exclusively.
