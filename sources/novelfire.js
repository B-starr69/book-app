// NovelFire Extension
// Based on the Rust SourceConfig from defaults.rs

export const source = {
    id: "novelfire",
    name: "NovelFire",
    url: "https://novelfire.net",
    version: "1.0.0",

    // Search for books
    search: async function(query) {
        // NovelFire search API
        let url = `https://novelfire.net/ajax/searchLive?keyword=${encodeURIComponent(query)}&type=title`;
        let jsonStr = await fetch_json(url); // fetch_json returns the string now
        let response = JSON.parse(jsonStr);

        let results = [];
        if (response && response.data) {
            for (let item of response.data) {
                results.push({
                    id: item.slug,
                    title: item.title,
                    cover_url: item.image ? `https://novelfire.net/${item.image}` : null,
                    author: null, // Not provided in search
                });
            }
        }
        return results;
    },

    // Get book details
    get_details: async function(book_id) {
        let url = `https://novelfire.net/book/${book_id}`;
        let html = await fetch_html(url); // fetch_html provided by Rust

        let title = select_text(html, ".novel-title");
        let author = select_text(html, ".author span[itemprop='author']");

        let cover_url = select_attr(html, ".fixed-img .cover img", "data-src");
        if (!cover_url) {
            cover_url = select_attr(html, ".fixed-img .cover img", "src");
        }

        let summary = select_text(html, ".summary .content");
        let status = select_text(html, ".header-stats .completed, .header-stats .ongoing");

        return {
            id: book_id,
            title,
            author,
            cover_url,
            summary,
            status,
        };
    },

    // Get chapter list
    get_chapters: async function(book_id) {
        let url = `https://novelfire.net/book/${book_id}/chapters`; // Usually chapters are on a separate tab/page
        let html = await fetch_html(url);

        let chapter_elements = select_elements(html, "ul.chapter-list li a"); // select_elements provided by Rust

        let chapters = [];
        for (let el of chapter_elements) {
            let href = el.attr("href");
            if (href) {
                // Extract chapter ID from the URL using regex
                let match = href.match(/\/book\/[^\/]+\/([^/?#]+)/);
                if (match) {
                    chapters.push({
                        id: match[1],
                        title: el.text(),
                        url: href
                    });
                }
            }
        }
        return chapters;
    },

    // Get chapter content
    get_chapter_content: async function(book_id, chapter_id) {
        let url = `https://novelfire.net/book/${book_id}/${chapter_id}`;
        let html = await fetch_html(url);

        let title = select_text(html, ".chapter-title");
        let content_html = select_html(html, "#content"); // Gets the inner HTML of the content div

        return {
            id: chapter_id,
            title: title,
            content: content_html
        };
    }
};
