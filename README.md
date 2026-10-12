# WordMock

A personal desktop word processor built in Rust. The project starts with the document editing loop and will grow into a richer, paginated editor over time.

## Current milestone

- Native desktop window with a page-like writing surface and ribbon tabs
- Font family, size, bold, italic, underline, text color, highlight, and clear-formatting controls
- Bundled Roboto, Open Sans, Lora, and Roboto Mono fonts from Google Fonts
- Paragraph alignment, line spacing, bullets, and numbering
- Letter, A4, and Legal page sizes, orientation, and adjustable margins
- Insert date/time, hyperlinks, page-break markers, and editable tab-separated text tables
- Find/replace, word count, and character count
- Undo and redo, including grouped typing and formatting changes
- Adjustable display size and zoom
- Open `.wordmock`, `.txt`, and `.md` files; save formatted `.wordmock` documents or export plain text
- `Cmd/Ctrl+N`, `Cmd/Ctrl+O`, `Cmd/Ctrl+S`, `Cmd/Ctrl+F`, `Cmd/Ctrl+Z`, `Cmd/Ctrl+Shift+Z`, and `Cmd/Ctrl+B/I/U` shortcuts
- Save, discard, or cancel before replacing an unsaved document

WordMock documents store text, character-format ranges, and page settings in a versioned JSON format. Saving as `.txt` or `.md` exports only the text.

The bundled fonts are distributed under the SIL Open Font License. Their license texts ship in `assets/fonts` and are copied into the installed macOS app bundle.
Sources: [Roboto](https://github.com/google/fonts/tree/main/ofl/roboto), [Open Sans](https://github.com/google/fonts/tree/main/ofl/opensans), [Lora](https://github.com/google/fonts/tree/main/ofl/lora), and [Roboto Mono](https://github.com/google/fonts/tree/main/ofl/robotomono).

## Run

Install a recent Rust toolchain, then run:

```sh
cargo run
```

On macOS, install a standalone app in `~/Applications/WordMock.app` with:

```sh
./scripts/install_macos.sh
open ~/Applications/WordMock.app
```

## Roadmap

See [the detailed phase plan](docs/PHASE_2_PLAN.md) for delivered tools and the next implementation steps.

This release's page-break tool inserts a visible text marker, and its table tool inserts editable tab-separated text. True page breaks and grid tables require the structured document model described in the plan. Images, headers/footers, print export, `.docx`, spelling, and tracked changes remain future work.
