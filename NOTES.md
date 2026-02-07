# HTMLReader - Design Notes

## Goal

Parse HTML snippets and extract/render data using annotation attributes.

## Annotation Attributes

### Standard Elements

Use `rmx-name` and `rmx-type` attributes:

```html
<span rmx-name="title" rmx-type="text">Hello World</span>
<img rmx-name="logo" rmx-type="image" src="logo.png">
<a rmx-name="link" rmx-type="attr:href" href="https://example.com">Click</a>
```

### Extraction Types

| Type | Extracts | Example |
|------|----------|---------|
| `text` | innerText of the node | `"Hello World"` |
| `image` | src attribute (shorthand for attr:src) | `"logo.png"` |
| `attr:<name>` | any attribute value | `"https://example.com"` |
| `list` | array of objects from children | `[{...}, {...}]` |

### Nested Lists

Lists can contain nested objects and lists at arbitrary depth:

```html
<div rmx-name="categories" rmx-type="list">
    <div>
        <span rmx-name="name" rmx-type="text">Electronics</span>
        <ul rmx-name="products" rmx-type="list">
            <li rmx-name="title" rmx-type="text">Phone</li>
        </ul>
    </div>
</div>
```

Extracts:
```json
{
  "categories": [
    {
      "name": "Electronics",
      "products": [{ "title": "Phone" }]
    }
  ]
}
```

### Web Components

For custom elements (tags with hyphens), use `data-rmx` to bind all attributes:

```html
<image-carousel data-rmx="carousel" speed="3000" autoplay="true">
    <img src="a.png" alt="First">
    <img src="b.png" alt="Second">
</image-carousel>
```

Extracts:
```json
{
  "carousel": {
    "speed": "3000",
    "autoplay": "true",
    "children": [
      { "src": "a.png", "alt": "First" },
      { "src": "b.png", "alt": "Second" }
    ]
  }
}
```

## Rendering

The `render_data` function takes a template HTML and JSON data, and populates the template:

- Text values replace innerText
- Attribute values update the specified attribute
- Lists clone the first child as a template and replicate for each item
- Web components update all attributes and replicate children

## CLI Usage

```bash
# Extract data from HTML (outputs JSON)
cargo run -- template.html

# Render template with data (outputs HTML to stdout)
cargo run -- template.html data.json

# Render template with data (writes to file)
cargo run -- template.html data.json output.html
```

## Example Files

- `html/remix_home_annotated.html` - Base template with annotations
- `html/salesforce_data.json` - Salesforce-branded data
- `html/petavue_data.json` - Petavue-branded data
- `html/salesforce_home.html` - Generated Salesforce page
