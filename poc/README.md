# RaTeQ PoC

| Command | What it does/did |
|---|---|
| `poc/scripts/local.sh` | Runs rendering/export cases locally, including valid and invalid LaTeX. Writes timings, logs, and PNG/SVG samples. |
| `poc/scripts/linux.sh [MACHINE_NAME]` | Runs the same cases through OrbStack. Defaults to `rateq-ubuntu-arm64`. |
| `poc/scripts/equation.sh -i matrix -i large -i fraction --export` | Exports the selected named equations at full size. |
| `poc/scripts/equation.sh -i matrix -i large -i fraction --preview scroll` | Previews named equations with scrolling at half the export scale. |
| `poc/scripts/equation.sh -i matrix -i large -i fraction --preview break` | Compares preview-only line breaks before top-level `+` operators. Oversized indivisible terms remain scrollable. |
| `poc/scripts/preview.sh PATH_TO_PNG [--halfblocks]` | Opens an exported PNG in the legacy preview viewer. Optionally forces halfblocks. |
| `poc/scripts/input.sh` | Probes key events, mouse capture, and bracketed paste. F2/F3 toggle capture/paste. F10 exits. |
| `poc/scripts/schedule.sh` | Compares canceling and finishing renderer subprocesses during a synthetic edit burst. |

Named equations are `fraction`, `matrix`, `multiline`, and `large`. In the equation viewer, use `n`/`p` to switch equations, arrows to scroll, and `q` to quit.

Run scripts in the terminal being checked. Logs and generated files are under `poc/logs/` and `poc/output/`.

## Notes

| Area | PoC detail |
|---|---|
| Cancellation | The scheduling experiment terminates renderer child processes. Its timings include process startup. An in-process worker would need its own cancellation mechanism. |
| Paste | The input probe toggles bracketed paste and displays received events. Product text-insertion behavior is defined in [Input](../docs/editing.md#input). |
| Appearance | Black math, 32 px base font, 8 px padding, transparent PNG at 2× raster scale, preview at 1× composited onto white, and SVG with embedded glyph outlines. |
