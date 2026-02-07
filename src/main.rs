use htmlreader::{extract_data, render_data};
use serde_json::{json, Value};
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Usage: htmlreader <template.html> [data.json] [output.html]
    // - If only template is provided: extract and print JSON
    // - If template and data are provided: render and print HTML
    // - If template, data, and output are provided: render and write to file

    let template_path = if args.len() > 1 {
        args[1].clone()
    } else {
        "html/remix_home_annotated.html".to_string()
    };

    // Read the template HTML file
    let template = match fs::read_to_string(&template_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading template '{}': {}", template_path, e);
            std::process::exit(1);
        }
    };

    // If data file is provided, render; otherwise extract
    if args.len() > 2 {
        let data_path = &args[2];

        // Read and parse the JSON data file
        let data_str = match fs::read_to_string(data_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Error reading data file '{}': {}", data_path, e);
                std::process::exit(1);
            }
        };

        let data: Value = match serde_json::from_str(&data_str) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Error parsing JSON from '{}': {}", data_path, e);
                std::process::exit(1);
            }
        };

        // Render the template with the data
        let rendered = render_data(&template, &data);

        // If output file is provided, write to it; otherwise print to stdout
        if args.len() > 3 {
            let output_path = &args[3];
            match fs::write(output_path, &rendered) {
                Ok(_) => {
                    eprintln!("Wrote rendered HTML to '{}'", output_path);
                }
                Err(e) => {
                    eprintln!("Error writing to '{}': {}", output_path, e);
                    std::process::exit(1);
                }
            }
        } else {
            println!("{}", rendered);
        }
    } else {
        // Extract mode: extract data and print as JSON
        let data = extract_data(&template);
        let sections = organize_into_sections(&data);
        println!("{}", serde_json::to_string_pretty(&sections).unwrap());
    }
}

/// Organizes flat extracted data into logical sections
fn organize_into_sections(data: &Value) -> Value {
    let obj = match data.as_object() {
        Some(o) => o,
        None => return data.clone(),
    };

    let mut hero = json!({});
    let mut patterns = json!({});
    let mut experiences = json!({});
    let mut cta = json!({});
    let mut footer = json!({});
    let mut nav = json!({});
    let mut meta = json!({});

    for (key, value) in obj {
        if key.starts_with("hero_") {
            let short_key = key.strip_prefix("hero_").unwrap();
            hero[short_key] = value.clone();
        } else if key.starts_with("patterns_") {
            let short_key = key.strip_prefix("patterns_").unwrap();
            patterns[short_key] = value.clone();
        } else if key == "patterns" {
            patterns["items"] = value.clone();
        } else if key.starts_with("experiences_") {
            let short_key = key.strip_prefix("experiences_").unwrap();
            experiences[short_key] = value.clone();
        } else if key == "experiences" {
            experiences["items"] = value.clone();
        } else if key.starts_with("cta_") {
            let short_key = key.strip_prefix("cta_").unwrap();
            cta[short_key] = value.clone();
        } else if key.starts_with("footer_") {
            let short_key = key.strip_prefix("footer_").unwrap();
            footer[short_key] = value.clone();
        } else if key.starts_with("nav_") {
            let short_key = key.strip_prefix("nav_").unwrap();
            nav[short_key] = value.clone();
        } else if key == "page_title" || key == "company_name" || key == "logo_letter" {
            meta[key] = value.clone();
        } else {
            // Put unmatched keys in meta
            meta[key] = value.clone();
        }
    }

    json!({
        "meta": meta,
        "nav": nav,
        "hero": hero,
        "patterns": patterns,
        "experiences": experiences,
        "cta": cta,
        "footer": footer
    })
}
