# Phase 2: Word editing tools

## Goal

Turn WordMock's writing surface into a useful everyday editor by replacing the placeholder Insert, Layout, and Review ribbons and making the Home controls affect saved document content.

## Delivered in this phase

### Home and typography

- Add Roboto, Open Sans, Lora, and Roboto Mono from the official Google Fonts repository. Keep each family’s SIL Open Font License beside the font, embed the font data in the app, and install the license notices in the macOS bundle.
- Let users choose a font family and size, apply bold/italic/underline, choose text color, apply yellow highlight, and clear character formatting.
- Preserve formatting by UTF-8 byte ranges and inherit the active style for newly typed text.
- Add left, center, right, and justified document alignment; line spacing; and current-paragraph bullet/number prefixes.

### Insert

- Insert the current date and time at the selection or caret.
- Create a styled hyperlink on selected text or insert a linked label. Store its address in the WordMock document.
- Insert a visible page-break marker and a small editable, tab-separated text table.

### Layout and review

- Set Letter, A4, or Legal paper; portrait or landscape; and a shared page margin.
- Add find-next, replace-current, replace-all, word count, and character count.
- Keep zoom controls and make the page surface scroll when the chosen paper size exceeds the window.

### Saved documents

- Advance the `.wordmock` JSON format to version 2 for font, color, highlight, hyperlink, alignment, spacing, and page settings.
- Read version 1 documents with defaults for fields that did not exist yet. Continue reading and exporting `.txt` and `.md` as plain text.
- Keep undo/redo snapshots for format and page-setting changes, plus grouped typing and replace-all actions.

## Next implementation steps

The implementation order below keeps editing, pagination, and file interchange on one dependable content model. Each stage is useful on its own and prepares the data needed by the next stage.

### Phase 3 — structured document content

- Replace the flat string and format spans with a document tree: document → paragraphs → inline text, links, and images; paragraph blocks can also contain tables and explicit page breaks.
- Keep selections and undo operations anchored to stable document positions so inserting an image or table does not invalidate later formatting ranges.
- Replace text prefixes with real list properties, including bullet styles, ordered numbering, indentation, continuation, and toggle-off behavior across selected paragraphs.
- Build editable tables with row/column insertion and deletion, cell navigation, tab-to-next-cell behavior, and copy/paste of tabular text.
- Add image insertion from local files, inline sizing, aspect-ratio preservation, alignment, alt text, and delete/undo.
- **Done when:** a document can mix formatted paragraphs, lists, images, links, tables, and page breaks; the content survives save/reopen and undo/redo.

### Phase 4 — page layout and printing

- Measure paragraph and object heights against the chosen paper, margins, font metrics, and line spacing; flow content across real pages.
- Implement page breaks, page-specific headers and footers, and page-number fields. Keep each page editable while the document scrolls continuously.
- Add common paper sizes, independent margins, gutter, paragraph indents, before/after spacing, and page orientation.
- Produce a print preview and PDF export from the same layout engine so the page shown on screen matches the output.
- **Done when:** long documents paginate consistently, page-number fields update after edits, and print/PDF output matches the on-screen page layout.

### Phase 5 — Word file interoperability

- Import `.docx` text, headings, common character and paragraph styles, lists, tables, images, hyperlinks, and page setup into the structured model.
- Export the supported model back to `.docx`; preserve unknown package parts where possible and report unsupported features clearly.
- Retain `.wordmock` as the lossless native format and continue plain-text/Markdown export.
- Add document recovery for interrupted writes and a clear conversion summary when an import cannot preserve a feature.
- **Done when:** a representative round trip preserves supported text, formatting, layout, and embedded objects without silently discarding content.

### Phase 6 — review, navigation, and everyday polish

- Add spelling suggestions, language selection, comments, and tracked insertions/deletions on top of stable document positions.
- Make search case/word aware, highlight all matches, allow navigation among matches, and keep replace operations as one undo step.
- Make links open on activation and add editing affordances to images and tables without interfering with text selection.
- Add heading styles, a styles pane, an outline/navigation pane, full list controls, keyboard navigation, and accessible names/states for ribbon controls.
- Add autosave/recovery, a recent-files list, and shortcuts for the most common editing and navigation actions.
- **Done when:** review annotations remain attached to their content through edits, search results are visible, and the main workflows can be completed from the keyboard.

## Current limits

- The editor still stores a single text string with character-format spans. Alignment and line spacing apply to the whole document, and bullets/numbering prefix the current paragraph.
- The page-break tool is a visible text marker, not a pagination directive. The table tool is tab-separated editable text, not a cell grid.
- Images, headers/footers, real page numbers, DOCX, PDF, spelling, comments, and tracked changes are not implemented yet.
