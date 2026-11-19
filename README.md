# RaTeQ

RaTeQ is a terminal editor for LaTeX math fragments on macOS/Linux, with a live & manualpreview.
> **Status:** Active development.
> **[v1.0.0-beta.1](https://github.com/suzuha-takaki/RaTeQ/tree/v1.0.0-beta.1)** is now available. See [Releases](https://github.com/suzuha-takaki/RaTeQ/releases) for more detail.

## Quick Start

Download the prebuilt archive for your platform from the [Releases](https://github.com/suzuha-takaki/RaTeQ/releases):

- **macOS (Apple Silicon):** `rateq-1.0.0-beta.1-macos-arm64.tar.gz` *(macOS 11.0+)*
- **Linux (ARM64):** `rateq-1.0.0-beta.1-linux-arm64.tar.gz` *(requires glibc 2.39+)*

Extract the archive and run the binary:

```bash
./rateq
```

## Environment

### Preview may not display correctly

RaTeQ checks terminal graphics at launch. 
If it cannot confirm support and has no known compatible alternative, it shows a warning with a copyable issue-report template. 
Press **Enter** to continue anyway.

### Persistence setup (opt-in) for resume edit

Use `./target/release/rateq --resume` to reopen the last source saved through RaTeQ. 
The first Resume request offers opt-in persistence setup; ordinary launches do not create application-data directories. 
You can choose a data location, continue without persistence, or cancel. 
If there is no saved-file history yet, Resume reports `E105`; save a source afterward to establish the history.

## Acknowledgements

I'd like to thank the maintainers and contributors of:

- [RaTeX](https://github.com/erweixin/RaTeX) for math rendering.
- [Ratatui](https://github.com/ratatui/ratatui) for the terminal interface.
- [ratatui-image](https://github.com/ratatui/ratatui-image) for displaying the preview.

Thanks also to the authors and contributors of the other open-source libraries RaTeQ depends on.

## License

RaTeQ is licensed under the [MIT License](LICENSE). Third-party libraries and assets, including bundled fonts, are subject to their own licenses.

## Feedback & Contributing

Bug reports, ideas, and feedback are welcome via GitHub issues and releases.
