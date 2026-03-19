## Plan: Ground-Up Subtitle Workspace Rewrite

DRAFT: Reframe `teamy-subs` as a small workspace with a thin CLI crate, a subtitle-domain crate, and a top-level package boundary that keeps command UX separate from file-format logic. Keep current CLI parsing on `figue` + Facet metadata, but remove `vtt`, `serde`, and `serde_json` from subtitle handling. Use Tracey from the start to specify three concerns independently: CLI behavior, VTT/TXT file-format behavior, and cross-cutting conversion/output rules. Treat current `teamy-subs` behavior as the initial compatibility baseline, then broaden it with vendored official WebVTT requirements plus a real-world sample corpus from `ffmpeg`, `yt-dlp`, and Whisper outputs.

## Progress

- [x] Step 1 started: created a Cargo workspace scaffold with `teamy-subs-cli` and `teamy-facet-vtt`
- [x] Step 1 bridge: root `teamy-subs` now delegates through `teamy-subs-cli` while keeping the installed binary/package name stable
- [x] Step 3 started: added Tracey configuration and initial spec partition files for CLI, VTT, TXT, and conversions
- [x] Step 5 started: added `teamy-facet-vtt` with foundational subtitle shape types and a reusable `VttTimestamp`
- [x] Step 5 progressed: moved readable cue payload parsing and TXT projection helpers into `teamy-facet-vtt`
- [x] Step 5 progressed again: `teamy-facet-vtt` now parses and renders native WebVTT documents for `.vtt -> .txt` and `.vtt -> .vtt`
- [x] Step 9 progressed: `teamy-subs-cli` no longer depends on `serde` / `serde_json` for ffprobe subtitle extraction
- [x] Step 1 progressed again: `teamy-subs-cli` now owns local `logging_init` and `paths` modules instead of bridging them from the root source tree
- [x] Validation: `cargo test --workspace` passes after the initial workspace split
- [ ] Source files are temporarily shared into `teamy-subs-cli` via `#[path = ...]` bridging; physical relocation is still pending

## Current checkpoint notes

- The first implementation pass is deliberately non-breaking.
- The new workspace shape exists before parser behavior is rewritten.
- Tracey docs/spec scaffolding exists so future parser work can be requirement-driven.
- The new `teamy-facet-vtt` crate currently models core data shapes rather than parsing full files yet.
- The next major milestone is to move VTT/TXT parsing and projection behavior into `teamy-facet-vtt` and then make `teamy-subs-cli` consume it.

**Steps**
1. Recast teamy-subs into a Cargo workspace modeled after tracey/Cargo.toml, with crate boundaries for `teamy-subs-cli`, `teamy-facet-vtt`, and a small top-level package that owns release metadata and the installed binary name.
2. Freeze the current CLI contract from teamy-subs/src/cli/mod.rs, teamy-subs/src/lib.rs, and teamy-subs/README.md into a Tracey CLI spec first, so command names, argument shapes, and output expectations are captured before any crate split.
3. Create a Tracey configuration for the `teamy-subs` repo patterned after tracey/.config/tracey/config.styx, but partitioned into multiple specs: one for CLI structure, one for VTT, one for TXT, and one for conversion semantics and extensions.
4. Vendor the external behavioral sources that define the program, starting with the formal WebVTT rules and explicitly marking “official” versus “supported extension” requirements in markdown, following the requirement/implementation/verification flow documented in tracey/README.md and exemplified by tracey/docs/content/spec/tracey.md.
5. Define the first-principles data shapes for subtitle interaction in `teamy-facet-vtt`: document/file, header, metadata blocks, cue identifiers, cue timing, cue settings, cue payload fragments, text projections, parse diagnostics, and normalization policies. Base the shape design on current working concepts from vtt/src/lib.rs, vtt/src/cue_payload.rs, and teamy-subs/src/cli/convert/convert_cli.rs, but redesign them as Facet-derived types rather than serde-oriented transport types.
6. Separate “data we must preserve” from “data we expose ergonomically”: keep a loss-aware VTT model for round-tripping/canonicalization and a projected readable-text model for `.vtt -> .txt`, including the timestamped line-prefix policy you chose. This avoids conflating parsing fidelity with human-readable export behavior.
7. Design the parser as staged passes instead of a single ad hoc routine: source normalization, top-level block scanning, cue boundary detection, timing/settings parsing, payload fragment parsing, and readable-text projection. This should explicitly absorb the real-world cases currently papered over in teamy-subs/src/cli/convert/convert_cli.rs: `NOTE`, `STYLE`, `REGION`, cue identifiers, timed fragments, and overlap removal.
8. Define TXT as its own shaped format rather than “just strings”: capture line text, optional coarse timestamp prefix, grouping/paragraph boundaries, and graceful import rules when timing is absent. That lets `.txt -> other` conversions stay deterministic instead of relying on opaque string heuristics.
9. Move subprocess-facing concerns out of subtitle parsing. `teamy-subs-cli` should own `figue`, runtime setup, logging, `yt-dlp`/`ffmpeg` orchestration, and filesystem UX, while `teamy-facet-vtt` stays pure and testable. Current extraction JSON handling in teamy-subs/src/cli/extract/extract_cli.rs should be planned for a non-serde replacement separately from subtitle parsing.
10. Use Facet deliberately in the new domain crate: `#[derive(Facet)]` for all stable shape types, Facet reflection for any schema/debug tooling, and facet ecosystem crates where serialization/parsing support is genuinely needed, per facet/.claude/skills/use-facet-crates/SKILL.md. Keep `figue` for CLI in phase 1 rather than taking on facet-args risk now.
11. Build a sample corpus before parser implementation: include curated fixtures from the current local `vtt` repo, current `teamy-subs` edge cases, `yt-dlp` downloads, `ffmpeg` extractions from anime/movie MKVs, and Whisper outputs. Treat these as permanent repository fixtures and map them to Tracey requirements.
12. Layer testing from the bottom up:
    - unit tests for timestamps, settings, cue payload fragments, and block scanners
    - fixture/golden tests for full-file parse and render behavior
    - round-trip tests for normalization stability
    - projection tests for readable TXT output
    - CLI tests for `convert`, `extract`, and `download`
    - fuzz/property tests similar in spirit to teamy-subs/tests/cli_fuzzing.rs and Facet’s stronger safety tests
13. Define migration checkpoints so the rewrite stays shippable:
    - checkpoint 1: workspace split with no behavior change
    - checkpoint 2: Tracey spec committed and coverage wired up
    - checkpoint 3: `teamy-facet-vtt` parses the supported fixture corpus
    - checkpoint 4: `.vtt -> .txt` and `.vtt -> .vtt` moved off external `vtt`
    - checkpoint 5: remove `vtt`, `serde`, and `serde_json` from teamy-subs/Cargo.toml
14. Treat unsupported or ambiguous syntax as an explicit design surface: decide for each case whether it is rejected, preserved as opaque data, normalized, or accepted under an extension rule. Record that decision in Tracey rather than burying it in parser code.

**Verification**
- Tracey validation on every spec partition using the query flow described in tracey/README.md, especially `status`, `uncovered`, `untested`, and `validate`
- `cargo test` at workspace level and crate level
- golden-fixture assertions for:
  - parse tree shape
  - normalized VTT rendering
  - readable TXT rendering with timestamp prefixes
  - conversion round-trips where intended
- CLI regression checks for current commands from teamy-subs/src/cli/mod.rs
- dependency audit confirming `vtt`, `serde`, and `serde_json` are removed from the subtitle path and ultimately from the workspace where intended

**Decisions**
- CLI parser: keep `figue` + Facet metadata for the first rewrite phase, matching teamy-subs/src/lib.rs and tracey/crates/tracey/src/main.rs
- VTT scope: start from current `teamy-subs` user-visible behavior, then extend with vendored formal spec requirements plus local real-world corpora from `ffmpeg`, `yt-dlp`, and Whisper
- TXT output: optimize for human-readable deduplicated text with fixed-width timestamp prefixes at line granularity
- Spec structure: use multiple Tracey specs/partitions rather than one monolith, separating CLI, VTT, TXT, and cross-cutting conversion semantics