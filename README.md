# RaTeQ

RaTeQ is a terminal editor for LaTeX math fragments on macOS and Linux, with a preview rendered by [RaTeX](https://github.com/erweixin/RaTeX).

## Build and run

Install Rust and Cargo, then run:

```sh
cargo build --release
./target/release/rateq
```

To open an existing UTF-8 source file, pass its path:

```sh
./target/release/rateq equation.txt
```

If the file does not exist, RaTeQ starts with an empty buffer. Type a math fragment such as `\frac{1}{2}` and press **Enter** to render it. Run `./target/release/rateq --help` for the command-line options.

Press **Ctrl+G** for contextual Help and **Ctrl+X** to return. RaTeQ requires a terminal of at least 80 columns by 24 rows; a resize screen appears below that size.

Use `./target/release/rateq --resume` to reopen the last source saved through RaTeQ. The first Resume request offers opt-in persistence setup; ordinary launches do not create application-data directories. You can choose a data location, continue without persistence, or cancel. If there is no saved-file history yet, Resume reports `E105`; save a source afterward to establish the history.

## Save and export

| Key | Where | Action |
|---|---|---|
| Ctrl+S | Edit | Open source save |
| Ctrl+E | Edit | Choose an export format |
| P / S | Export | Choose PNG / SVG |
| Tab | File entry | Complete the path; press again to list matches |
| Up / Down | File entry | Scroll completion matches |
| Ctrl+U | File entry | Clear the path |
| Enter | File entry | Accept displayed warnings and write the file |
| Ctrl+X | Export or file entry | Cancel |

In single-window mode, save is available in the Editor and export in the Preview. When exiting with unsaved changes, you can save, discard, or return to the editor. Validation errors must be corrected before writing.

Export requires a successful render. If the source has since changed or the latest render failed, RaTeQ warns that the export will use the older retained result.

For unnamed sources, RaTeQ suggests a filename made from the word lists in [assets/names.toml](assets/names.toml). One name is reused for source and export suggestions. The lists are embedded in the binary, so rebuild after editing them; if either list is empty, the suggested stem is `equation`.

## Development

Run the tests with:

```sh
cargo test
```

See the [product specification](docs/README.md), [roadmap](docs/ROADMAP.md), and [PoC experiments](poc/README.md) for more detail.

## Acknowledgements

I'd like to thank the maintainers and contributors of:

- [RaTeX](https://github.com/erweixin/RaTeX) for math rendering.
- [Ratatui](https://github.com/ratatui/ratatui) for the terminal interface.
- [ratatui-image](https://github.com/ratatui/ratatui-image) for displaying the preview.

Thanks also to the authors and contributors of the other open-source libraries RaTeQ depends on.

## License

RaTeQ is licensed under the [MIT License](LICENSE). Third-party libraries and assets, including bundled fonts, are subject to their own licenses.
