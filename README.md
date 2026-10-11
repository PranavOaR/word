# WordMock

A personal desktop word processor built in Rust. The project starts with the document editing loop and will grow into a richer, paginated editor over time.

## Current milestone

- Native desktop window with a page-like writing surface and ribbon tabs
- Plain-text editing with adjustable display size and zoom
- Word count and an estimated page count
- Open and save `.txt` and `.md` files
- `Cmd/Ctrl+N`, `Cmd/Ctrl+O`, and `Cmd/Ctrl+S` shortcuts
- Save, discard, or cancel before replacing an unsaved document

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

1. Build a structured document model with paragraphs, selections, and undo/redo.
2. Add actual inline formatting and paragraph styles.
3. Implement page layout, margins, headers, footers, and print/export.
4. Add `.docx` import/export and preserve common Word formatting.
5. Add tables, images, lists, find/replace, spelling tools, and accessibility.

The current editor stores plain text. The ribbon's Insert, Layout, and Review tabs are placeholders, and the B/I/U buttons are disabled until the document model supports rich text.
