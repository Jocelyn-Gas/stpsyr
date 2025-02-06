use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;

use stpsyr::types::*;

impl Stpsyr {
    pub fn render(self, output_file: String) {
        let file_path = "data/standard.svg";
        let mut colors = HashMap::new();
        let mut definitions = HashMap::new();

        definitions.insert(Power::from("Austria"), "#B19517");
        definitions.insert(Power::from("Russia"), "#613697");
        definitions.insert(Power::from("Italy"), "#4CB56A");
        definitions.insert(Power::from("Germany"), "#C66813");
        definitions.insert(Power::from("France"), "#0766B9");
        definitions.insert(Power::from("England"), "#9C1E1E");
        definitions.insert(Power::from("Turkey"), "#3487AD");

        // Read SVG file
        let svg_content = fs::read_to_string(file_path).expect("Failed to read SVG file");

        // Step 1: Collect colors based on ownership
        for region in self.map.iter() {
            if let Some(ref owner) = region.owner {
                println!("Region {:?} is owned by {:?}", region.province, owner);
                colors.insert(
                    region.province.name.clone(),
                    definitions.get(owner).unwrap().to_string(),
                );
            }
        }

        // Step 2: Modify SVG colors
        let modified_svg = change_fill_colors(&svg_content, &colors);

        // Step 3: Open output file with buffering
        let mut out_file = File::create(output_file).expect("Failed to create output file");

        // Step 4: Write modified SVG up to `</svg>`
        let svg_closing = "</svg>";
        let split_index = modified_svg
            .rfind(svg_closing)
            .unwrap_or(modified_svg.len());
        let content_before_svg_end = &modified_svg[..split_index]; // Slice before </svg>
        writeln!(out_file, "{}", content_before_svg_end).unwrap();

        // Step 5: Append new units
        for region in self.map.iter() {
            if let Some(ref unit) = region.unit {
                let (x, y) = region.center;
                println!("Name: {}", &unit.owner.name);
                let color = adjust_luminance(definitions.get(&unit.owner).unwrap(), 15.);

                match unit.unit_type {
                    UnitType::Army => {
                        writeln!(out_file,
r#"<g style="fill:{}" stroke="black" stroke-width="2" transform="translate({},{})">
    <path d="M3.50009 14L7.00009 18.5H35L36 17.5L39 14.5C41.8 11.7 39.5 8.5 37.5 8L27.5 7.5H23.5V6.5H26.5C27 6 28.8 5 32 5C35.2 5 33.3333 3 32 2H29L28 1H24L23 2H19.5L18.5 2.5H14L13 3.5H1V5H13L16 6.5H18.5V7.5H11.5C11 8.5 8.5 10 7.00009 10C2.20009 10 2.66676 12.6667 3.50009 14Z" stroke="black"/>
</g>"#, color, x, y).unwrap();
                    }
                    UnitType::Fleet => {
                        writeln!(out_file,
r#"<g style="fill:{}" stroke="black" stroke-width="2" transform="translate({},{})">
    <path d="M31 14.5V12M31 12V11H28.5V8H26.5L25 6.5V1H23V3L21 4.5V10V8H20.5V5H17V10V7H15V9.5H14V12H12V14H11.5V12H9V14H1V16.5L2.5 18H41.5L45 12H41V13H37V14H34V12M31 12H34M35.5 12H34" stroke="black"/>
</g>"#, color, x, y).unwrap();
                    }
                }
            }
        }

        // Step 6: Close the SVG tag
        writeln!(out_file, "</svg>").unwrap();
    }
}

fn change_fill_colors(svg: &str, color_map: &HashMap<String, String>) -> String {
    let mut result = String::new();
    let mut inside_target_group: Option<String> = None;

    for line in svg.lines() {
        let mut modified_line = line.to_string();

        // Check if the line contains an id from our map
        for id in color_map.keys() {
            if line.contains(&format!("id=\"{}\"", id)) {
                inside_target_group = Some(id.clone());
                break;
            }
        }

        // If inside a group and we find a fill attribute, replace it
        if let Some(ref id) = inside_target_group {
            if modified_line.contains("fill=") {
                if let Some(new_color) = color_map.get(id) {
                    modified_line = replace_fill(&modified_line, new_color);
                }
            }
        }

        result.push_str(&modified_line);
        result.push('\n');

        // Stop modifying when we exit the group
        if inside_target_group.is_some() && line.contains("</g>") {
            inside_target_group = None;
        }
    }

    result
}

fn replace_fill(line: &str, new_color: &str) -> String {
    let fill_pattern = r#"fill="([^"]*)""#; // Match `fill="..."`

    let replaced_line = regex::Regex::new(fill_pattern)
        .unwrap()
        .replace(line, format!(r#"fill="{}""#, new_color))
        .to_string();

    replaced_line
}
fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let r = u8::from_str_radix(&hex[1..3], 16).unwrap();
    let g = u8::from_str_radix(&hex[3..5], 16).unwrap();
    let b = u8::from_str_radix(&hex[5..7], 16).unwrap();
    (r, g, b)
}

fn rgb_to_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

// Convert RGB to HSL
fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    let mut h = 0.0;
    let mut s = 0.0;

    if max != min {
        let d = max - min;
        s = if l > 0.5 {
            d / (2.0 - max - min)
        } else {
            d / (max + min)
        };

        h = if max == r {
            (g - b) / d + if g < b { 6.0 } else { 0.0 }
        } else if max == g {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        };
        h /= 6.0;
    }
    (h, s, l)
}

// Convert HSL back to RGB
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let hue_to_rgb = |p: f32, q: f32, t: f32| -> f32 {
        let mut t = t;
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            return p + (q - p) * 6.0 * t;
        }
        if t < 1.0 / 2.0 {
            return q;
        }
        if t < 2.0 / 3.0 {
            return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
        }
        p
    };

    if s == 0.0 {
        let gray = (l * 255.0) as u8;
        return (gray, gray, gray);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;

    let r = (hue_to_rgb(p, q, h + 1.0 / 3.0) * 255.0) as u8;
    let g = (hue_to_rgb(p, q, h) * 255.0) as u8;
    let b = (hue_to_rgb(p, q, h - 1.0 / 3.0) * 255.0) as u8;

    (r, g, b)
}

// Adjust luminance by +10
fn adjust_luminance(hex: &str, delta_l: f32) -> String {
    let (r, g, b) = hex_to_rgb(hex);
    let (h, s, mut l) = rgb_to_hsl(r, g, b);

    l = (l + delta_l / 100.0).clamp(0.0, 1.0); // Ensure it's within range [0,1]

    let (new_r, new_g, new_b) = hsl_to_rgb(h, s, l);
    rgb_to_hex(new_r, new_g, new_b)
}
