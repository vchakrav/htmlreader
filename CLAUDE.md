# Claude Session Notes

## Project Overview

HTMLReader is a Rust library and CLI tool for extracting data from annotated HTML templates and rendering data back into them. It enables a "template + data = page" workflow.

## Architecture

### Core Functions (src/lib.rs)

- `extract_data(html: &str) -> Value` - Parses HTML and extracts all annotated values into JSON
- `render_data(html: &str, data: &Value) -> String` - Populates an HTML template with JSON data

### Key Implementation Details

1. **DOM Parsing**: Uses `dom_query` crate (not `scraper`) because we need DOM manipulation for rendering, not just parsing
2. **List Handling**: First child of a list element is used as the template; cloned for each data item
3. **Nested Lists**: Recursively handled - lists can contain objects with nested lists
4. **Web Components**: Elements with `data-rmx` attribute extract all attributes + children

### CLI (src/main.rs)

Three modes based on argument count:
- 1 arg: Extract JSON from template
- 2 args: Render and print HTML
- 3 args: Render and write to file

Also includes `organize_into_sections()` for grouping extracted data by prefix.

## Test Coverage

40 tests in `src/lib.rs` covering:
- Text, image, and attribute extraction
- Empty and nested lists
- Web component attributes and children
- Round-trip (extract -> render -> extract) integrity
- Integration tests with real templates

## Working Image Carousel

The template includes a working `<image-carousel>` web component:
- CSS animation for smooth scrolling
- JavaScript custom element that duplicates images for infinite loop
- Falls back to text (alt attribute) if images fail to load
- Pauses on hover

## Files Structure

```
├── src/
│   ├── lib.rs          # Core extract/render logic + tests
│   └── main.rs         # CLI interface
├── html/
│   ├── remix_home.html           # Original template
│   ├── remix_home_annotated.html # Template with rmx-* annotations
│   ├── salesforce_data.json      # Sample data
│   ├── salesforce_home.html      # Generated output
│   ├── petavue_data.json         # Sample data
│   └── patterns_only.html        # Minimal template for testing
├── NOTES.md            # User-facing documentation
└── CLAUDE.md           # This file
```

## Key Decisions Made

1. **attr:<name> syntax** for arbitrary attribute extraction (vs separate rmx-attr attribute)
2. **data-rmx for web components** (data-* attributes don't interfere with component behavior)
3. **Fixed "children" key** for web component child elements
4. **dom_query over scraper** because scraper's html() reorders attributes, breaking string replacement

## Potential Future Work

- Strip rmx-* attributes from rendered output (optional flag)
- Support for conditional rendering
- Support for loops with index variable
- WASM build for browser usage
- Template validation/linting

## Common Commands

```bash
# Run tests
cargo test

# Extract data
cargo run -- html/remix_home_annotated.html

# Generate branded page
cargo run -- html/remix_home_annotated.html html/salesforce_data.json html/salesforce_home.html

# Open in browser
open html/salesforce_home.html
```
