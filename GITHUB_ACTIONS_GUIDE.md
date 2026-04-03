# GitHub Actions - Mobile CI/CD Setup

This document explains how to use GitHub Actions to automatically build your Book App for Android and iOS.

## Overview

With GitHub Actions, you can:
- ✅ Build Android APKs on Linux runners (Ubuntu)
- ✅ Build iOS apps on macOS runners (Apple Silicon & Intel)
- ✅ Automatically trigger builds on push or pull request
- ✅ Download compiled binaries as artifacts
- ✅ Stay on Windows and let cloud runners handle mobile builds

## Workflows

### Android Build (`.github/workflows/android-build.yml`)

**Targets:**
- ARM64 (aarch64-linux-android) - Most common
- ARM 32-bit (armv7-linux-android)
- x86_64 (x86_64-linux-android) - Emulator

**Runs on:** Ubuntu Linux (free)

**Triggers:**
- Push to `main` or `develop` branches
- Pull requests to `main` or `develop`
- Manual trigger via workflow_dispatch

**Outputs:**
- `.so` (shared library) files for each architecture
- Artifacts available for 30 days

### iOS Build (`.github/workflows/ios-build.yml`)

**Targets:**
- ARM64 device (aarch64-apple-ios) - Real devices
- ARM64 simulator (aarch64-apple-ios-sim) - Apple Silicon Macs

**Runs on:** macOS Latest (free GitHub runner with Xcode)

**Triggers:**
- Push to `main` or `develop` branches
- Pull requests to `main` or `develop`
- Manual trigger via workflow_dispatch

**Outputs:**
- `.a` (static library) files
- `book_egui` executable
- Artifacts available for 30 days

## How to Use

### 1. Initial Setup

No special setup needed! The workflows are already in `.github/workflows/`:
- `android-build.yml`
- `ios-build.yml`

Just push to your repository and GitHub Actions will automatically run.

### 2. Trigger a Manual Build

Go to your GitHub repository:

1. Click **Actions** tab
2. Select **Build Android APK** or **Build iOS App**
3. Click **Run workflow**
4. Select branch (main/develop)
5. Click **Run workflow** button

The build will start immediately!

### 3. Monitor Build Progress

1. Go to **Actions** tab
2. Click the workflow run
3. See real-time logs
4. Watch each build step execute

### 4. Download Artifacts

After build completes:

1. Go to **Actions** tab
2. Click the completed workflow
3. Scroll down to **Artifacts** section
4. Download the artifact you need:
   - `android-aarch64-linux-android-release` - ARM64 library
   - `android-armv7-linux-android-release` - ARM32 library
   - `android-x86_64-linux-android-release` - x86_64 library
   - `ios-aarch64-apple-ios-release` - Device binary
   - `ios-aarch64-apple-ios-sim-release` - Simulator binary

### 5. Use Build Results

**For Android:**
The `.so` files can be integrated into an Android Gradle project:

```gradle
android {
    sourceSets {
        main {
            jniLibs.srcDirs = ['src/main/jniLibs']
        }
    }
}
```

Place `.so` files in: `src/main/jniLibs/arm64-v8a/libbook_egui.so`

**For iOS:**
The binaries and libraries can be linked into an Xcode project:

1. Add `book_egui` executable to Target Build Phases
2. Link against static libraries (`.a` files)
3. Configure in Xcode build settings

## Automatic Build Triggers

### On Push
Every push to `main` or `develop` automatically triggers both workflows.

Example:
```bash
git push origin main
# Both Android and iOS builds start automatically
```

### On Pull Request
Every PR to `main` or `develop` triggers builds.

Example:
```bash
git push origin feature/new-chapter-display
# Creates PR → builds trigger automatically
```

### Manual Trigger
Manually start a build anytime via Actions tab.

## Build Configuration

### Caching
Both workflows use GitHub Actions caching for faster builds:
- Cargo registry cache
- Cargo index cache
- Build artifacts cache

Second build is ~50% faster due to caching.

### Build Parameters

**Android NDK:**
- Version: 25.2.9519653
- Automatically downloaded from official source
- Configured in `.cargo/config.toml`

**iOS SDK:**
- Provided by GitHub macOS runner
- Xcode verified at build time
- All necessary SDKs included

### Artifacts Retention

Artifacts are kept for **30 days** by default. You can manually delete them or modify in workflow:

```yaml
retention-days: 30  # Change this number
```

## Troubleshooting

### Build Fails

1. Click the failed workflow
2. Check the logs for error message
3. Common issues:
   - **Dependency not found** → Check Cargo.lock is committed
   - **NDK not installed** → Workflow handles this
   - **Xcode not found** → Using correct macOS runner
   - **Rust target missing** → Workflow installs it

### Build Took Too Long

- First build: 10-15 minutes (no cache)
- Subsequent builds: 3-5 minutes (with cache)
- Large changes may require clean rebuild

### Artifact Not Found

- Check build status (green checkmark)
- Scroll down in workflow - Artifacts section at bottom
- If missing, build may have failed silently - check logs

## Cost

GitHub Actions is **free**:
- 2,000 minutes/month for private repos
- Unlimited for public repos
- Our builds: ~5-10 minutes each
- ~15 builds/month = plenty of free quota

## Environment Variables

Available in build environment:
- `ANDROID_SDK_ROOT` - Android SDK path
- `ANDROID_NDK_HOME` - Set by Android workflow
- `CARGO_TERM_COLOR` - Colored output
- `RUST_BACKTRACE` - Debug info

Add secrets for:
- API keys
- Signing certificates
- Deployment credentials

See GitHub Secrets documentation to add them.

## Integration with CI/CD Pipeline

You can add more workflows:

### Code Quality Checks
```yaml
# .github/workflows/test.yml
- cargo fmt --check
- cargo clippy
- cargo test
```

### Release on Tag
```yaml
on:
  push:
    tags:
      - 'v*'
```

### Deploy to App Stores
```yaml
- Build on runners
- Sign APK/IPA
- Upload to Play Store/App Store
```

## Security Considerations

### Secrets
Store sensitive data as Secrets:

1. Go to **Settings** → **Secrets and variables**
2. Create new repository secret
3. Use in workflow: `${{ secrets.MY_SECRET }}`

### Code Signing
For production releases, add signing:

```yaml
- name: Sign APK
  env:
    SIGNING_KEY: ${{ secrets.SIGNING_KEY }}
    SIGNING_PASS: ${{ secrets.SIGNING_PASS }}
  run: |
    # Sign APK using secrets
```

### Permissions
Workflows have minimal required permissions:
- Read repository code
- Upload artifacts
- No access to secrets by default

## Next Steps

1. **Push code to GitHub**
   ```bash
   git add .
   git commit -m "Add GitHub Actions CI/CD"
   git push origin main
   ```

2. **Monitor first build**
   - Go to Actions tab
   - Watch workflows run
   - Download artifacts

3. **Set up local development**
   - Use release artifacts for testing
   - Integrate into native mobile apps

4. **Expand CI/CD**
   - Add testing workflows
   - Add code quality checks
   - Add automated release process

## Useful Links

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Rust Toolchain Action](https://github.com/dtolnay/rust-toolchain)
- [Upload Artifact Action](https://github.com/actions/upload-artifact)
- [Cargo Documentation](https://doc.rust-lang.org/cargo/)

## Files Modified

- `.github/workflows/android-build.yml` - Android build automation
- `.github/workflows/ios-build.yml` - iOS build automation
- `.cargo/config.toml` - Build target configurations
- `book-core/Cargo.toml` - Platform dependencies
- `book-egui/Cargo.toml` - Mobile feature flags

## Summary

GitHub Actions provides a complete CI/CD solution for your mobile builds:
- ✅ Build on push (automatic)
- ✅ Build on PR (automatic)
- ✅ Manual trigger anytime
- ✅ Free macOS runners for iOS
- ✅ Free Linux runners for Android
- ✅ Artifacts available for download
- ✅ No need for Xcode on Windows

Stay on Windows, push code, and let GitHub's runners handle the mobile builds!
