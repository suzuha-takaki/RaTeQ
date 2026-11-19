# Interactions

Entry bars stay open until the operation proceeds without error or is canceled. Each Entry bar has an accompanying Navigation bar. Application warnings and errors appear in the RaTeQ message bar.

## Edit

| Action | Key | Double window | Single Editor | Single Preview |
|---|---|---|---|---|
| Render/switch | Enter | Render immediately | Render and show Preview | Show Editor |
| Find | Ctrl+F | Find source | Find source | Switch to Editor and Find (hidden fallback) |
| Save | Ctrl+S | Save source | Save source | Unavailable |
| Export | Ctrl+E | Export retained result | Unavailable | Export retained result |
| Exit | Ctrl+X | Exit entry bar | Exit entry bar | Exit entry bar |
| Help | Ctrl+G | Help | Help | Help |

Editor actions are defined in [Editing](editing.md).

### Navigation Bar

| Session/view | Shortcuts |
|---|---|
| Manual, double window | `^G Get Help   ENTER Render   ^F Find   ^S Save   ^E Export   ^X Exit` |
| Live, double window | `^G Get Help   ENTER Render   ^F Find   ^S Save   ^E Export   ^X Exit` |
| Single, Editor | `^G Get Help   ENTER Preview   ^F Find   ^S Save   ^X Exit` |
| Single, Preview | `^G Get Help   ENTER Editor   ^E Export   ^X Exit` |

## File Entry

| Opened from | File type | On normal success |
|---|---|---|
| Ctrl+S | Source file | Return to Edit in the Editor window |
| Export entry bar | PNG/SVG file | Return to the previous Edit view, Preview in single-window mode |
| Exit entry bar | Source file | Close RaTeQ after saving successfully in either display mode. |

### Entry Bar

```text
File Type :
File to write :
```

### Navigation Bar

```text
^G Get Help       TAB   Complete path
^X Cancel         ENTER Complete
```

| Key | Action |
|---|---|
| Enter | Acknowledge displayed warnings and submit the full path |
| Ctrl+X | Cancel a normal save/export and return to the previous Edit view |
| Tab | Complete the path |

Validation errors hide `ENTER Complete` and block writing until corrected. During save-before-exit, Ctrl+X returns to the Exit entry bar. A failed write keeps the File entry bar open with the error and allows retry.

### Filename Defaults

The default directory for new output paths is the launch directory. Saving a named source defaults to its current source path.

| Source identity | Source filename | PNG filename | SVG filename |
|---|---|---|---|
| Unnamed | `{generated-stem}.txt` | `{generated-stem}.png` | `{generated-stem}.svg` |
| Named | Existing source basename and extension | Source stem with `.png` | Source stem with `.svg` |

- Generate one stem per unnamed source.
- Store the word lists in a `toml` file using the `first` and `second` array fields, embedded in the distributed binary.
- Choose one word independently from each list and join them as `{first}-{second}`. If either list is empty, use `equation` as the suggested stem.
- Reuse the generated stem across source and export suggestions. Existing-destination warnings apply to generated suggestions too.
- Users can edit the suggested path. Corresponding source and export files should share a directory and stem. A difference displays a warning and allows saving.
- After a successful source save, use the saved path for later source saves and export-name suggestions.
- Naming warnings follow the launch-file baseline and acknowledgment rules below.

### Path Completion

- Accept relative paths from the launch directory and absolute paths.
- Paths ending in `/` represent directories.
- Tab completes a unique matching filename or directory name.
- With several matches, Tab extends the input to their longest common prefix.
- Press Tab again without changing the input to display matching names below the field.
- Append `/` when completing a directory. Insert names containing spaces literally.
- Show completion candidates alongside the filename field. Completion output appears outside the RaTeQ message bar.
- With no matches, leave the input unchanged and show an empty candidate list. A new filename can still be saved when its destination is valid.
- Compare source/export directories after resolving relative paths, `.` and `..`, and directory aliases to their actual locations.
- Use warnings for risky paths that can be saved and errors for paths that cannot be saved.

### Messages

| Case | RaTeQ message bar |
|---|---|
| Destination exists | [W201](messages.md#w201) |
| Path includes a space | [W202](messages.md#w202) |
| Source path differs from launch file, or export directory/stem differs from launch file | [W203](messages.md#w203) |
| Export directory and/or filename stem differs from named source | [W204](messages.md#w204) |
| PNG path does not end in `.png`, or SVG path does not end in `.svg` | [E202](messages.md#e202) |
| Empty path or a directory without a filename | [E203](messages.md#e203) |
| Parent directory missing | [E204](messages.md#e204) |
| Cannot write destination | [E205](messages.md#e205) |

- Naming warnings use the acknowledgment rules below. Other warnings, including overwriting an existing file, continue to apply on every relevant save.
- Source extensions follow [File Formats](session.md#file-formats).

### Naming Warnings

- Keep the file opened at launch as the baseline, including a file opened through resume. A new unnamed session has no launch-file baseline.
- For source output, compare the full destination path with the launch file. For PNG/SVG, compare directory and stem with the launch file and with the current named source.
- Show the applicable naming warning in the RaTeQ message bar while the File entry bar is open, including when suggesting a path. If both comparisons differ, show W203.
- Remember the last successfully saved, acknowledged destination separately for source, PNG, and SVG. Repeating that destination suppresses its naming warning.
- A successful source save to a different current source path starts a new acknowledgment period for all three file types. A warning acknowledged for that source save counts in the new period.
- Canceled and failed saves leave acknowledgments unchanged. Tracking lasts for the current session.
- An unnamed source has no source filename to compare. After its first successful source save, use the resulting named source for export comparisons.

| Starting file | Operation | Naming warning |
|---|---|---|
| `equation.txt` | Save source as `equation2.txt` | Yes |
| Same session | Export `equation2.png` | Yes, first PNG acknowledgment |
| Same session | Export `equation2.svg` | Yes, separate SVG acknowledgment |
| Same session | Export `equation2.png` again | Suppressed |
| Same session | Save source as `equation3.txt` | Yes, start new acknowledgment period |
| Same session | Export `equation3.png` | Yes |

## Export

Use RaTeX for PNG and SVG output corresponding to the retained rendered result. Save writes the current source in the Editor window.

| Rendered result condition | Export behavior |
|---|---|
| No retained result | Disable submission and explain with [E301](messages.md#e301) in the RaTeQ message bar |
| Retained result differs from the current source, or the latest render failed | Allow export of the retained result with [W205](messages.md#w205) |
| Retained result matches the current source and the latest render succeeded | Export the retained result |

The same precondition applies when terminal display is unavailable. A retained result can supply export output even when the terminal cannot display its preview.

### Entry Bar

```text
Export preview as ?
P png   S svg
```

### Navigation Bar

```text
^G Get Help   ^X Cancel
```

| Key | Action |
|---|---|
| P | Open File entry with PNG file type |
| S | Open File entry with SVG file type |
| Ctrl+X | Cancel and return to the previous Edit view |

## Exit

### Entry Bar

```text
Save modified buffer ?
Y Yes   N No
```

### Navigation Bar

```text
^G Get Help   ^X Cancel
```

| Key | Action |
|---|---|
| Y | Open File entry with source file type. Exit after a successful save in either display mode. |
| N | Exit |
| Ctrl+X | Cancel exit |

| Case | RaTeQ message bar |
|---|---|
| Exit entry bar open | [W501](messages.md#w501) |

If the source has no unsaved changes, exit immediately. Otherwise, open the Exit entry bar.

## Help

Ctrl+G opens Help from every interaction, including Find. List all public keyboard shortcuts for the calling session, active window, and interaction, including shortcuts omitted from the Navigation bar.

- Explain terminal-dependent features that are unavailable and hidden from the Navigation bar.
- Omit the hidden Ctrl+F fallback from single-window Preview and hidden previous-match navigation.
- Ctrl+X returns to the exact calling interaction, active window, focus, and in-progress input.
- Show Help full-screen as a scrollable contextual shortcut list.

### Navigation Bar

```text
^G Get Help   ↑↓ Scroll   PgUp/PgDn Page   ^X Return
```
