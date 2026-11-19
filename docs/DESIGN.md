# Design

Open product decisions are listed below. Final decisions live in the official documentation and are recorded in the log.

## Product Decisions

All current product decisions are resolved.

## Decision Log

| Date | Status | Decision | Official documentation |
|---|---|---|---|
| 2026-11-18 | Adopted | Use Rust and Ratatui for the implementation and terminal interface; chosen for their appearance. | [Implementation](README.md#implementation) |
| 2026-11-18 | Adopted | Use ratatui-image for graphics with its built-in character-art fallback. | [Implementation](README.md#implementation), [Graphics](README.md#graphics) |
| 2026-11-18 | Adopted | Support macOS and Linux and distribute prebuilt binaries in `.tar.gz` archives. | [Implementation](README.md#implementation), [Distribution](README.md#distribution) |
| 2026-11-18 | Adopted | Use category-based diagnostic IDs to make related messages identifiable. | [Numbering](messages.md#numbering) |
| 2026-11-18 | Adopted | Use RaTeX for math rendering and PNG/SVG outputs; selected for its supported outputs and maintenance. | [Implementation](README.md#implementation), [Render](editing.md#render), [Export](interactions.md#export) |
| 2026-11-18 | Deprecated | Replace the previous terminal release targets with the WezTerm-focused v1 target and release-time compatibility reporting. | [Release Targets](README.md#release-targets) |
| 2026-11-18 | Adopted | Keep editing, source saves, and export available when Preview display fails; require a successful render for export and warn when exporting an older result. | [Graphics](README.md#graphics), [Export](interactions.md#export) |
| 2026-11-18 | Adopted | Bundle math fonts and assets, and include license notices in per-target archives. | [Distribution](README.md#distribution) |
| 2026-11-18 | Adopted | Open a nonexistent input file as an empty named source and write it on save. | [CLI](session.md#cli) |
| 2026-11-18 | Adopted | Reject unknown flags, multiple filenames, conflicting modes, and combined help/version invocations before opening the UI; suggest standalone help/version commands when appropriate. | [CLI Validation](session.md#cli-validation) |
| 2026-11-18 | Adopted | Show short text labels for the five Preview statuses. | [Preview Status](editing.md#preview-status) |
| 2026-11-18 | Adopted | Resume the last successfully saved source and report unavailable history or files. | [Resume](session.md#resume) |
| 2026-11-18 | Adopted | Provide full-screen, scrollable contextual Help with exact caller return, initially including About. | [Help](interactions.md#help) |
| 2026-11-18 | Deprecated | Drop About from v1 Help because CLI usage and contextual shortcuts are sufficient. | [Help](interactions.md#help) |
| 2026-11-18 | Adopted | Place Editor above Preview with equal initial heights and a vertical layout in narrow terminals; keep bars accessible and warn when space is insufficient. | [Layout](session.md#layout) |
| 2026-11-19 | Adopted | Keep PoC experiments in this repository with a command reference. | [PoC](../poc/README.md) |
| 2026-11-19 | Adopted | Use development releases and prereleases on the path to 1.0.0. | [Roadmap](ROADMAP.md) |
| 2026-11-19 | Adopted | Keep platform, terminal, and feature support evidence in the main README and release planning in the roadmap. | [Support](README.md#support), [Roadmap](ROADMAP.md) |
| 2026-11-19 | Adopted | Store filename word lists as initially empty named arrays in an embedded TOML asset; suggest `equation` while either list is empty. | [Filename Defaults](interactions.md#filename-defaults) |
| 2026-11-19 | Adopted | Cancel in-flight rendering after source edits and, in live mode, restart after the typing delay; PoC feedback showed this was fast enough. | [Render](editing.md#render) |
| 2026-11-19 | Adopted | Scroll previews by default, enable preview-only line breaking with `-b` / `--break`, and display previews at half the export scale in each dimension. | [Preview Flow](session.md#preview-flow), [Render](editing.md#render) |
| 2026-11-19 | Adopted | Target WezTerm on macOS and Ubuntu in OrbStack, recording working versions and limitations at release. | [Release Targets](README.md#release-targets) |
| 2026-11-19 | Adopted | Reaffirm `.tar.gz` as the per-target binary archive format. | [Distribution](README.md#distribution) |
| 2026-11-19 | Adopted | Use CLI help for usage and Ctrl+G for contextual shortcuts. | [CLI Help](session.md#cli-help), [Help](interactions.md#help) |
| 2026-11-19 | Deprecated | Replace mandatory first-launch persistence setup and its original choices with opt-in setup when Resume is requested, including an option to continue without persistence. | [Persistence](session.md#persistence) |
| 2026-11-19 | Adopted | Insert pasted newlines as source text, then follow normal manual or live rendering behavior. | [Input](editing.md#input) |
| 2026-11-19 | Removed | Remove Shift+Enter shortcuts because the development terminal cannot distinguish them from Enter; use Ctrl+N for newlines and cycle forward through Find matches with Enter. | [Input](editing.md#input), [Find](editing.md#find) |
| 2026-11-19 | Adopted | Require 80 columns by 24 rows; below either minimum, show a resize screen while preserving the interaction. | [Layout](session.md#layout), [W403](messages.md#w403) |
| 2026-11-19 | Adopted | Ask for persistence approval only when Resume is requested without an approved location; allow continuing without persistence and require no new CLI option. | [Persistence](session.md#persistence), [Resume](session.md#resume) |
