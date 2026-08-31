#[derive(Debug, Clone)]
pub enum ReaderSpan {
    Text(String),
    Bold(String),
    Italic(String),
    BoldItalic(String),
}

#[derive(Debug, Clone)]
pub enum ReaderBlock {
    Paragraph(Vec<ReaderSpan>),
    Heading(String, u8), // text, level
    Image(String),       // url or path
}

pub fn parse_html(html: &str) -> Vec<ReaderBlock> {
    // 1. Tokenize HTML into Tags and Text
    let mut tokens = Vec::new();
    let mut current_text = String::new();

    let chars: Vec<char> = html.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '<' {
            if !current_text.is_empty() {
                tokens.push(HtmlToken::Text(unescape_html(&current_text)));
                current_text.clear();
            }
            let mut tag = String::new();
            i += 1;
            while i < chars.len() && chars[i] != '>' {
                tag.push(chars[i]);
                i += 1;
            }
            tokens.push(HtmlToken::Tag(tag));
        } else {
            current_text.push(c);
        }
        i += 1;
    }
    if !current_text.is_empty() {
        tokens.push(HtmlToken::Text(unescape_html(&current_text)));
    }

    // 2. Parse tokens into blocks
    let mut blocks = Vec::new();
    let mut current_paragraph = Vec::new();
    let mut current_heading = None; // (text, level)

    let mut bold = false;
    let mut italic = false;

    for token in tokens {
        match token {
            HtmlToken::Tag(tag) => {
                let tag_lower = tag.to_lowercase();
                let tag_name = tag_lower.split_whitespace().next().unwrap_or("");

                match tag_name {
                    "p" | "div" => {
                        flush_paragraph(&mut blocks, &mut current_paragraph);
                    }
                    "/p" | "/div" => {
                        flush_paragraph(&mut blocks, &mut current_paragraph);
                    }
                    "br" => {
                        flush_paragraph(&mut blocks, &mut current_paragraph);
                    }
                    "strong" | "b" => {
                        bold = true;
                    }
                    "/strong" | "/b" => {
                        bold = false;
                    }
                    "em" | "i" => {
                        italic = true;
                    }
                    "/em" | "/i" => {
                        italic = false;
                    }
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        flush_paragraph(&mut blocks, &mut current_paragraph);
                        let level = tag_name.chars().nth(1).unwrap_or('3').to_digit(10).unwrap_or(3) as u8;
                        current_heading = Some((String::new(), level));
                    }
                    "/h1" | "/h2" | "/h3" | "/h4" | "/h5" | "/h6" => {
                        if let Some((text, level)) = current_heading.take() {
                            if !text.trim().is_empty() {
                                blocks.push(ReaderBlock::Heading(text.trim().to_string(), level));
                            }
                        }
                    }
                    "img" => {
                        flush_paragraph(&mut blocks, &mut current_paragraph);
                        if let Some(src) = parse_image_src(&tag) {
                            blocks.push(ReaderBlock::Image(src));
                        }
                    }
                    _ => {}
                }
            }
            HtmlToken::Text(text) => {
                let text_trimmed = text.replace('\r', "").replace('\n', " ");
                if text_trimmed.is_empty() {
                    continue;
                }
                
                if let Some((h_text, _)) = &mut current_heading {
                    h_text.push_str(&text_trimmed);
                } else {
                    let span = match (bold, italic) {
                        (true, true) => ReaderSpan::BoldItalic(text_trimmed),
                        (true, false) => ReaderSpan::Bold(text_trimmed),
                        (false, true) => ReaderSpan::Italic(text_trimmed),
                        (false, false) => ReaderSpan::Text(text_trimmed),
                    };
                    current_paragraph.push(span);
                }
            }
        }
    }

    flush_paragraph(&mut blocks, &mut current_paragraph);

    blocks
}

#[derive(Debug)]
enum HtmlToken {
    Tag(String),
    Text(String),
}

fn flush_paragraph(blocks: &mut Vec<ReaderBlock>, current_paragraph: &mut Vec<ReaderSpan>) {
    if !current_paragraph.is_empty() {
        let has_content = current_paragraph.iter().any(|span| {
            let text = match span {
                ReaderSpan::Text(t) => t,
                ReaderSpan::Bold(t) => t,
                ReaderSpan::Italic(t) => t,
                ReaderSpan::BoldItalic(t) => t,
            };
            !text.trim().is_empty()
        });
        if has_content {
            blocks.push(ReaderBlock::Paragraph(current_paragraph.clone()));
        }
        current_paragraph.clear();
    }
}

fn unescape_html(html: &str) -> String {
    let mut cleaned = String::new();
    let mut in_entity = false;
    let mut entity = String::new();

    for c in html.chars() {
        if c == '&' {
            in_entity = true;
            entity.clear();
        } else if c == ';' && in_entity {
            in_entity = false;
            match entity.as_str() {
                "lt" => cleaned.push('<'),
                "gt" => cleaned.push('>'),
                "amp" => cleaned.push('&'),
                "quot" => cleaned.push('"'),
                "apos" => cleaned.push('\''),
                "nbsp" => cleaned.push(' '),
                _ => {
                    if entity.starts_with('#') {
                        let parsed = if entity.starts_with("#x") {
                            u32::from_str_radix(&entity[2..], 16)
                        } else {
                            entity[1..].parse::<u32>()
                        };
                        if let Ok(code) = parsed {
                            if let Some(unicode_char) = std::char::from_u32(code) {
                                cleaned.push(unicode_char);
                                continue;
                            }
                        }
                    }
                    cleaned.push('&');
                    cleaned.push_str(&entity);
                    cleaned.push(';');
                }
            }
        } else if in_entity {
            entity.push(c);
        } else {
            cleaned.push(c);
        }
    }
    cleaned
}

fn parse_image_src(tag: &str) -> Option<String> {
    if let Some(pos) = tag.find("src=") {
        let rest = &tag[pos + 4..];
        let quote = rest.chars().next()?;
        if quote == '"' || quote == '\'' {
            let end = rest[1..].find(quote)?;
            Some(rest[1..end + 1].to_string())
        } else {
            let end = rest.find(|c: char| c.is_whitespace() || c == '/' || c == '>');
            let end_idx = end.unwrap_or(rest.len());
            Some(rest[..end_idx].to_string())
        }
    } else {
        None
    }
}
