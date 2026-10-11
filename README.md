# WordMock

A personal desktop word processor built in Rust. The project starts with the document editing loop and will grow into a richer, paginated editor over time.

## Current milestone

- Native desktop window with a page-like writing surface and ribbon tabs
- Editable text with bold, italic, and underline character formatting
- Undo and redo, including grouped typing and formatting changes
- Adjustable display size and zoom
- Word count and an estimated page count
- Open `.wordmock`, `.txt`, and `.md` files; save formatted `.wordmock` documents or export plain text
- `Cmd/Ctrl+N`, `Cmd/Ctrl+O`, `Cmd/Ctrl+S`, `Cmd/Ctrl+Z`, `Cmd/Ctrl+Shift+Z`, and `Cmd/Ctrl+B/I/U` shortcuts
- Save, discard, or cancel before replacing an unsaved document

WordMock documents store text, character-format ranges, and font size in a versioned JSON format. Saving as `.txt` or `.md` exports only the text.

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

1. Build paragraph-aware editing with line and paragraph styles.
2. Implement real page layout, margins, headers, footers, and print/export.
3. Add `.docx` import/export and preserve common Word formatting.
4. Add tables, images, lists, find/replace, spelling tools, and accessibility.

The Insert, Layout, and Review ribbon tabs are still placeholders. Bold uses an installed system font when one is available; italic and underline are rendered by the editor.
