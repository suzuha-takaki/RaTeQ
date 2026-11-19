# Design

Open product decisions are listed below. Final decisions live in the official documentation and are recorded in the log.

## Product Decisions

All current product decisions are resolved.

## Decision Log

| Date | Decision/option | Status | Description / Reason | Official documentation |
|---|---|---|---|---|
| 2026-11-18 | Rust and Ratatui | Adopted | Looks cool | [Implementation](README.md#implementation) |
| 2026-11-18 | ratatui-image | Adopted | High compatibility with ratatui | [Implementation](README.md#implementation), [Graphics](README.md#graphics) |
| 2026-11-18 | Linux and macOS | Adopted | Because I use them | [Implementation](README.md#implementation) |
| 2026-11-18 | Prebuilt binaries | Adopted | macOS and Linux binaries in `.tar.gz` archives. | [Implementation](README.md#implementation) |
| 2026-11-18 | Built-in character-art fallback | Adopted | It comes with ratatui-image | [Graphics](README.md#graphics) |
| 2026-11-18 | Category-based diagnostic IDs | Adopted | Category ranges make related diagnostics identifiable | [Numbering](messages.md#numbering) |
| 2026-11-18 | RaTeX | Adopted | Supports math rendering and PNG/SVG outputs and the repo seems well-managed | [Implementation](README.md#implementation), [Render](editing.md#render), [Export](interactions.md#export) |
| 2026-11-18 | Terminal release targets | Deprecated | Replaced by the WezTerm-focused v1 target and release-time compatibility reporting | [Release Targets](README.md#release-targets) |
| 2026-11-18 | Unavailable Preview | Adopted | Accepted keeping editing, source saves, and export available when display fails | [Graphics](README.md#graphics) |
| 2026-11-18 | Asset packaging | Adopted | Accepted bundled math fonts/assets and per-target archives with license notices | [Distribution](README.md#distribution) |
| 2026-11-18 | Nonexistent input file | Adopted | Accepted creating a named empty source and writing on save | [CLI](session.md#cli) |
| 2026-11-18 | CLI invalid input | Adopted | Accepted rejection of unknown flags, multiple filenames, and conflicting session modes before the UI | [CLI Validation](session.md#cli-validation) |
| 2026-11-18 | Text indicators | Adopted | Accepted short labels for the five Preview statuses | [Preview Status](editing.md#preview-status) |
| 2026-11-18 | Resume behavior | Adopted | Accepted storing the last successful source save and reporting unavailable history/files | [Resume](session.md#resume) |
| 2026-11-18 | Export precondition | Adopted | Accepted requiring a successful render before submission | [Export](interactions.md#export) |
| 2026-11-18 | Older-preview export | Adopted | Accepted exporting the retained rendered result with a warning | [Export](interactions.md#export) |
| 2026-11-18 | Contextual Help | Adopted | Accepted full-screen scrollable shortcuts, exact caller return, and About | [Help](interactions.md#help) |
| 2026-11-18 | V1 Help destinations | Deprecated | About was dropped because CLI usage and contextual shortcuts are sufficient | [Help](interactions.md#help) |
| 2026-11-18 | Standalone CLI help/version | Adopted | Help/version each require a standalone invocation. Combination errors suggest the intended command. | [CLI Validation](session.md#cli-validation) |
| 2026-11-18 | Layout | Adopted | Accepted Editor above Preview, equal initial window heights, vertical layout in narrow terminals, and accessible bars with a resize warning when space is insufficient | [Layout](session.md#layout) |
| 2026-11-19 | PoC organization | Adopted | Keep experiments in this repository, with a command reference in the PoC README. | [PoCs](../poc/README.md) |
| 2026-11-19 | Release roadmap | Adopted | Use development releases and prereleases leading to the documented 1.0.0 target | [Roadmap](ROADMAP.md) |
| 2026-11-19 | Support documentation | Adopted | Group platform, terminal, and feature support with verification evidence | [Support](README.md#support) |
| 2026-11-19 | Name-list format | Adopted | Store both word lists as named arrays in an embedded TOML asset | [Filename Defaults](interactions.md#filename-defaults) |
| 2026-11-19 | Render scheduling | Adopted | PoC feedback confirmed rendering is fast enough to cancel and restart after source edits | [Render](editing.md#render) |
| 2026-11-19 | Preview line breaking | Adopted | Scroll by default. `-b` / `--break` enables preview-only line breaking. | [Preview Flow](session.md#preview-flow) |
| 2026-11-19 | Preview scale | Adopted | Display at half the export render scale in each dimension | [Render](editing.md#render) |
| 2026-11-19 | V1 compatibility | Adopted | Target WezTerm on macOS and Ubuntu in OrbStack. Record working versions and limitations at release. | [Release Targets](README.md#release-targets) |
| 2026-11-19 | Binary archives | Adopted | Distribute macOS and Linux `.tar.gz` archives | [Distribution](README.md#distribution) |
| 2026-11-19 | Name-list contents | Adopted | Keep empty arrays ready for the user to fill whenever desired. Use `equation` while either list is empty. | [Filename Defaults](interactions.md#filename-defaults) |
| 2026-11-19 | Help scope | Adopted | CLI help explains usage and Ctrl+G shows contextual shortcuts | [CLI Help](session.md#cli-help), [Help](interactions.md#help) |
| 2026-11-19 | Persistence approval | Adopted | Ask before setting up platform-appropriate application directories, with default/custom/cancel choices | [Persistence](session.md#persistence) |
| 2026-11-19 | Persistence setup | Adopted | Ask on first interactive launch, remember the approved location, and exit on Cancel before creating application data. Standalone help/version run directly. | [Persistence](session.md#persistence) |
| 2026-11-19 | Multiline paste | Adopted | Insert pasted newlines as source text, then follow normal manual/live rendering behavior | [Input](editing.md#input) |
| 2026-11-19 | Support location | Adopted | Keep support information in the main README and release planning in ROADMAP | [Support](README.md#support), [Roadmap](ROADMAP.md) |
