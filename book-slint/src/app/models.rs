use crate::{App, BookData, ChapterData, SectionData, SearchResultData};
use book_core::{Book, Chapter, HomeSection, SearchResult};
use slint::{Image, Model, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, VecModel};

pub fn bytes_to_image(data: &[u8]) -> Option<Image> {
    let img = image::load_from_memory(data).ok()?;
    let rgba = img.to_rgba8();
    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(rgba.as_raw(), rgba.width(), rgba.height());
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
        date: SharedString::from(c.date.as_deref().unwrap_or("")),
        progress: c.progress,
    }).collect();
    ModelRc::new(VecModel::from(items))
}

pub fn search_results_to_model(results: &[SearchResult]) -> ModelRc<SearchResultData> {
    let items: Vec<SearchResultData> = results.iter().map(|r| SearchResultData {
        id: SharedString::from(&r.id),
        source_id: SharedString::from(r.source_id.as_deref().unwrap_or("")),
        title: SharedString::from(&r.title),
        cover_url: SharedString::from(&r.cover_url),
        cover_image: Image::default(),
        source_name: SharedString::from(r.source_name.as_deref().unwrap_or("")),
    }).collect();
    ModelRc::new(VecModel::from(items))
}

pub fn section_to_slint(section: &HomeSection, source_id: &str) -> SectionData {
    let books: Vec<BookData> = section.books.iter().map(|b| BookData {
        id: SharedString::from(&b.id),
        source_id: SharedString::from(source_id),
        title: SharedString::from(&b.title),
        author: SharedString::default(),
        cover_url: SharedString::from(&b.cover_url),
        cover_image: Image::default(),
        progress: 0.0,
        chapters_count: 0,
        in_library: false,
    }).collect();

    SectionData {
        title: SharedString::from(&section.title),
        books: ModelRc::new(VecModel::from(books)),
    }
}

pub fn book_to_slint(book: &Book) -> BookData {
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
        cover_image: Image::default(),
        progress,
        chapters_count: book.chapters_count,
        in_library: book.in_library,
    }
}

pub fn update_book_cover_models(ui: &App, source_id: &str, book_id: &str, image: Image) {
    let library = ui.get_library_books();
    let updated_library: Vec<BookData> = (0..library.row_count())
        .filter_map(|i| library.row_data(i))
        .map(|mut book| {
            if book.id == book_id && book.source_id == source_id {
                book.cover_image = image.clone();
            }
            book
        })
        .collect();
    ui.set_library_books(ModelRc::new(VecModel::from(updated_library)));

    let discover = ui.get_discover_sections();
    let updated_sections: Vec<SectionData> = (0..discover.row_count())
        .filter_map(|i| discover.row_data(i))
        .map(|section| {
            let books: Vec<BookData> = (0..section.books.row_count())
                .filter_map(|j| section.books.row_data(j))
                .map(|mut book| {
                    if book.id == book_id && book.source_id == source_id {
                        book.cover_image = image.clone();
                    }
                    book
                })
                .collect();

            SectionData {
                title: section.title,
                books: ModelRc::new(VecModel::from(books)),
            }
        })
        .collect();
    ui.set_discover_sections(ModelRc::new(VecModel::from(updated_sections)));

    let results = ui.get_search_results();
    let updated_results: Vec<SearchResultData> = (0..results.row_count())
        .filter_map(|i| results.row_data(i))
        .map(|mut r| {
            if r.id == book_id && r.source_id == source_id {
                r.cover_image = image.clone();
            }
            r
        })
        .collect();
    ui.set_search_results(ModelRc::new(VecModel::from(updated_results)));
}
