//! Shared parser utilities used by both `native_parser` and `configurable_parser`.

use crate::models::{LayoutMapping, SectionLayout};
use regex::Regex;
use scraper::{Html, Selector};

/// Parse a CSS selector string, returning a descriptive error on failure.
pub fn parse_selector(selector: &str) -> Result<Selector, String> {
    Selector::parse(selector).map_err(|e| format!("Invalid selector '{}': {:?}", selector, e))
}

/// Extract the text content of the first element matching `selector`.
pub fn extract_text(document: &Html, selector: &str) -> Option<String> {
    let sel = Selector::parse(selector).ok()?;
    document
        .select(&sel)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string())
}

/// Extract text content from all elements matching `selector`.
pub fn extract_all_text(document: &Html, selector: &str) -> Vec<String> {
    Selector::parse(selector)
        .ok()
        .map(|sel| {
            document
                .select(&sel)
                .map(|el| el.text().collect::<String>().trim().to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// Extract the value of attribute `attr` from the first element matching `selector`.
pub fn extract_attr(document: &Html, selector: &str, attr: &str) -> Option<String> {
    let sel = Selector::parse(selector).ok()?;
    document
        .select(&sel)
        .next()
        .and_then(|el| el.value().attr(attr))
        .map(|s| s.to_string())
}

/// Extract an ID from `href` using the first capture group in `regex`.
pub fn extract_id_from_pattern(href: &str, regex: &Regex) -> Option<String> {
    regex
        .captures(href)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
        .filter(|s| !s.is_empty())
}

/// Determine the layout for a section based on its title and layout mappings.
pub fn determine_layout(title: &str, mappings: &[LayoutMapping]) -> SectionLayout {
    for mapping in mappings {
        if title.to_lowercase().contains(&mapping.title_contains.to_lowercase()) {
            return mapping.layout.clone();
        }
    }
    SectionLayout::Grid
}

/// Make a URL absolute if it's relative, using `base_url` as the origin.
pub fn make_absolute_url(url: &str, base_url: &str) -> String {
    if url.is_empty() {
        return String::new();
    }
    if url.starts_with("http://") || url.starts_with("https://") {
        return url.to_string();
    }
    if url.starts_with("//") {
        return format!("https:{}", url);
    }
    if url.starts_with('/') {
        if let Some(pos) = base_url.find("://") {
            let after_scheme = &base_url[pos + 3..];
            if let Some(slash_pos) = after_scheme.find('/') {
                let origin = &base_url[..pos + 3 + slash_pos];
                return format!("{}{}", origin, url);
            }
        }
        return format!("{}{}", base_url.trim_end_matches('/'), url);
    }
    format!("{}/{}", base_url.trim_end_matches('/'), url)
}

/// Strip HTML tags and ad markers from chapter content, converting to plain text.
pub fn sanitize_chapter_html(html: &str) -> String {
    let ad_re = Regex::new(r"(?is)<!--\s*END\s+AADS\s+AD\s+UNIT\s+\d+\s*--\s*>").unwrap();
    let mut text = ad_re.replace_all(html, "").to_string();

    for (from, to) in [
        ("<br>", "\n"),
        ("<br/>", "\n"),
        ("<br />", "\n"),
        ("</p>", "\n\n"),
        ("</div>", "\n"),
        ("</li>", "\n"),
        ("</h1>", "\n\n"),
        ("</h2>", "\n\n"),
        ("</h3>", "\n\n"),
        ("</h4>", "\n\n"),
        ("</h5>", "\n\n"),
        ("</h6>", "\n\n"),
    ] {
        text = text.replace(from, to);
    }

    let tag_re = Regex::new(r"(?is)<[^>]+>").unwrap();
    text = tag_re.replace_all(&text, "").to_string();

    text = text
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'");

    let line_re = Regex::new(r"\n{3,}").unwrap();
    line_re.replace_all(text.trim(), "\n\n").to_string()
}
