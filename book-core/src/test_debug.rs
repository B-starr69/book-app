// Temporary debug module - will be deleted
#[cfg(test)]
mod test_debug {
    use rquickjs::{Context, Function, Runtime, Value};

    #[test]
    fn test_js_array_serialization() {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();
        ctx.with(|ctx| {
            // Test 1: Simple array
            let val: Value = ctx.eval(r#"[1, 2, 3]"#).unwrap();
            let globals = ctx.globals();
            let json_obj: rquickjs::Object = globals.get("JSON").unwrap();
            let stringify_fn: Function = json_obj.get("stringify").unwrap();
            let json_str: String = stringify_fn.call((val,)).unwrap();
            println!("Simple array: {}", json_str);

            // Test 2: Array of objects
            let val2: Value = ctx.eval(r#"
                var arr = [];
                arr.push({id: "test", title: "hello"});
                arr;
            "#).unwrap();
            let json_str2: String = stringify_fn.call((val2,)).unwrap();
            println!("Array of objects: {}", json_str2);

            // Test 3: Simulate parseChapters with small HTML
            let val3: Value = ctx.eval(r#"
                function testParse(html) {
                    var chapters = [];
                    var liRegex = /<li><a\s+href="([^"]+)"[^>]*>([\s\S]*?)<\/a><\/li>/g;
                    var match;
                    while ((match = liRegex.exec(html)) !== null) {
                        var href = match[1];
                        var content = match[2];
                        var idMatch = /\/chapter\/([^/?#]+)/.exec(href);
                        if (!idMatch) continue;
                        chapters.push({
                            id: idMatch[1],
                            title: "test",
                            date: null
                        });
                    }
                    return chapters;
                }
                testParse('<li><a href="/book/test/chapter-1" title="Ch1"><strong class="chapter-title">Ch 1</strong></a></li><li><a href="/book/test/chapter-2" title="Ch2"><strong class="chapter-title">Ch 2</strong></a></li>');
            "#).unwrap();
            let json_str3: String = stringify_fn.call((val3,)).unwrap();
            println!("Parsed chapters: {}", json_str3);
        });
    }
}
