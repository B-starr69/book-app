# 📱 Book App - Mobile Reader

A beautiful, high-performance ebook reader built with Rust and slint, designed for mobile (Android/iOS) and desktop platforms.

## Quick Links

Choose your platform:

### 🪟 **Windows Developer?**
→ [WINDOWS_DEVELOPER_GUIDE.md](WINDOWS_DEVELOPER_GUIDE.md)

Build for iOS without Xcode using GitHub Actions!
```bash
git push origin main
# Build ends automatically on GitHub
# Download artifacts from Actions tab
```

### 🔧 **GitHub Actions CI/CD**
→ [GITHUB_ACTIONS_GUIDE.md](GITHUB_ACTIONS_GUIDE.md)

Complete CI/CD documentation:
- Automatic builds on push/PR
- Manual build triggers
- Artifact downloads
- Integration guides

### 🏗️ **Traditional Local Builds**
→ [MOBILE_BUILD.md](MOBILE_BUILD.md)

Build locally on your machine:
- Android (NDK setup required)
- iOS (Xcode required)
- Direct `cargo` commands
- Android Gradle integration
- Xcode project setup

### ⚡ **Quick Reference**
→ [QUICK_START.md](QUICK_START.md)

Command reference:
- Quick Android builds
- Quick iOS builds
- Common tasks

---

## 🎯 Choose Your Path

| Your Setup | Recommended | Why |
|-----------|------------|-----|
| Windows, **no iOS plans** | [QUICK_START.md](QUICK_START.md) - Android on WSL/GitHub | Fast Android builds |
| Windows, **iOS + Android** | [WINDOWS_DEVELOPER_GUIDE.md](WINDOWS_DEVELOPER_GUIDE.md) | GitHub Actions for iOS |
| Mac/Linux | [MOBILE_BUILD.md](MOBILE_BUILD.md) | Local builds everywhere |
| CI/CD Pipeline | [GITHUB_ACTIONS_GUIDE.md](GITHUB_ACTIONS_GUIDE.md) | Automated everything |

---

## 📋 What's Included

### Source Code
- **book-core/** - Rust library (book parsing, database, API)
- **book-slint/** - UI layer (desktop/mobile)
- **Cargo.toml** - Workspace configuration

### Build Configuration
- **.cargo/config.toml** - Android/iOS target settings
- **.github/workflows/** - CI/CD workflows

### Documentation
- **WINDOWS_DEVELOPER_GUIDE.md** - Windows + GitHub Actions (RECOMMENDED)
- **GITHUB_ACTIONS_GUIDE.md** - CI/CD detailed guide
- **MOBILE_BUILD.md** - Traditional local builds
- **QUICK_START.md** - Command reference

---

## 🚀 Get Started in 3 Steps

### Step 1: Choose Your Method

- **On Windows?** → Follow [WINDOWS_DEVELOPER_GUIDE.md](WINDOWS_DEVELOPER_GUIDE.md)
- **On Mac/Linux?** → Follow [MOBILE_BUILD.md](MOBILE_BUILD.md)
- **Want CI/CD?** → Follow [GITHUB_ACTIONS_GUIDE.md](GITHUB_ACTIONS_GUIDE.md)

### Step 2: Push Code (or Trigger Manually)

```bash
git push origin main
# OR
# Go to GitHub Actions tab → Run workflow manually
```

### Step 3: Download Artifacts

- Go to GitHub repo → Actions tab
- Click completed workflow
- Download artifacts (bottom of page)

---

## 🎯 Platform Support

```
✅ Android (Full Slint app - ARM64, ARM32, x86_64)
✅ iOS (Rust library backend - device + simulator)
✅ macOS (Desktop)
✅ Linux (Desktop)
✅ Windows (Desktop)
```

**Note:** iOS uses native Swift UI with Rust backend. See [iOS_INTEGRATION_GUIDE.md](iOS_INTEGRATION_GUIDE.md)

---

## 💡 Key Features

- **Single Codebase** - Write once, run everywhere
- **Cross-Platform** - Desktop, Android, iOS
- **High Performance** - Optimized rendering, lazy loading
- **Offline Ready** - SQLite database, persistent cache
- **Network APIs** - Streams chapters from web sources
- **Platform Abstraction** - Automatic path/logging handling

---

## 🏗️ Architecture

```
Book App
├── book-core (Library)
│   ├── API handling
│   ├── Database (SQLite)
│   ├── Book parsing
│   └── Platform abstraction
│
└── book-slint (UI)
    ├── Desktop entry point
    └── Android entry point (JNI)
```

---

## 📦 Technology Stack

- **Language**: Rust
- **UI Framework**: slint
- **Database**: SQLite
- **Networking**: reqwest (async)
- **Build System**: Cargo

### Platform-Specific
- **Android**: JNI, NDK
- **iOS**: Native Swift interop
- **Desktop**: Native windowing

---

## 🔄 CI/CD Pipeline

Automated builds via GitHub Actions:

```
Your Code
   ↓
Git Push to GitHub
   ↓
├─ Android Workflow (Ubuntu)
│  ├─ ARM64 build
│  ├─ ARM32 build
│  └─ x86_64 build
│
└─ iOS Workflow (macOS)
   ├─ Device build
   └─ Simulator build
   ↓
Artifacts Ready for Download
```

**Cost**: Free (2000 min/month free tier)

---

## 📚 Documentation Files

| File | Purpose | Best For |
|-----|---------|----------|
| WINDOWS_DEVELOPER_GUIDE.md | GitHub Actions for Windows | Windows devs |
| GITHUB_ACTIONS_GUIDE.md | Complete CI/CD docs | Automation setup |
| MOBILE_BUILD.md | Local builds reference | Mac/Linux devs |
| QUICK_START.md | Command cheatsheet | Quick reference |
| QUICK_START.md | 3-line builds | Developers |

---

## 🎬 Quick Examples

### Build Android via GitHub Actions (Windows)
```bash
git push origin main
# Wait 5-10 min, download from Actions tab
```

### Build Android Locally (with NDK)
```bash
cargo build --target aarch64-linux-android --release
```

### Build iOS via GitHub Actions (Windows)
```bash
git push origin main
# macOS runner handles it
# Download from Actions tab
```

### Build iOS Locally (Mac)
```bash
cargo build --target aarch64-apple-ios --release
```

---

## ⚙️ Project Status

- ✅ Desktop builds working
- ✅ Mobile platform abstraction complete
- ✅ GitHub Actions CI/CD configured
- ✅ Cross-platform ready
- 🚀 Ready for Android/iOS deployment

---

## 🤝 Contributing

1. Clone repository
2. Follow build guide for your platform
3. Make changes
4. Push to branch
5. GitHub Actions tests build automatically
6. Submit PR

---

## 📄 License

[Your License Here]

---

## 🆘 Need Help?

1. **Windows + GitHub Actions?** → [WINDOWS_DEVELOPER_GUIDE.md](WINDOWS_DEVELOPER_GUIDE.md)
2. **Local builds?** → [MOBILE_BUILD.md](MOBILE_BUILD.md)
3. **CI/CD setup?** → [GITHUB_ACTIONS_GUIDE.md](GITHUB_ACTIONS_GUIDE.md)
4. **Quick commands?** → [QUICK_START.md](QUICK_START.md)

---

**Ready to build?** Pick your guide above and get started! 🚀
