# Windows Developer Guide - Mobile Builds via GitHub Actions

Since you're on Windows, here's how to build for Android & iOS using GitHub Actions (no Xcode needed!).

## The Setup (You Only Need GitHub)

```
Windows Dev Machine
        ↓
Git Push to GitHub
        ↓
GitHub Actions Runs
├─ Android: Builds on Ubuntu runner
└─ iOS: Builds on macOS runner
        ↓
Download Artifacts
```

## Quick Start - 3 Steps

### 1. Commit & Push Your Code

```powershell
cd c:\Users\batta\new_book_app
git add .
git commit -m "Add mobile builds"
git push origin main
```

### 2. Watch the Build

Go to your GitHub repo → **Actions** tab → Watch builds run automatically!

### 3. Download Artifacts

After build completes (~5-10 min):
- Click workflow result
- Scroll to **Artifacts** section
- Download what you need

✨ That's it!

## Manual Trigger (No Push Needed)

Want to build without pushing?

1. Go to repo → **Actions** tab
2. Click **Build Android APK** or **Build iOS App**
3. Click **Run workflow** dropdown
4. Select branch
5. Click **Run workflow** button
6. Build starts in ~30 seconds

## What You Get

### Android Artifacts
- `android-aarch64-linux-android-release` → `.so` files (ARM64)
- `android-armv7-linux-android-release` → `.so` files (ARM 32-bit)
- `android-x86_64-linux-android-release` → `.so` files (Emulator)

### iOS Artifacts
- `ios-book-core-aarch64-apple-ios-release` → Rust library (`.a` file for device)
- `ios-book-core-aarch64-apple-ios-sim-release` → Rust library (`.a` file for simulator)

**Note:** iOS builds **only the Rust backend library**. You'll create a native Swift UI to use it.

## Next: Integrate into Native Apps

### For Android (Using Gradle)

1. Download `android-aarch64-linux-android-release`
2. Create folder: `android/app/src/main/jniLibs/arm64-v8a/`
3. Place files there
4. In `build.gradle`:
   ```gradle
   android {
       externalNativeBuild {
           cmake { path "CMakeLists.txt" }
       }
   }
   ```

### For iOS (Using Native Swift)

**iOS needs a native app wrapper** - see [iOS_INTEGRATION_GUIDE.md](iOS_INTEGRATION_GUIDE.md)

Quick steps:
1. Download `ios-book-core-aarch64-apple-ios-release`
2. Create Xcode project (Swift + SwiftUI)
3. Link the `.a` library file
4. Create FFI bindings (book_core.h)
5. Implement SwiftUI UI
6. Run on simulator or device

The Xcode project deletion wasn't the issue - iOS requires a native Swift wrapper anyway!
- You can do ~200 builds/month for free

## Files Created

```
.github/workflows/
├── android-build.yml    ← Builds Android (.so libraries)
└── ios-build.yml        ← Builds iOS (binaries + libraries)
```

These workflows:
- Run automatically on push
- Run automatically on PR
- Can be manually triggered
- Cache dependencies (faster rebuilds)
- Upload artifacts for download

## Status Badges

Add to your README.md:

```markdown
[![Android Build](https://github.com/YOUR_USER/YOUR_REPO/actions/workflows/android-build.yml/badge.svg)](https://github.com/YOUR_USER/YOUR_REPO/actions)

[![iOS Build](https://github.com/YOUR_USER/YOUR_REPO/actions/workflows/ios-build.yml/badge.svg)](https://github.com/YOUR_USER/YOUR_REPO/actions)
```

## Common Workflows

### Daily Dev Cycle (Windows)

```
1. Make code changes in VS Code
2. git commit -m "Feature X"
3. git push origin main
4. Go to Actions tab
5. ☕ Get coffee while builds run
6. Download artifacts
7. Test/integrate into native app
```

### Build Before Release

```
1. Review all code
2. Tag release: git tag v1.0.0 && git push --tags
3. GitHub Actions automatically builds
4. Download release artifacts
5. Upload to app stores
```

### Test a Feature Branch

```
1. git checkout -b feature/new-ui
2. Make changes
3. git push origin feature/new-ui
4. Create PR on GitHub
5. Workflows automatically test & build
6. Review results
7. Merge when ready
```

## Troubleshooting

**Build failed?**
- Click Actions tab
- Click failed workflow
- Check error message at end of logs
- Common: Missing dependency, network issue, or platform-specific code

**Can't find artifacts?**
- Make sure build succeeded (green checkmark)
- Scroll down - artifacts at very bottom
- Keep looking, they're there!

**Build too slow?**
- First build: slow (no cache)
- Subsequent builds: fast (cached)
- Check "Finished in X seconds"

## Advanced: Customize Builds

Edit workflow files in `.github/workflows/`:

```yaml
# Add new target
- aarch64-linux-android  # Already here
- aarch64-apple-ios      # Already here
- x86_64-apple-ios       # Add if needed
```

See **GITHUB_ACTIONS_GUIDE.md** for more options.

## You're Set! 🚀

```
✅ No Xcode needed
✅ No NDK setup
✅ Just git push
✅ Artifacts download
✅ Free builds
✅ Windows compatible
```

That's the beauty of GitHub Actions - you develop on Windows, infrastructure runs on cloud, you get mobile binaries!

---

**Next:** Download your first build and integrate into native mobile apps!

See:
- `GITHUB_ACTIONS_GUIDE.md` - Full documentation
- `MOBILE_BUILD.md` - Complete build reference
- `QUICK_START.md` - Quick command reference
