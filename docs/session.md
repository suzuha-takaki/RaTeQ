# Session

## CLI

| Command | Action |
|---|---|
| `rateq` | Open an empty unnamed source, manual rendering, double window |
| `rateq -r` | Reopen the last source file saved through RaTeQ |
| `rateq equation.txt` | Open a named math fragment. If missing, start an empty named source. Create/write the file on successful save. |
| `rateq -l` | Use live rendering |
| `rateq -s` | Use single-window display |
| `rateq -b` | Break long equations into preview lines |
| `rateq -h` | Print CLI usage |
| `rateq -v` | Print version |

| Short | Long |
|---|---|
| `-r` | `--resume` |
| `-l` | `--live` |
| `-s` | `--single` |
| `-b` | `--break` |
| `-h` | `--help` |
| `-v` | `--version` |

Session flags can be combined in any order. Help and version each run as a standalone option. CLI warnings and errors appear in the terminal.

| Combination | Error |
|---|---|
| `-ls`, `-l -s` | [E101](messages.md#e101) |
| `-r equation.txt` | [E102](messages.md#e102) |

### CLI Validation

- Reject unknown flags and multiple filenames before opening the UI.
- Help accepts only `rateq -h` or `rateq --help`. Version accepts only `rateq -v` or `rateq --version`.
- Any extra flag, filename, or repeated help/version option produces [E106](messages.md#e106). This check takes priority when help/version is present.
- Otherwise, validate conflicting modes before opening the UI.

| Case | Error |
|---|---|
| Unknown flag | [E103](messages.md#e103) |
| Multiple filenames | [E104](messages.md#e104) |
| Help/version combined with other arguments or repeated | [E106](messages.md#e106) |

The second line of E106 suggests the standalone command.

| Requested option | Hint |
|---|---|
| Help | `Did you mean 'rateq -h'?` |
| Version | `Did you mean 'rateq -v'?` |
| Both | `Did you mean 'rateq -h' or 'rateq -v'?` |

### CLI Help

```text
Usage: rateq [OPTIONS] [FILE]

Open and edit a LaTeX math fragment.

Options:
  -r, --resume   Reopen the last source file saved through RaTeQ
  -l, --live     Render after 300 ms without source edits
  -s, --single   Show one window at a time
  -b, --break    Break long equations into preview lines
  -h, --help     Show CLI help
  -v, --version  Show version

With no options, start an unnamed source in manual, double-window mode.
Long equations scroll by default; --break enables preview line breaks.
Enter renders immediately in manual and live modes.
Live and single-window modes cannot be combined.
Resume cannot be combined with FILE.
Run help on its own with rateq -h, or version on its own with rateq -v.
```

## Resume

- Record the path of the last successfully saved source file.
- `--resume` reopens that file.
- With no saved-file history, report [E105](messages.md#e105).
- If the recorded file is missing or unreadable, report [E206](messages.md#e206) and preserve the recorded path for diagnosis.
- Store resume metadata in the approved application-data location. See [Persistence](#persistence).

## Persistence

- Use platform-appropriate application directories by default.
- On the first interactive launch, ask for approval before creating application directories or writing persistent application data.
- Show the proposed locations and offer Use defaults, Customize, or Cancel.
- Customize lets the user choose the application-data location.
- Remember the approved location and reuse it on future launches.
- Cancel exits before creating application directories or writing application data.
- Standalone CLI help/version run directly without opening persistence setup.
- Source saves and exports follow the paths submitted through File entry.

## File Identity

| State | Meaning |
|---|---|
| Unnamed | Source has no file path yet |
| Named | Source has a file path |

The launch directory is the working directory from which RaTeQ was called. Relative save/export paths resolve from this directory.

## File Formats

| Extension | Contents | On opening |
|---|---|---|
| `.txt` | Raw LaTeX math fragment | Edit raw fragment |
| `.tex` | Raw LaTeX math fragment | Edit raw fragment |
| Unknown/none | Raw LaTeX math fragment in a valid text file | Treat as `.txt` |
| Binary, including `.png` | Binary data | Reject with [E201](messages.md#e201) |

### Text Encodings

- UTF-8 without a BOM
- UTF-8 with a BOM
- US-ASCII (also valid UTF-8)

## Rendering Mode

| Mode | Trigger | Initial Preview window |
|---|---|---|
| Manual | Enter | `PRESS ENTER TO SEE PREVIEW` |
| Live | 300 ms after the last source edit, or immediately on Enter. Restart the delay after each edit. | Empty |

Render behavior and LaTeX diagnostics are defined under [Render](editing.md#render).

## Display Mode

| Mode | Windows | Enter during Edit |
|---|---|---|
| Double window | Editor and Preview | Render immediately in manual and live modes |
| Single window | Editor or Preview | From Editor, render and show Preview. From Preview, return to Editor. |

Single-window mode starts in the Editor window.

| Rendering | Display | Supported |
|---|---|---|
| Manual | Double window | Yes |
| Live | Double window | Yes |
| Manual | Single window | Yes |
| Live | Single window | Rejected by CLI |

### Layout

- In double-window mode, place Editor above Preview. Initially give the windows equal heights after reserving space for the bars.
- Keep this vertical arrangement in narrow terminals.
- In single-window mode, the active Editor or Preview window uses the available window area.
- Keep Entry bar, Navigation bar, and message access visible when the terminal is small.
- Show [W403](messages.md#w403) when the Editor/Preview area cannot fit.
- The minimum dimensions and below-minimum behavior are open decisions; see [Design](DESIGN.md#product-decisions).

## Preview Flow

| CLI | Behavior |
|---|---|
| Default | Keep the equation's layout and scroll to view overflow |
| `-b`, `--break` | Add preview-only line breaks where valid to fit the window width. Reflow when its width changes. Oversized indivisible expressions remain scrollable. |

Preview flow applies in both display modes. Source saves and PNG/SVG exports use the original source layout.

## Terminal Support

Feature availability and messages are defined in [Support](README.md#support).
