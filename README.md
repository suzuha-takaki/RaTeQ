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

## Current status

The `0.1.0` implementation provides an editor and RaTeX preview. **Source saving and PNG/SVG export are not implemented**; they are planned for `0.2.0`. If you exit after editing, you can cancel the exit or discard your changes, but you cannot save them in RaTeQ yet. Resume, persistence setup, contextual Help, and reflow of an outdated preview are also unfinished. The `--resume` option cannot reopen a file in this build. The current build creates no application-data directories.

The automated checks pass, but the interactive editor and graphics still need validation in the primary development terminal. Run the tests with:

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
