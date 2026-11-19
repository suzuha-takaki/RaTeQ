# RaTeQ v1.0.0

Product specification targeting RaTeQ 1.0.0, a terminal editor for LaTeX math fragments. MIT license. English UI.

Repository [RaTeQ](https://github.com/suzuha-takaki/RaTeQ).

| Document | Defines |
|---|---|
| [Session](session.md) | CLI, file identity, formats, rendering mode, display mode |
| [Editing](editing.md) | Input, Find, Render |
| [Interactions](interactions.md) | Edit, File entry, Export, Exit, Help |
| [Roadmap](ROADMAP.md) | Versioning, development milestones, and release validation |
| [PoC](../poc/README.md) | Commands and what they exercise |
| [Design](DESIGN.md) | Remaining product decisions and decision history |
| [Messages](messages.md) | Numbered warning/error catalog and behavior links |

## Implementation

| Area | Decision |
|---|---|
| Language | Rust |
| Terminal UI | Ratatui |
| Image display | ratatui-image, including its built-in character-art fallback |
| Math rendering | RaTeX |
| Operating systems | macOS and Linux |
| Distribution | macOS and Linux `.tar.gz` binary archives |

## Distribution

- Bundle RaTeX math fonts and assets needed for standalone PNG/SVG output in the distributed binaries.
- Ship per-target `.tar.gz` archives containing the executable and license notices.
- Record the OS and CPU versions used for each release in [Support](#release-targets).

## Components

| Component | Purpose |
|---|---|
| Editor window | Edit source contents |
| Preview window | Display the preview of the retained rendered result |
| LaTeX message bar | Display LaTeX rendering warnings and errors |
| RaTeQ message bar | Display application warnings and errors |
| Entry bars | Accept paths, search words, letters, and other input |
| Navigation bar | Display available keyboard shortcuts |

Single/double window counts the Editor and Preview windows. Message bars, Entry bars, and the Navigation bar accompany those windows.

## Layers

| Layer | Values | Details |
|---|---|---|
| Session settings | Named/unnamed, manual/live, single/double window, scroll/break | CLI establishes initial settings. Saving an unnamed source gives it a name. |
| Editor actions | Find, Render | Actions operate on the source or its rendered result. Find opens an Entry bar. |
| Interaction states | Edit, File entry, Export, Exit, Help | Determine the current Entry bar and Navigation bar. |

In single-window mode, the active window is Editor or Preview. Interaction tables include session-dependent behavior.

## Terminology

| Term | Meaning |
|---|---|
| Source | The LaTeX text being edited. |
| Source file | A file containing source. An unnamed source has no associated source file. |
| Source path | The path currently associated with the source, if any. |
| Launch file | The source file opened at the start of the session, including through resume. It remains the naming comparison baseline. |
| Render | The operation that produces output from a particular source revision. |
| Rendered result | The output of a successful render, associated with the source revision that produced it. It can exist without terminal display. The retained result is the last successful result. |
| Preview | The presentation of the retained rendered result in the Preview window. |
| Save source | Write the current source to a file. |
| Export | Write the retained rendered result as PNG or SVG. |
| Interaction | A UI state such as Edit, File entry, Export, Exit, or Help. Find is an editor action with its own entry bar. |
| File entry | The shared path-entry interaction used to save source or write an export. |
| Active window | The Editor or Preview window selected in single-window mode. |
| Focus | The component receiving keyboard input. |

## Support

### Release Targets

| Release | Target |
|---|---|
| `1.0.0` | WezTerm on macOS and Ubuntu in OrbStack, using the macOS WezTerm frontend |

Check versions when preparing a release and report which OS/CPU, terminal version, and features worked, together with observed limitations. Broader terminal coverage can follow those results.

For `1.0.0-beta.1`, the `0.5.0` preview was checked in WezTerm `20240203-110809-5046fc22` on macOS 26.6.2 (ARM64) and Ubuntu 24.04.5 LTS (ARM64) in OrbStack using the macOS WezTerm frontend. Other terminal and platform combinations have not been verified as release targets.

### Graphics

At each interactive launch, use ratatui-image to detect terminal graphics. When Kitty graphics is selected, also query whether the terminal confirms support for the Unicode image placeholders RaTeQ uses. A missing or older response does not prove the feature is unsupported. For WezTerm, if Kitty placeholder support cannot be confirmed, use its iTerm2-compatible image protocol instead; this applies on any operating system.

If detection fails, selects halfblocks, or cannot confirm Kitty placeholder support and no compatible alternative is known, show [W401](messages.md#w401) before editing and retain the detected preview method, including ratatui-image's halfblocks fallback when selected. The warning includes a scrollable, copyable issue-report template with the RaTeQ version, available OS and terminal details, detected and selected graphics protocols, the check result, and the `TERM`, `TERM_PROGRAM`, `TERM_PROGRAM_VERSION`, and `COLORTERM` variables. Mark unavailable information accordingly. Enter continues; Up/Down scroll. Include no other environment variables in the report and do not submit it automatically.

Keep source editing/saving and export available when Preview display is unavailable. A retained rendered result can exist without terminal display. See [Export](interactions.md#export).

### Feature Availability

| Situation | Behavior |
|---|---|
| A message can be shown | Show [W402](messages.md#w402), identifying the unavailable feature and apparently unsupported capability, and inviting contributions |
| A message cannot be shown | Hide the feature from the Navigation bar and explain its unavailability in Ctrl+G Help |

CLI diagnostics appear in the terminal. In-app capability messages appear in the RaTeQ message bar.

### Selection

Mouse selection and native terminal copy/paste work in WezTerm. Record limitations encountered in other terminals, including GNOME Terminal, with the release compatibility results.

### Keyboard

Use Ctrl+N to insert a newline. In Find, Enter moves forward and wraps, so repeated presses can reach an earlier match. See [Input](editing.md#input) and [Find](editing.md#find).

Apply feature-availability rules to public shortcuts. Hidden shortcuts stay omitted from the Navigation bar and Help.

## Scope

### Later Versions

- Undo/Redo, with Ctrl+Z (`^Z`) reserved for Undo and Ctrl+Y (`^Y`) reserved for Redo. Undo grouping remains to be defined.

- Syntax highlighting
- Markdown `$$` block handling
- Shell redirection of an interactive session
- Configurable themes, key remapping, plugins, or a built-in package manager
- In-app adjustable settings
- Multiple UI languages
- Highlighting Find matches in the corresponding Preview window area

### Not Planned

- Full `.tex` document compilation
