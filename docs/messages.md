# Messages

## Numbering

- IDs use `{severity}{category}{number}`. `W` identifies warnings and `E` identifies errors. The first digit identifies the category and the final two digits identify the diagnostic within that severity/category.
- Display the ID alongside its message. Keep assigned IDs stable when wording or document order changes. Add new diagnostics using the next unused number in their severity/category, starting at `01`.
- Behavior and display location are linked below. File entry validation errors block submission. Warnings allow submission after acknowledgment.
- Informational messages, completion candidates, and Preview statuses use their existing display rules.
- LaTeX diagnostics use the renderer's wording in the LaTeX message bar.

| Range | Category | Examples |
|---|---|---|
| `1xx` | CLI | Flag combinations and startup arguments |
| `2xx` | Files | Input formats, paths, naming, saving, exporting |
| `3xx` | Rendering | RaTeQ rendering-pipeline failures. Codes will be assigned when behavior is defined. |
| `4xx` | Terminal | Capabilities, graphics, and input support |
| `5xx` | Session | Interactive session decisions, including exit |

## Warnings

| ID | Message | Behavior |
|---|---|---|
| <a id="w201"></a>W201 | `WARNING: This file already exists and will be overwritten` | [Destination exists](interactions.md#messages) |
| <a id="w202"></a>W202 | `WARNING: This path includes a space` | [Path contains spaces](interactions.md#messages) |
| <a id="w203"></a>W203 | `WARNING: The output name differs from the file opened at launch` | [Launch-file comparison and acknowledgment](interactions.md#naming-warnings) |
| <a id="w204"></a>W204 | `WARNING: The filename does not match the source file` | [Current-source comparison and acknowledgment](interactions.md#naming-warnings) |
| <a id="w205"></a>W205 | `WARNING: Exporting the displayed preview, which differs from the current source or predates a failed render` | [Older-preview export](interactions.md#export) |
| <a id="w401"></a>W401 | `In-terminal preview unavailable` followed by `Your terminal does not appear to support a preview method used by rateq.` and `Contributions are welcome!` | [Missing terminal graphics](README.md#graphics) |
| <a id="w402"></a>W402 | Identify the unavailable feature, explain the apparently unsupported capability, and state `Contributions are welcome!` | [Other unavailable terminal features](README.md#feature-availability) |
| <a id="w403"></a>W403 | `WARNING: Terminal too small. Please resize.` | [Small-terminal layout](session.md#layout) |
| <a id="w501"></a>W501 | `WARNING: Answering "No" will destroy changes made` | [Exit with unsaved changes](interactions.md#exit) |

## Errors

| ID | Message | Behavior |
|---|---|---|
| <a id="e101"></a>E101 | `error: --live (-l) cannot be combined with --single (-s)` | [Incompatible CLI modes](session.md#cli) |
| <a id="e102"></a>E102 | `error: --resume (-r) cannot be combined with a filename` | [Resume with filename](session.md#cli) |
| <a id="e103"></a>E103 | `error: unknown option {option}` | [Unknown CLI flag](session.md#cli-validation) |
| <a id="e104"></a>E104 | `error: only one filename may be provided` | [Multiple CLI filenames](session.md#cli-validation) |
| <a id="e105"></a>E105 | `error: no saved source file to resume` | [Missing resume history](session.md#resume) |
| <a id="e106"></a>E106 | `error: help and version must each be run on their own`<br>`Did you mean {standalone-command}?` | [Standalone help/version and hint selection](session.md#cli-validation) |
| <a id="e201"></a>E201 | Explain that RaTeQ edits text and binary input is rejected | [Binary input](session.md#file-formats) |
| <a id="e202"></a>E202 | `ERROR: The file extension does not match the selected export format` | [Export extension validation](interactions.md#messages) |
| <a id="e203"></a>E203 | `ERROR: Enter a file path` | [Empty path or directory without filename](interactions.md#messages) |
| <a id="e204"></a>E204 | `ERROR: The destination directory does not exist` | [Missing parent directory](interactions.md#messages) |
| <a id="e205"></a>E205 | `ERROR: Cannot write file` followed by the reason | [Write failure and retry](interactions.md#file-entry) |
| <a id="e206"></a>E206 | `error: cannot reopen {path}` followed by the reason | [Missing or unreadable resume file](session.md#resume) |
| <a id="e301"></a>E301 | `ERROR: Render a preview before exporting` | [Export precondition](interactions.md#export) |
