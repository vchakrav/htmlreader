use dom_query::{Document, NodeRef, Selection};
use serde_json::{json, Value};

/// Extracts data from HTML snippets based on rmx-name and rmx-type attributes.
///
/// # Attributes
/// - `rmx-name`: The key name for the extracted value
/// - `rmx-type`: The type of extraction
/// - `data-rmx`: For web components (custom elements), extracts all attributes as an object
///
/// # Extraction Types
/// - `text`: Extracts the inner text content of the element (default)
/// - `image`: Extracts the `src` attribute (shorthand for `attr:src`)
/// - `attr:<name>`: Extracts the specified attribute (e.g., `attr:href`, `attr:data-id`)
/// - `list`: Recursively extracts data from child elements with rmx-* attributes
///
/// # Web Components
/// For custom elements (tags containing a hyphen), use `data-rmx` to bind all attributes:
/// ```html
/// <my-carousel data-rmx="carousel" speed="500">
///     <img src="a.png" alt="First">
/// </my-carousel>
/// ```
/// Extracts: `{"carousel": {"speed": "500", "children": [{"src": "a.png", "alt": "First"}]}}`
pub fn extract_data(html: &str) -> Value {
    let document = Document::from(html);
    let mut result = json!({});

    // Extract from rmx-name elements
    let rmx_selection = document.select("[rmx-name]");
    let rmx_result = extract_from_selection(&rmx_selection, &document);
    if let Value::Object(map) = rmx_result {
        for (k, v) in map {
            result[k] = v;
        }
    }

    // Extract from data-rmx web components
    let web_components = document.select("[data-rmx]");
    for node in web_components.nodes().iter() {
        let sel = Selection::from(node.clone());
        if let Some((key, value)) = extract_web_component(&sel) {
            result[key] = value;
        }
    }

    result
}

/// Extracts data from a selection, only processing top-level rmx elements.
fn extract_from_selection(selection: &Selection, doc: &Document) -> Value {
    let mut result = json!({});

    for node in selection.nodes().iter() {
        let sel = Selection::from(node.clone());

        // Skip if this element is inside a list
        if is_inside_list(node, doc) {
            continue;
        }

        if let Some((key, value)) = extract_element(&sel) {
            result[key] = value;
        }
    }

    result
}

/// Checks if an element is inside a list element.
fn is_inside_list(node: &NodeRef, doc: &Document) -> bool {
    // Check if any ancestor has rmx-type="list"
    let lists = doc.select("[rmx-type=\"list\"]");
    for list_node in lists.nodes().iter() {
        // Skip if it's the same node
        if node.id == list_node.id {
            continue;
        }
        // Check if node is a descendant of this list
        let list_sel = Selection::from(list_node.clone());
        if list_sel
            .select("[rmx-name]")
            .nodes()
            .iter()
            .any(|n| n.id == node.id)
        {
            return true;
        }
    }
    false
}

/// Extracts data from a single element based on its rmx-type attribute.
fn extract_element(sel: &Selection) -> Option<(String, Value)> {
    let name = sel.attr("rmx-name")?;
    let rmx_type = sel.attr("rmx-type").unwrap_or_default();
    let rmx_type = if rmx_type.is_empty() {
        "text"
    } else {
        &rmx_type
    };

    let value = extract_value(sel, rmx_type);

    Some((name.to_string(), value))
}

/// Extracts a value from an element based on the rmx-type.
fn extract_value(sel: &Selection, rmx_type: &str) -> Value {
    match rmx_type {
        "text" => extract_text(sel),
        "image" => extract_attr(sel, "src"),
        "list" => extract_list(sel),
        _ if rmx_type.starts_with("attr:") => {
            let attr_name = &rmx_type[5..];
            extract_attr(sel, attr_name)
        }
        _ => extract_text(sel),
    }
}

/// Extracts inner text from an element.
fn extract_text(sel: &Selection) -> Value {
    Value::String(sel.text().trim().to_string())
}

/// Extracts a specific attribute from an element.
fn extract_attr(sel: &Selection, attr_name: &str) -> Value {
    match sel.attr(attr_name) {
        Some(val) => Value::String(val.to_string()),
        None => Value::Null,
    }
}

/// Extracts data from a web component (custom element with data-rmx attribute).
/// Returns all attributes as an object, plus children as a list.
fn extract_web_component(sel: &Selection) -> Option<(String, Value)> {
    let name = sel.attr("data-rmx")?;
    let mut obj = json!({});

    // Extract all attributes except data-rmx using Selection's attrs()
    for attr in sel.attrs() {
        let attr_name = attr.name.local.to_string();
        if attr_name != "data-rmx" {
            obj[attr_name] = Value::String(attr.value.to_string());
        }
    }

    // Extract children - each child's attributes become an object in the list
    let children = sel.children();

    let mut child_list = Vec::new();
    for child_node in children.nodes().iter() {
        let child_sel = Selection::from(child_node.clone());
        let child_attrs = child_sel.attrs();

        if !child_attrs.is_empty() {
            let mut child_obj = json!({});
            for attr in child_attrs {
                let attr_name = attr.name.local.to_string();
                child_obj[attr_name] = Value::String(attr.value.to_string());
            }
            child_list.push(child_obj);
        }
    }

    if !child_list.is_empty() {
        obj["children"] = Value::Array(child_list);
    }

    Some((name.to_string(), obj))
}

/// Extracts a list of objects from child elements with rmx-* attributes.
fn extract_list(sel: &Selection) -> Value {
    let mut items = Vec::new();

    // Get direct children
    let children = sel.children();
    for node in children.nodes().iter() {
        let child_sel = Selection::from(node.clone());
        let item = extract_list_item(node, &child_sel);
        if !item.is_null() && item != json!({}) {
            items.push(item);
        }
    }

    Value::Array(items)
}

/// Extracts data from a list item element.
fn extract_list_item(node: &NodeRef, sel: &Selection) -> Value {
    let mut item = json!({});

    // Check if the element itself has rmx-name
    if let Some(name) = sel.attr("rmx-name") {
        let rmx_type = sel.attr("rmx-type").unwrap_or_default();
        let rmx_type = if rmx_type.is_empty() {
            "text"
        } else {
            &rmx_type
        };
        let value = extract_value(sel, rmx_type);
        item[name.to_string()] = value;
    }

    // Extract from descendant elements with rmx-name, but not those inside nested lists
    let descendants = sel.select("[rmx-name]");
    let nested_lists = sel.select("[rmx-type=\"list\"]");

    for desc_node in descendants.nodes().iter() {
        let desc_sel = Selection::from(desc_node.clone());

        // Skip if same as parent
        if desc_node.id == node.id {
            continue;
        }

        // Skip if inside a nested list
        let inside_nested = nested_lists.nodes().iter().any(|list_node| {
            // Not the same as the descriptor node
            if list_node.id == desc_node.id {
                return false;
            }
            let list_sel = Selection::from(list_node.clone());
            list_sel
                .select("[rmx-name]")
                .nodes()
                .iter()
                .any(|n| n.id == desc_node.id)
        });

        if inside_nested {
            continue;
        }

        if let Some((key, value)) = extract_element(&desc_sel) {
            item[key] = value;
        }
    }

    item
}

// ============================================================================
// Rendering: Recreate HTML from JSON data
// ============================================================================

/// Renders HTML by populating a template with JSON data.
///
/// Uses the same rmx-name and rmx-type attributes to determine where to inject data.
/// For lists, the first child element is used as a template and replicated for each
/// item in the JSON array.
///
/// For web components with `data-rmx`, all attributes are set from the corresponding
/// JSON object, and children are rendered using the first child as a template.
pub fn render_data(html: &str, data: &Value) -> String {
    let document = Document::from(html);

    // Process web components first
    render_web_components(&document, data);

    // Process lists (they need special handling with templates)
    render_lists(&document, data);

    // Then process simple values (text, image, attr)
    render_values(&document, data);

    document.html().to_string()
}

/// Renders all web components (elements with data-rmx attribute) in the document.
fn render_web_components(doc: &Document, data: &Value) {
    let web_components = doc.select("[data-rmx]");

    for node in web_components.nodes().iter() {
        let sel = Selection::from(node.clone());

        let Some(name) = sel.attr("data-rmx") else {
            continue;
        };

        let Some(obj) = data.get(name.as_ref()).and_then(|v| v.as_object()) else {
            continue;
        };

        // Set all attributes from the JSON object (except "children")
        for (attr_name, attr_value) in obj.iter() {
            if attr_name == "children" {
                continue;
            }
            if let Some(value_str) = attr_value.as_str() {
                sel.set_attr(attr_name, value_str);
            }
        }

        // Render children if present
        if let Some(children) = obj.get("children").and_then(|v| v.as_array()) {
            render_web_component_children(&sel, children);
        }
    }
}

/// Renders children of a web component using the first child as a template.
fn render_web_component_children(sel: &Selection, children: &[Value]) {
    // Get the first child element as template
    let existing_children = sel.children();
    let first_child = existing_children.nodes().first();
    let Some(template_node) = first_child else {
        return;
    };

    // Get the tag name of the template child
    let tag_name = template_node
        .node_name()
        .map(|n| n.to_string())
        .unwrap_or_default();

    if tag_name.is_empty() {
        return;
    }

    // Get template info
    let template_html = template_node.html().to_string();

    // Remove all existing children
    sel.children().remove();

    // Generate HTML for each child and append
    for child_data in children {
        let Some(child_obj) = child_data.as_object() else {
            continue;
        };

        let child_doc = Document::from(template_html.as_str());

        // Select the actual element (not html/head/body wrapper)
        let child_sel = child_doc.select(&tag_name);

        // Set all attributes on the child element
        for (attr_name, attr_value) in child_obj.iter() {
            if let Some(value_str) = attr_value.as_str() {
                child_sel.set_attr(attr_name, value_str);
            }
        }

        // Get the modified element HTML
        sel.append_html(child_sel.html().to_string().as_str());
    }
}

/// Renders all list elements in the document.
fn render_lists(doc: &Document, data: &Value) {
    let lists = doc.select("[rmx-type=\"list\"]");

    for node in lists.nodes().iter() {
        let list_sel = Selection::from(node.clone());

        let Some(name) = list_sel.attr("rmx-name") else {
            continue;
        };

        let Some(items) = data.get(name.as_ref()).and_then(|v| v.as_array()) else {
            continue;
        };

        render_list_element(&list_sel, items);
    }
}

/// Renders a single list element with the provided items.
fn render_list_element(list_sel: &Selection, items: &[Value]) {
    // Get the first child element as template
    let children = list_sel.children();
    let first_child = children.nodes().first();
    let Some(template_node) = first_child else {
        return;
    };
    let template_html = template_node.html().to_string();

    // Remove all existing children
    list_sel.children().remove();

    // Generate HTML for each item and append
    for item in items {
        let child_doc = Document::from(template_html.as_str());

        // Recursively render nested lists first
        render_lists(&child_doc, item);

        // Then render simple values
        render_values(&child_doc, item);

        list_sel.append_html(child_doc.html().to_string().as_str());
    }
}

/// Renders simple values (text, image, attr) in the document.
fn render_values(doc: &Document, data: &Value) {
    let elements = doc.select("[rmx-name]");

    for node in elements.nodes().iter() {
        let sel = Selection::from(node.clone());

        let Some(name) = sel.attr("rmx-name") else {
            continue;
        };

        let rmx_type = sel.attr("rmx-type").unwrap_or_default();
        let rmx_type = if rmx_type.is_empty() {
            "text"
        } else {
            &rmx_type
        };

        // Skip lists - they're handled separately
        if rmx_type == "list" {
            continue;
        }

        let Some(value) = data.get(name.as_ref()) else {
            continue;
        };

        let value_str = match value {
            Value::String(s) => s.clone(),
            Value::Null => continue,
            v => v.to_string(),
        };

        render_element_value(&sel, rmx_type, &value_str);
    }
}

/// Renders a single element with a new value.
fn render_element_value(sel: &Selection, rmx_type: &str, value: &str) {
    match rmx_type {
        "text" => {
            sel.set_html(value);
        }
        "image" => {
            sel.set_attr("src", value);
        }
        _ if rmx_type.starts_with("attr:") => {
            let attr_name = &rmx_type[5..];
            sel.set_attr(attr_name, value);
        }
        _ => {
            sel.set_html(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_extract_text() {
        let html = r#"<div rmx-name="title" rmx-type="text">Hello World</div>"#;
        let result = extract_data(html);
        assert_eq!(result, json!({"title": "Hello World"}));
    }

    #[test]
    fn test_extract_text_default_type() {
        let html = r#"<div rmx-name="title">Hello World</div>"#;
        let result = extract_data(html);
        assert_eq!(result, json!({"title": "Hello World"}));
    }

    #[test]
    fn test_extract_text_with_whitespace() {
        let html = r#"<div rmx-name="title" rmx-type="text">
            Hello World
        </div>"#;
        let result = extract_data(html);
        assert_eq!(result, json!({"title": "Hello World"}));
    }

    #[test]
    fn test_extract_image() {
        let html = r#"<img rmx-name="avatar" rmx-type="image" src="https://example.com/pic.jpg">"#;
        let result = extract_data(html);
        assert_eq!(result, json!({"avatar": "https://example.com/pic.jpg"}));
    }

    #[test]
    fn test_extract_image_no_src() {
        let html = r#"<div rmx-name="avatar" rmx-type="image"></div>"#;
        let result = extract_data(html);
        assert_eq!(result, json!({"avatar": null}));
    }

    #[test]
    fn test_extract_multiple_fields() {
        let html = r#"
            <div>
                <h1 rmx-name="title" rmx-type="text">My Title</h1>
                <img rmx-name="image" rmx-type="image" src="https://example.com/img.png">
                <p rmx-name="description" rmx-type="text">Some description</p>
            </div>
        "#;
        let result = extract_data(html);
        assert_eq!(
            result,
            json!({
                "title": "My Title",
                "image": "https://example.com/img.png",
                "description": "Some description"
            })
        );
    }

    #[test]
    fn test_extract_list() {
        let html = r#"
            <ul rmx-name="items" rmx-type="list">
                <li>
                    <span rmx-name="name" rmx-type="text">Item 1</span>
                    <img rmx-name="icon" rmx-type="image" src="https://example.com/1.png">
                </li>
                <li>
                    <span rmx-name="name" rmx-type="text">Item 2</span>
                    <img rmx-name="icon" rmx-type="image" src="https://example.com/2.png">
                </li>
            </ul>
        "#;
        let result = extract_data(html);
        assert_eq!(
            result,
            json!({
                "items": [
                    {"name": "Item 1", "icon": "https://example.com/1.png"},
                    {"name": "Item 2", "icon": "https://example.com/2.png"}
                ]
            })
        );
    }

    #[test]
    fn test_extract_empty_list() {
        let html = r#"<ul rmx-name="items" rmx-type="list"></ul>"#;
        let result = extract_data(html);
        assert_eq!(result, json!({"items": []}));
    }

    #[test]
    fn test_nested_text() {
        let html = r#"<div rmx-name="content" rmx-type="text"><span>Hello</span> <strong>World</strong></div>"#;
        let result = extract_data(html);
        assert_eq!(result, json!({"content": "Hello World"}));
    }

    #[test]
    fn test_no_rmx_attributes() {
        let html = r#"<div>Just some regular HTML</div>"#;
        let result = extract_data(html);
        assert_eq!(result, json!({}));
    }

    #[test]
    fn test_unknown_type_defaults_to_text() {
        let html = r#"<div rmx-name="field" rmx-type="unknown">Content</div>"#;
        let result = extract_data(html);
        assert_eq!(result, json!({"field": "Content"}));
    }

    #[test]
    fn test_extract_attr_href() {
        let html =
            r#"<a rmx-name="link" rmx-type="attr:href" href="https://example.com">Click</a>"#;
        let result = extract_data(html);
        assert_eq!(result, json!({"link": "https://example.com"}));
    }

    #[test]
    fn test_extract_attr_data_attribute() {
        let html = r#"<div rmx-name="item_id" rmx-type="attr:data-id" data-id="12345">Item</div>"#;
        let result = extract_data(html);
        assert_eq!(result, json!({"item_id": "12345"}));
    }

    #[test]
    fn test_extract_attr_missing() {
        let html = r#"<div rmx-name="missing" rmx-type="attr:data-foo">Content</div>"#;
        let result = extract_data(html);
        assert_eq!(result, json!({"missing": null}));
    }

    #[test]
    fn test_extract_attr_class() {
        let html =
            r#"<div rmx-name="classes" rmx-type="attr:class" class="foo bar baz">Content</div>"#;
        let result = extract_data(html);
        assert_eq!(result, json!({"classes": "foo bar baz"}));
    }

    #[test]
    fn test_nested_list_in_list_item() {
        let html = r#"
            <ul rmx-name="categories" rmx-type="list">
                <li>
                    <span rmx-name="name">Electronics</span>
                    <ul rmx-name="products" rmx-type="list">
                        <li>
                            <span rmx-name="title">Phone</span>
                            <span rmx-name="price">$999</span>
                        </li>
                        <li>
                            <span rmx-name="title">Laptop</span>
                            <span rmx-name="price">$1499</span>
                        </li>
                    </ul>
                </li>
                <li>
                    <span rmx-name="name">Books</span>
                    <ul rmx-name="products" rmx-type="list">
                        <li>
                            <span rmx-name="title">Novel</span>
                            <span rmx-name="price">$15</span>
                        </li>
                    </ul>
                </li>
            </ul>
        "#;
        let result = extract_data(html);
        assert_eq!(
            result,
            json!({
                "categories": [
                    {
                        "name": "Electronics",
                        "products": [
                            {"title": "Phone", "price": "$999"},
                            {"title": "Laptop", "price": "$1499"}
                        ]
                    },
                    {
                        "name": "Books",
                        "products": [
                            {"title": "Novel", "price": "$15"}
                        ]
                    }
                ]
            })
        );
    }

    #[test]
    fn test_deeply_nested_structure() {
        let html = r#"
            <div rmx-name="store" rmx-type="list">
                <div>
                    <span rmx-name="department">Clothing</span>
                    <div rmx-name="sections" rmx-type="list">
                        <div>
                            <span rmx-name="section">Men</span>
                            <div rmx-name="items" rmx-type="list">
                                <div>
                                    <span rmx-name="item">Shirt</span>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        "#;
        let result = extract_data(html);
        assert_eq!(
            result,
            json!({
                "store": [
                    {
                        "department": "Clothing",
                        "sections": [
                            {
                                "section": "Men",
                                "items": [
                                    {"item": "Shirt"}
                                ]
                            }
                        ]
                    }
                ]
            })
        );
    }

    #[test]
    fn test_five_levels_deep() {
        let html = r#"
            <div rmx-name="l1" rmx-type="list">
                <div>
                    <span rmx-name="a">1</span>
                    <div rmx-name="l2" rmx-type="list">
                        <div>
                            <span rmx-name="b">2</span>
                            <div rmx-name="l3" rmx-type="list">
                                <div>
                                    <span rmx-name="c">3</span>
                                    <div rmx-name="l4" rmx-type="list">
                                        <div>
                                            <span rmx-name="d">4</span>
                                            <div rmx-name="l5" rmx-type="list">
                                                <div>
                                                    <span rmx-name="e">5</span>
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        "#;
        let result = extract_data(html);
        assert_eq!(
            result,
            json!({
                "l1": [
                    {
                        "a": "1",
                        "l2": [
                            {
                                "b": "2",
                                "l3": [
                                    {
                                        "c": "3",
                                        "l4": [
                                            {
                                                "d": "4",
                                                "l5": [
                                                    {"e": "5"}
                                                ]
                                            }
                                        ]
                                    }
                                ]
                            }
                        ]
                    }
                ]
            })
        );
    }

    // ========================================================================
    // Render tests
    // ========================================================================

    #[test]
    fn test_render_text() {
        let template = r#"<div rmx-name="title" rmx-type="text">Placeholder</div>"#;
        let data = json!({"title": "Hello World"});
        let result = render_data(template, &data);
        assert!(result.contains("Hello World"));
        assert!(!result.contains("Placeholder"));
    }

    #[test]
    fn test_render_image() {
        let template = r#"<img rmx-name="pic" rmx-type="image" src="old.jpg">"#;
        let data = json!({"pic": "new.png"});
        let result = render_data(template, &data);
        assert!(result.contains(r#"src="new.png""#));
        assert!(!result.contains("old.jpg"));
    }

    #[test]
    fn test_render_attr() {
        let template = r#"<a rmx-name="link" rmx-type="attr:href" href="http://old.com">Click</a>"#;
        let data = json!({"link": "http://new.com"});
        let result = render_data(template, &data);
        assert!(result.contains(r#"href="http://new.com""#));
        assert!(!result.contains("http://old.com"));
    }

    #[test]
    fn test_render_list_same_count() {
        let template = r#"<ul rmx-name="items" rmx-type="list"><li><span rmx-name="name" rmx-type="text">X</span></li></ul>"#;
        let data = json!({"items": [{"name": "A"}, {"name": "B"}]});
        let result = render_data(template, &data);
        assert!(result.contains("A"));
        assert!(result.contains("B"));
        assert!(!result.contains("X"));
    }

    #[test]
    fn test_render_list_more_items() {
        let template = r#"<ul rmx-name="items" rmx-type="list"><li><span rmx-name="n" rmx-type="text">X</span></li></ul>"#;
        let data = json!({"items": [{"n": "1"}, {"n": "2"}, {"n": "3"}, {"n": "4"}]});
        let result = render_data(template, &data);
        assert!(result.contains("1"));
        assert!(result.contains("2"));
        assert!(result.contains("3"));
        assert!(result.contains("4"));
    }

    #[test]
    fn test_render_list_fewer_items() {
        let template = r#"<ul rmx-name="items" rmx-type="list"><li><span rmx-name="n" rmx-type="text">X</span></li><li><span rmx-name="n" rmx-type="text">Y</span></li></ul>"#;
        let data = json!({"items": [{"n": "Only"}]});
        let result = render_data(template, &data);
        assert!(result.contains("Only"));
        // Should only have one li now
        assert_eq!(result.matches("<li>").count(), 1);
    }

    #[test]
    fn test_render_empty_list() {
        let template = r#"<ul rmx-name="items" rmx-type="list"><li><span rmx-name="n" rmx-type="text">X</span></li></ul>"#;
        let data = json!({"items": []});
        let result = render_data(template, &data);
        assert!(!result.contains("<li>"));
    }

    #[test]
    fn test_render_nested_list() {
        let template = r#"
            <div rmx-name="categories" rmx-type="list">
                <div>
                    <span rmx-name="name" rmx-type="text">Cat</span>
                    <ul rmx-name="products" rmx-type="list">
                        <li><span rmx-name="title" rmx-type="text">Prod</span></li>
                    </ul>
                </div>
            </div>
        "#;
        let data = json!({
            "categories": [
                {
                    "name": "Electronics",
                    "products": [
                        {"title": "Phone"},
                        {"title": "Laptop"}
                    ]
                },
                {
                    "name": "Books",
                    "products": [
                        {"title": "Novel"}
                    ]
                }
            ]
        });
        let result = render_data(template, &data);
        assert!(result.contains("Electronics"));
        assert!(result.contains("Books"));
        assert!(result.contains("Phone"));
        assert!(result.contains("Laptop"));
        assert!(result.contains("Novel"));
    }

    #[test]
    fn test_render_roundtrip() {
        let template = r#"<div rmx-name="title" rmx-type="text">X</div>"#;
        let data = json!({"title": "Test"});
        let rendered = render_data(template, &data);
        let extracted = extract_data(&rendered);
        assert_eq!(extracted, data);
    }

    #[test]
    fn test_render_list_roundtrip() {
        let template = r#"<ul rmx-name="items" rmx-type="list"><li><span rmx-name="name" rmx-type="text">X</span><span rmx-name="value" rmx-type="text">Y</span></li></ul>"#;
        let data = json!({"items": [{"name": "A", "value": "1"}, {"name": "B", "value": "2"}]});
        let rendered = render_data(template, &data);
        let extracted = extract_data(&rendered);
        assert_eq!(extracted, data);
    }

    #[test]
    fn test_render_preserves_other_content() {
        let template = r#"<div class="wrapper"><h1>Title</h1><p rmx-name="desc" rmx-type="text">Old</p><footer>Footer</footer></div>"#;
        let data = json!({"desc": "New Description"});
        let result = render_data(template, &data);
        assert!(result.contains("class=\"wrapper\""));
        assert!(result.contains("<h1>Title</h1>"));
        assert!(result.contains("<footer>Footer</footer>"));
        assert!(result.contains("New Description"));
    }

    #[test]
    fn test_two_level_nested_roundtrip() {
        // Template with two levels of nesting: categories -> products
        let template = r##"
            <div rmx-name="categories" rmx-type="list">
                <div>
                    <span rmx-name="category_name" rmx-type="text">Category</span>
                    <img rmx-name="category_icon" rmx-type="image" src="icon.png">
                    <ul rmx-name="products" rmx-type="list">
                        <li>
                            <span rmx-name="product_name" rmx-type="text">Product</span>
                            <span rmx-name="price" rmx-type="text">$0</span>
                            <a rmx-name="link" rmx-type="attr:href" href="#">Buy</a>
                        </li>
                    </ul>
                </div>
            </div>
        "##;

        let data = json!({
            "categories": [
                {
                    "category_name": "Electronics",
                    "category_icon": "https://example.com/electronics.png",
                    "products": [
                        {"product_name": "Phone", "price": "$999", "link": "https://example.com/phone"},
                        {"product_name": "Laptop", "price": "$1499", "link": "https://example.com/laptop"},
                        {"product_name": "Tablet", "price": "$599", "link": "https://example.com/tablet"}
                    ]
                },
                {
                    "category_name": "Books",
                    "category_icon": "https://example.com/books.png",
                    "products": [
                        {"product_name": "Novel", "price": "$15", "link": "https://example.com/novel"},
                        {"product_name": "Textbook", "price": "$80", "link": "https://example.com/textbook"}
                    ]
                }
            ]
        });

        // Render the template with data
        let rendered = render_data(template, &data);

        // Verify rendered content contains expected values
        assert!(rendered.contains("Electronics"));
        assert!(rendered.contains("Books"));
        assert!(rendered.contains("Phone"));
        assert!(rendered.contains("Laptop"));
        assert!(rendered.contains("Tablet"));
        assert!(rendered.contains("Novel"));
        assert!(rendered.contains("Textbook"));
        assert!(rendered.contains("$999"));
        assert!(rendered.contains("$1499"));
        assert!(rendered.contains("https://example.com/electronics.png"));
        assert!(rendered.contains("https://example.com/phone"));

        // Round-trip: extract from rendered HTML
        let extracted = extract_data(&rendered);

        // Verify the extracted data matches the original
        assert_eq!(extracted, data);
    }

    #[test]
    fn test_two_level_nested_different_item_counts() {
        // Test that nested lists can have different numbers of items
        let template = r#"
            <div rmx-name="groups" rmx-type="list">
                <div>
                    <span rmx-name="group" rmx-type="text">G</span>
                    <ul rmx-name="members" rmx-type="list">
                        <li><span rmx-name="name" rmx-type="text">M</span></li>
                    </ul>
                </div>
            </div>
        "#;

        let data = json!({
            "groups": [
                {
                    "group": "Team A",
                    "members": [
                        {"name": "Alice"},
                        {"name": "Bob"},
                        {"name": "Charlie"}
                    ]
                },
                {
                    "group": "Team B",
                    "members": [
                        {"name": "David"}
                    ]
                },
                {
                    "group": "Team C",
                    "members": [
                        {"name": "Eve"},
                        {"name": "Frank"}
                    ]
                }
            ]
        });

        let rendered = render_data(template, &data);
        let extracted = extract_data(&rendered);
        assert_eq!(extracted, data);
    }

    // ========================================================================
    // Web component tests
    // ========================================================================

    #[test]
    fn test_extract_web_component_attributes() {
        let html = r#"<my-button data-rmx="button" label="Click me" variant="primary" disabled="true"></my-button>"#;
        let result = extract_data(html);
        assert_eq!(
            result,
            json!({
                "button": {
                    "label": "Click me",
                    "variant": "primary",
                    "disabled": "true"
                }
            })
        );
    }

    #[test]
    fn test_extract_web_component_with_children() {
        let html = r#"
            <image-carousel data-rmx="carousel" speed="500" autoplay="true">
                <img src="img1.png" alt="First">
                <img src="img2.png" alt="Second">
                <img src="img3.png" alt="Third">
            </image-carousel>
        "#;
        let result = extract_data(html);
        assert_eq!(
            result,
            json!({
                "carousel": {
                    "speed": "500",
                    "autoplay": "true",
                    "children": [
                        {"src": "img1.png", "alt": "First"},
                        {"src": "img2.png", "alt": "Second"},
                        {"src": "img3.png", "alt": "Third"}
                    ]
                }
            })
        );
    }

    #[test]
    fn test_render_web_component_attributes() {
        let template = r#"<my-button data-rmx="btn" label="Old" variant="secondary"></my-button>"#;
        let data = json!({
            "btn": {
                "label": "New Label",
                "variant": "primary"
            }
        });
        let result = render_data(template, &data);
        assert!(result.contains(r#"label="New Label""#));
        assert!(result.contains(r#"variant="primary""#));
    }

    #[test]
    fn test_render_web_component_children() {
        let template = r#"
            <image-carousel data-rmx="carousel" speed="300">
                <img src="placeholder.png" alt="Placeholder">
            </image-carousel>
        "#;
        let data = json!({
            "carousel": {
                "speed": "500",
                "children": [
                    {"src": "a.png", "alt": "Image A"},
                    {"src": "b.png", "alt": "Image B"}
                ]
            }
        });
        let result = render_data(template, &data);
        assert!(result.contains(r#"speed="500""#));
        assert!(result.contains(r#"src="a.png""#));
        assert!(result.contains(r#"src="b.png""#));
        assert!(result.contains(r#"alt="Image A""#));
        assert!(result.contains(r#"alt="Image B""#));
        assert!(!result.contains("placeholder.png"));
    }

    #[test]
    fn test_web_component_roundtrip() {
        let template = r#"
            <custom-tabs data-rmx="tabs" active="0">
                <div slot="tab" title="Tab 1"></div>
            </custom-tabs>
        "#;
        let data = json!({
            "tabs": {
                "active": "2",
                "children": [
                    {"slot": "tab", "title": "Home"},
                    {"slot": "tab", "title": "About"},
                    {"slot": "tab", "title": "Contact"}
                ]
            }
        });
        let rendered = render_data(template, &data);
        let extracted = extract_data(&rendered);
        assert_eq!(extracted, data);
    }

    #[test]
    fn test_web_component_no_children() {
        let html = r#"<my-icon data-rmx="icon" name="star" size="24"></my-icon>"#;
        let result = extract_data(html);
        assert_eq!(
            result,
            json!({
                "icon": {
                    "name": "star",
                    "size": "24"
                }
            })
        );

        // Render and roundtrip
        let data = json!({
            "icon": {
                "name": "heart",
                "size": "32"
            }
        });
        let rendered = render_data(html, &data);
        assert!(rendered.contains(r#"name="heart""#));
        assert!(rendered.contains(r#"size="32""#));
    }

    // ========================================================================
    // Integration test with remix_home_annotated.html
    // ========================================================================

    #[test]
    fn test_remix_home_extract() {
        let html = include_str!("../html/remix_home_annotated.html");
        let data = extract_data(html);

        // Verify hero section extraction
        assert_eq!(data["hero_title_line1"], "Digital Experience Platform");
        assert_eq!(data["hero_title_highlight"], "Modern Data Stack");
        assert!(data["hero_description"]
            .as_str()
            .unwrap()
            .contains("AI-enabled"));

        // Verify patterns list extraction
        let patterns = data["patterns"].as_array().unwrap();
        assert!(!patterns.is_empty());
        assert_eq!(patterns[0]["number"], "1");
        assert_eq!(patterns[0]["title"], "Standalone Deployment");

        // Verify experiences list extraction
        let experiences = data["experiences"].as_array().unwrap();
        assert!(!experiences.is_empty());
        assert_eq!(experiences[0]["icon"], "📱");
        assert_eq!(experiences[0]["title"], "Widgets in Mobile");

        // Verify CTA section
        assert_eq!(data["cta_title"], "Ready to Build Your Next Experience?");
        assert_eq!(data["cta_primary_text"], "Get Started Free");
    }

    #[test]
    fn test_remix_home_render_and_roundtrip() {
        let template = include_str!("../html/remix_home_annotated.html");

        // Create new data to render
        let new_data = json!({
            "company_name": "Acme Corp",
            "logo_letter": "A",
            "hero_title_line1": "Build Amazing Apps",
            "hero_title_highlight": "With Our Platform",
            "hero_description": "The best platform for building modern applications.",
            "hero_cta_primary_text": "Start Now",
            "hero_cta_secondary_text": "Learn More",
            "patterns_badge": "How It Works",
            "patterns_title": "Two Deployment Options",
            "patterns_description": "Choose the right approach for your needs.",
            "patterns": [
                {
                    "number": "1",
                    "title": "Cloud Hosted",
                    "description": "We manage everything for you.",
                    "best_for": "Startups and small teams"
                },
                {
                    "number": "2",
                    "title": "Self Hosted",
                    "description": "Full control over your infrastructure.",
                    "best_for": "Enterprise customers"
                }
            ],
            "experiences_badge": "Features",
            "experiences_title": "What You Get",
            "experiences_description": "Everything you need to succeed.",
            "experiences": [
                {
                    "icon": "🚀",
                    "title": "Fast Deployment",
                    "description": "Deploy in minutes, not hours."
                },
                {
                    "icon": "🔒",
                    "title": "Secure by Default",
                    "description": "Enterprise-grade security built in."
                },
                {
                    "icon": "📊",
                    "title": "Analytics",
                    "description": "Understand your users better."
                }
            ],
            "cta_title": "Ready to Get Started?",
            "cta_description": "Join thousands of happy customers.",
            "cta_primary_text": "Sign Up Free",
            "cta_secondary_text": "Contact Sales",
            "footer_brand_name": "Acme Corp",
            "footer_brand_description": "Building the future of software.",
            "footer_copyright": "2025 Acme Corp. All rights reserved."
        });

        // Render with new data
        let rendered = render_data(template, &new_data);

        // Verify rendered content
        assert!(rendered.contains("Acme Corp"));
        assert!(rendered.contains("Build Amazing Apps"));
        assert!(rendered.contains("With Our Platform"));
        assert!(rendered.contains("Cloud Hosted"));
        assert!(rendered.contains("Self Hosted"));
        assert!(rendered.contains("Fast Deployment"));
        assert!(rendered.contains("Secure by Default"));
        assert!(rendered.contains("Analytics"));
        assert!(rendered.contains("🚀"));
        assert!(rendered.contains("🔒"));
        assert!(rendered.contains("📊"));

        // Round-trip: extract from rendered and verify key fields
        let extracted = extract_data(&rendered);
        assert_eq!(extracted["company_name"], "Acme Corp");
        assert_eq!(extracted["hero_title_line1"], "Build Amazing Apps");
        assert_eq!(extracted["cta_title"], "Ready to Get Started?");

        // Verify lists were rendered correctly
        let patterns = extracted["patterns"].as_array().unwrap();
        assert_eq!(patterns.len(), 2);
        assert_eq!(patterns[0]["title"], "Cloud Hosted");
        assert_eq!(patterns[1]["title"], "Self Hosted");

        let experiences = extracted["experiences"].as_array().unwrap();
        assert_eq!(experiences.len(), 3);
        assert_eq!(experiences[0]["title"], "Fast Deployment");
        assert_eq!(experiences[1]["title"], "Secure by Default");
        assert_eq!(experiences[2]["title"], "Analytics");
    }

    #[test]
    fn test_salesforce_branding_and_write_file() {
        use std::fs;

        let template = include_str!("../html/remix_home_annotated.html");

        // Create Salesforce-branded data
        let salesforce_data = json!({
            "page_title": "Salesforce - The Customer Company",
            "company_name": "Salesforce",
            "logo_letter": "S",
            "hero_title_line1": "The #1 AI CRM",
            "hero_title_highlight": "Customer 360",
            "hero_description": "Unite your teams around your customer with AI-powered apps. Get a complete view of every customer relationship.",
            "hero_cta_primary": "https://salesforce.com/trial",
            "hero_cta_primary_text": "Start Free Trial",
            "hero_cta_secondary": "https://salesforce.com/demo",
            "hero_cta_secondary_text": "Watch Demo",
            "nav_links": [
                {"text": "Products"},
                {"text": "Industries"},
                {"text": "Customers"},
                {"text": "Learning"},
                {"text": "Try for Free"}
            ],
            "patterns_badge": "Platform Solutions",
            "patterns_title": "Three Clouds, One Platform",
            "patterns_description": "Everything you need to connect with your customers in a whole new way.",
            "patterns": [
                {
                    "number": "1",
                    "title": "Sales Cloud",
                    "description": "Sell faster, smarter, and more efficiently with AI + Data + CRM. Close more deals and accelerate growth.",
                    "features": [
                        {"text": "Lead and opportunity management"},
                        {"text": "AI-powered sales insights"},
                        {"text": "Pipeline management"},
                        {"text": "Sales forecasting"}
                    ],
                    "best_for": "Sales teams looking to close more deals faster"
                },
                {
                    "number": "2",
                    "title": "Service Cloud",
                    "description": "Deliver personalized service at scale. Resolve cases faster with AI-powered tools.",
                    "features": [
                        {"text": "Omni-channel case management"},
                        {"text": "AI-powered chatbots"},
                        {"text": "Knowledge base integration"},
                        {"text": "Field service management"}
                    ],
                    "best_for": "Support teams delivering exceptional customer service"
                },
                {
                    "number": "3",
                    "title": "Marketing Cloud",
                    "description": "Create personalized journeys across every touchpoint. Drive engagement with data-driven marketing.",
                    "features": [
                        {"text": "Email and journey builder"},
                        {"text": "Social media management"},
                        {"text": "Advertising studio"},
                        {"text": "Analytics and personalization"}
                    ],
                    "best_for": "Marketers creating personalized customer journeys"
                }
            ],
            "experiences_badge": "Product Suite",
            "experiences_title": "The Full Salesforce Platform",
            "experiences_description": "Every app you need to deliver customer success, powered by AI.",
            "experiences": [
                {
                    "icon": "💼",
                    "title": "Sales Cloud",
                    "description": "Close more deals with intelligent sales automation and AI insights."
                },
                {
                    "icon": "🎧",
                    "title": "Service Cloud",
                    "description": "Deliver personalized support across every channel, powered by AI."
                },
                {
                    "icon": "📧",
                    "title": "Marketing Cloud",
                    "description": "Create personalized marketing at scale with unified customer data."
                },
                {
                    "icon": "🛒",
                    "title": "Commerce Cloud",
                    "description": "Unified commerce experiences across B2B and B2C."
                },
                {
                    "icon": "🤖",
                    "title": "Einstein AI",
                    "description": "AI that works across every Salesforce cloud and workflow."
                },
                {
                    "icon": "📊",
                    "title": "Tableau",
                    "description": "Visual analytics and business intelligence for everyone."
                },
                {
                    "icon": "🔗",
                    "title": "MuleSoft",
                    "description": "Connect any app, data, or device with API-led integration."
                },
                {
                    "icon": "💬",
                    "title": "Slack",
                    "description": "Bring teams together with the AI-powered work OS."
                },
                {
                    "icon": "⚙️",
                    "title": "Platform",
                    "description": "Build and extend apps with low-code tools and pro-code options."
                }
            ],
            "cta_title": "Start Your Free Trial Today",
            "cta_description": "Join 150,000+ companies growing with Salesforce. No credit card required.",
            "cta_primary_link": "https://salesforce.com/trial",
            "cta_primary_text": "Try for Free",
            "cta_secondary_link": "https://salesforce.com/contact",
            "cta_secondary_text": "Contact Sales",
            "footer_brand_name": "Salesforce",
            "footer_brand_description": "Salesforce brings companies and customers together. It's one integrated CRM platform that gives all your teams a single, shared view of every customer.",
            "footer_links_platform": [
                {"text": "Sales Cloud"},
                {"text": "Service Cloud"},
                {"text": "Marketing Cloud"},
                {"text": "Commerce Cloud"}
            ],
            "footer_copyright": "© 2025 Salesforce, Inc. All rights reserved."
        });

        // Render with Salesforce data
        let rendered = render_data(template, &salesforce_data);

        // Write to file
        let output_path = "html/salesforce_home.html";
        fs::write(output_path, &rendered).expect("Failed to write salesforce_home.html");

        // Verify file was written
        let read_back = fs::read_to_string(output_path).expect("Failed to read back file");
        assert_eq!(read_back, rendered);

        // Verify content
        assert!(rendered.contains("Salesforce"));
        assert!(rendered.contains("The #1 AI CRM"));
        assert!(rendered.contains("Customer 360"));
        assert!(rendered.contains("Sales Cloud"));
        assert!(rendered.contains("Service Cloud"));
        assert!(rendered.contains("Marketing Cloud"));
        assert!(rendered.contains("Einstein AI"));
        assert!(rendered.contains("Tableau"));
        assert!(rendered.contains("MuleSoft"));
        assert!(rendered.contains("Slack"));
        assert!(rendered.contains("150,000+"));
        assert!(rendered.contains("https://salesforce.com/trial"));

        // Round-trip: extract and verify
        let extracted = extract_data(&rendered);
        assert_eq!(extracted["company_name"], "Salesforce");
        assert_eq!(extracted["hero_title_line1"], "The #1 AI CRM");
        assert_eq!(extracted["hero_title_highlight"], "Customer 360");

        // Verify patterns with nested features
        let patterns = extracted["patterns"].as_array().unwrap();
        assert_eq!(patterns.len(), 3);
        assert_eq!(patterns[0]["title"], "Sales Cloud");
        assert_eq!(patterns[1]["title"], "Service Cloud");
        assert_eq!(patterns[2]["title"], "Marketing Cloud");

        // Verify nested features were preserved
        let features = patterns[0]["features"].as_array().unwrap();
        assert_eq!(features.len(), 4);
        assert_eq!(features[0]["text"], "Lead and opportunity management");

        // Verify experiences
        let experiences = extracted["experiences"].as_array().unwrap();
        assert_eq!(experiences.len(), 9);
        assert_eq!(experiences[4]["title"], "Einstein AI");
        assert_eq!(experiences[5]["title"], "Tableau");
        assert_eq!(experiences[6]["title"], "MuleSoft");
        assert_eq!(experiences[7]["title"], "Slack");

        // Clean up the generated file
        fs::remove_file(output_path).expect("Failed to clean up test file");
    }
}
