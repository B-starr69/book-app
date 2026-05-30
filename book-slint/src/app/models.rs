use crate::{BookData, ChapterData, SectionData, SearchResultData};
use book_core::{Book, Chapter, HomeSection, SearchResult};
use slint::{Image, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, VecModel};
use super::cover_registry;

// OPTIMIZATION: Avoid converting the entire image data if we can avoid it.
// Ensure your dependencies pass a pre-allocated buffer where possible.
pub fn bytes_to_image(data: &[u8]) -> Option<Image> {
    let img = image::load_from_memory(data).ok()?;
    let rgba = img.to_rgba8();

    // Slint's SharedPixelBuffer handles the cloning internals efficiently,
    // but moving the raw vec minimizes copies.
    let (width, height) = (rgba.width(), rgba.height());
    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(rgba.as_raw(), width, height);
    Some(Image::from_rgba8(buffer))
}

/// Create a Slint `Image` from raw RGBA8 bytes produced off the UI thread.
pub fn rgba_to_image(rgba_bytes: &[u8], width: u32, height: u32) -> Option<Image> {
    // Safety: ensure the slice length matches the expected size
    if rgba_bytes.len() != (width as usize) * (height as usize) * 4 {
        return None;
    }
    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(rgba_bytes, width, height);
    Some(Image::from_rgba8(buffer))
}

pub fn books_to_model(books: &[Book]) -> ModelRc<BookData> {
    let items: Vec<BookData> = books.iter().map(book_to_slint).collect();
    ModelRc::new(VecModel::from(items))
}

pub fn sections_to_model(sections: &[HomeSection], source_id: &str) -> ModelRc<SectionData> {
    let items: Vec<SectionData> = sections.iter().map(|section| section_to_slint(section, source_id)).collect();
    ModelRc::new(VecModel::from(items))
}

pub fn chapters_to_model(chapters: &[Chapter]) -> ModelRc<ChapterData> {
    let items: Vec<ChapterData> = chapters.iter().map(|c| ChapterData {
        id: SharedString::from(&c.id),
        title: SharedString::from(&c.title),
        // OPTIMIZATION: Use clear conditions instead of unwrap_or mapping overhead
        date: c.date.as_ref().map_or_else(SharedString::default, SharedString::from),
        progress: c.progress,
    }).collect();
    ModelRc::new(VecModel::from(items))
}

pub fn search_results_to_model(results: &[SearchResult]) -> ModelRc<SearchResultData> {
    let items: Vec<SearchResultData> = results.iter().map(|r| SearchResultData {
        id: SharedString::from(&r.id),
        source_id: r.source_id.as_ref().map_or_else(SharedString::default, SharedString::from),
        title: SharedString::from(&r.title),
        cover_url: SharedString::from(&r.cover_url),
        source_name: r.source_name.as_ref().map_or_else(SharedString::default, SharedString::from),
        // Try to use cached RGBA from cover registry to avoid re-decoding on UI thread
        cover_image: if let Some(src_id) = r.source_id.as_ref() {
            if let Some((rgba, w, h)) = cover_registry::get(src_id, &r.id) {
                rgba_to_image(&rgba, w, h).unwrap_or(Image::default())
            } else {
                Image::default()
            }
        } else {
            Image::default()
        },
    }).collect();
    ModelRc::new(VecModel::from(items))
}

pub fn section_to_slint(section: &HomeSection, source_id: &str) -> SectionData {
    let shared_source_id = SharedString::from(source_id); // Cache allocation out of loop
    let books: Vec<BookData> = section.books.iter().map(|b| BookData {
        id: SharedString::from(&b.id),
        source_id: shared_source_id.clone(), // Clone the atomic SharedString handle, don't re-allocate
        title: SharedString::from(&b.title),
        author: SharedString::default(),
        cover_url: SharedString::from(&b.cover_url),
        progress: 0.0,
        chapters_count: 0,
        cover_image: if let Some((rgba, w, h)) = cover_registry::get(source_id, &b.id) {
            rgba_to_image(&rgba, w, h).unwrap_or(Image::default())
        } else {
            Image::default()
        },
        in_library: false,
    }).collect();

    SectionData {
        title: SharedString::from(&section.title),
        books: ModelRc::new(VecModel::from(books)),
    }
}

pub fn book_to_slint(book: &Book) -> BookData {
    // OPTIMIZATION: loop optimization using standard iterators
    let progress = if !book.chapters.is_empty() {
        let read = book.chapters.iter().filter(|c| c.progress > 0.5).count();
        read as f32 / book.chapters.len() as f32
    } else {
        0.0
    };

    BookData {
        id: SharedString::from(&book.id),
        source_id: SharedString::from(&book.source_id),
        title: SharedString::from(&book.title),
        author: SharedString::from(&book.author),
        cover_url: SharedString::from(&book.cover_url),
        progress,
        chapters_count: book.chapters_count,
            cover_image: Image::default(),
        in_library: book.in_library,
    }
}