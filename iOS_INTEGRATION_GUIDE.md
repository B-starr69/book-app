# iOS Integration Guide

## Overview

The iOS build on GitHub Actions compiles **only the Rust backend library** (`book-core`), not the UI. This is because:

- ✅ `book-core` (pure Rust) - builds perfectly on iOS
- ❌ `book-slint` (GUI framework) - requires native iOS wrapper (Slint is UI-only, not a complete app framework)

**Your options:**

### Option 1: Native Swift App (Recommended)
Create a native iOS app in Swift that:
- Calls into the compiled Rust library for backend logic
- Implements native iOS UI (SwiftUI)
- Has full access to iOS features (biometric auth, notifications, etc.)

### Option 2: Quick Web Wrapper
Create a simple UIWebView that loads a web UI served by the Rust backend.

---

## The iOS Build Process

Your GitHub Actions workflow now:

1. **Compiles `book-core`** for iOS devices and simulators
2. **Produces `.a` (static library)** files
3. **Makes available as artifact** for download

```
iOS Workflow
├── Install Rust for iOS
├── Compile book-core library
│   ├── For device (aarch64-apple-ios)
│   └── For simulator (aarch64-apple-ios-sim)
└── Upload .a files
```

---

## Creating a Native iOS App

### Step 1: Create Xcode Project

```bash
# In same directory as book-core/
xcode-select --install  # If needed
cd ..
mkdir ios-app
cd ios-app
touch Book-App.xcodeproj  # Or use Xcode UI
```

### Step 2: Configure Xcode Project

In Xcode, create new iOS Project:
- Target: iOS 13.0+
- Language: Swift
- ☑️ SwiftUI

### Step 3: Add Rust Library to Xcode

In Xcode Build Settings:

```
Search Paths
├── Header Search Paths: ../book-core/target/aarch64-apple-ios/release
└── Library Search Paths: ../book-core/target/aarch64-apple-ios/release

Linking
└── Other Linker Flags: -lbook_core
```

### Step 4: Create Rust FFI Bindings

Create [FFI](https://doc.rust-lang.org/nomicon/ffi.html) header file `book_core.h`:

```c
// book_core.h
#include <stdint.h>

typedef struct {
    const char* title;
    const char* author;
    const char* id;
} Book;

typedef struct {
    Book* books;
    uint32_t count;
} BookList;

// Rust functions exposed to Swift
extern BookList* get_books(void);
extern void free_books(BookList* list);
extern const char* get_chapter(const char* book_id, const char* chapter_id);
extern void free_string(const char* str);
```

### Step 5: Create Swift Wrapper

Create `RustBridge.swift`:

```swift
import Foundation

// Load Rust library
private let rustLib = {
    return dlopen("libbook_core.a", RTLD_LAZY)
}()

class RustBridge {
    static let shared = RustBridge()

    func getBooks() -> [BookInfo] {
        // Call Rust function via FFI
        // let books = get_books()
        // Parse and return
        return []
    }

    func getChapter(bookId: String, chapterId: String) -> String {
        // Call Rust function
        // let content = get_chapter(bookId, chapterId)
        // return String(cString: content)
        return ""
    }
}

struct BookInfo {
    let id: String
    let title: String
    let author: String
}
```

### Step 6: Build UI in SwiftUI

```swift
import SwiftUI

struct ContentView: View {
    @State private var books: [BookInfo] = []

    var body: some View {
        NavigationStack {
            List(books, id: \.id) { book in
                NavigationLink(destination: BookDetailView(book: book)) {
                    VStack(alignment: .leading) {
                        Text(book.title)
                            .font(.headline)
                        Text(book.author)
                            .font(.subheadline)
                            .foregroundColor(.gray)
                    }
                }
            }
            .onAppear {
                books = RustBridge.shared.getBooks()
            }
            .navigationTitle("Books")
        }
    }
}

struct BookDetailView: View {
    let book: BookInfo

    var body: some View {
        ScrollView {
            VStack(alignment: .leading) {
                Text(book.title)
                    .font(.title)
                Text(book.author)
                    .font(.subheadline)
                    .foregroundColor(.gray)

                // Load chapter content
            }
            .padding()
        }
        .navigationTitle("Chapter")
    }
}
```

---

## Download & Integrate Artifacts

### 1. Get the Library

```bash
# From GitHub Actions artifacts
# Download: ios-book-core-aarch64-apple-ios-release.zip
# Contains: libbook_core.a

unzip ios-book-core-aarch64-apple-ios-release.zip
cp libbook_core.a xcode-project/Libraries/
```

### 2. Link in Xcode

In Xcode:
- **Build Phases** → **Link Binary With Libraries**
- Add `libbook_core.a`

### 3. Handle Directories

Update `book-core/src/platform.rs` for iOS data path:

```rust
#[cfg(target_os = "ios")]
{
    // iOS sandbox Documents directory
    if let Some(home) = home_directory() {
        return home.join("Documents/books.db");
    }
}
```

---

## Testing

### Simulator
```bash
# Build for simulator
cd book-core
cargo build --target aarch64-apple-ios-sim --lib --release

# In Xcode, select simulator and run
```

### Device
```bash
# Download device artifact from GitHub Actions
# Link into Xcode project
# Code sign (needs Apple Developer account)
# Run on connected iPhone/iPad
```

---

## Common Issues

### "libbook_core not found"
- Check **Library Search Paths** in Build Settings
- Verify `.a` file is in specified path
- Clean build folder (⌘⇧K)

### Symbol not found
- Ensure FFI bindings match Rust exports
- Check `extern` functions in Rust code
- Use `nm libbook_core.a` to list symbols

### Database path errors
- iOS apps are sandboxed
- Use Documents directory (handled in platform.rs)
- Don't try to write to app bundle

---

## Architecture

```
Your iOS App
├── Swift UI (SwiftUI)
├── RustBridge.swift (FFI wrapper)
└── libbook_core.a (Rust backend)
    ├── Database (SQLite)
    ├── API calls (reqwest)
    └── Book parsing
```

---

## Why Not Desktop Frameworks on iOS?

Desktop UI frameworks like Slint are designed primarily for desktop platforms. For iOS you need:

1. **App lifecycle management** - Desktop frameworks don't handle iOS lifecycle
2. **Native UI components** - iOS expects SwiftUI/UIKit
3. **System integration** - Camera, contacts, notifications, etc.
4. **App store requirements** - Apple's guidelines

**Best practice:** Use native UI with Rust backend.

---

## Benefits of This Approach

✅ Full native iOS experience
✅ Access to all iOS features
✅ Get App Store approval easily
✅ Use Swift for UI (faster iteration)
✅ Rust for high-performance backend
✅ Works great for book apps

---

## Next Steps

1. **Download library** from GitHub Actions (artifact)
2. **Create Xcode project** for iOS
3. **Create FFI bindings** between Swift and Rust
4. **Implement SwiftUI** for book browsing/reading
5. **Test on simulator** first
6. **Test on device** (requires Developer account)
7. **Submit to App Store**

---

## Resources

- [Rust FFI Documentation](https://doc.rust-lang.org/nomicon/ffi.html)
- [Swift C Interop](https://developer.apple.com/documentation/swift/imported_c_code)
- [SwiftUI Tutorials](https://developer.apple.com/tutorials/swiftui)
- [iOS App Development Guide](https://developer.apple.com/ios/)

---

## You're Not Alone!

Many Rust projects use this pattern:
- Tauri (Rust UI for desktop)
- Mozilla (Rust in Firefox)
- AWS SDKs (Rust backends)
- DuckDB (Rust engine + bindings)

It's a proven, scalable approach! 🚀
