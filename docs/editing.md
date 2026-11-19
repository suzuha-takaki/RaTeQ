# Editing

## Input

| Key | Action |
|---|---|
| ↑ ↓ ← → | Move cursor in the Editor window |
| Ctrl+N | Newline |
| Tab | Insert one space |

Mouse selection and native terminal copy/paste work in WezTerm. Record terminal-specific limitations with release compatibility results in [Support](README.md#selection).

- Insert pasted text into the source, including any newlines.
- After insertion, apply normal rendering behavior. Live mode waits for the typing-stop delay. Manual mode waits for Enter.

## Find

Ctrl+F opens the Find entry bar.

```text
Search:
```

Navigation bar

```text
^G Get Help   ^X Close    ENTER Next
```

| Key | Action |
|---|---|
| Enter | Next match |
| Ctrl+X | Close Find and return focus to Editor |

- Match literal, case-sensitive text in the source.
- Match navigation wraps at the beginning and end of the source.
- To reach an earlier match, press Enter repeatedly to move forward and wrap around.

| Case | RaTeQ message bar |
|---|---|
| No matches | `No matches` |

Find focuses on the Editor window. Ctrl+F from single-window Preview switches to Editor and opens Find. This fallback is documented internally and omitted from user-facing Help and the Preview Navigation bar.

## Render

- Use RaTeX to render the source. Retain the last successful rendered result and present its preview in the Preview window when display is available.
- Show LaTeX warnings and errors in the LaTeX message bar.
- Keep the retained result and its preview if rendering fails.
- Always show a status indicator in a corner of the Preview window. Update it as rendering and source contents change.

- On a source edit, cancel the in-flight render and discard its output and diagnostics.
- Live mode waits for the next 300 ms typing pause. Manual mode waits for Enter.
- Keep the retained rendered result while waiting for a replacement.
- Display the preview at half the export render scale in each dimension. Exports use the full render scale.

Render timing and window switching follow the [session settings](session.md#rendering-mode). Preview line breaking and scrolling follow [Preview Flow](session.md#preview-flow).

### Preview Status

| Status | Case |
|---|---|
| Rendering | A render is in progress, with or without a retained result |
| Failed | The current source's latest render failed, with or without a retained result |
| Current | The retained result matches the current source |
| Outdated | A retained result exists and differs from the current source, which has no current render failure |
| Awaiting render | No retained result exists and the current source has no current render failure |

- Apply the first matching row. The five statuses are mutually exclusive.
- A successful render with LaTeX warnings is Current. Show the warnings in the LaTeX message bar.
- After a source change, recalculate status using the current source and retained result.
- Failed and Rendering may retain an older result and its preview. Their status indicates that the result is awaiting a successful replacement.
- Display the status as its short text label in the Preview window corner.
