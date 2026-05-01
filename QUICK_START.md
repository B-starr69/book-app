# Quick Start: Mobile Build Guide

## Quick Reference

### Android Build (ARM64 - Most Common)
```powershell
# Set Android NDK path
$env:ANDROID_NDK_HOME = "C:\Users\<YourUsername>\AppData\Local\Android\Sdk\ndk\<version>"

# Add target
rustup target add aarch64-linux-android

# Build
cargo build --target aarch64-linux-android --release

# Or use the build script
.\build-android.ps1 -target arm64 -release
```

### iOS Build (Device)
```bash
# Add target
rustup target add aarch64-apple-ios

# Build
cargo build --target aarch64-apple-ios --release

# Or use the build script
.\build-ios.ps1 -target device -release
```

### Desktop Build (Current Setup)
```bash
# Debug build
cargo build

# Release build
cargo build --release
```

## File Structure

```
book-app/
├── .cargo/
│   └── config.toml           # Build target configurations
├── book-core/
│   ├── src/
│   │   ├── platform.rs       # Platform abstraction layer
│   │   └── lib.rs            # Exports platform module
│   └── Cargo.toml            # Updated for mobile dependencies
├── book-slint/
│   ├── src/
│   │   ├── main.rs           # Desktop entry point
│   │   ├── main_android.rs   # Android JNI entry point
│   │   ├── main_ios.rs       # iOS entry point
│   │   ├── app.rs
│   │   ├── ui.rs
│   │   ├── state.rs
│   │   ├── logic.rs
│   │   └── cover_cache.rs
│   └── Cargo.toml            # Updated for mobile dependencies
├── Cargo.toml                # Workspace configuration
├── MOBILE_BUILD.md           # Comprehensive mobile build guide
├── QUICK_START.md            # This file
├── build-android.ps1         # Android build script
└── build-ios.ps1             # iOS build script
```

## Key Changes for Mobile

### 1. Platform Abstraction (`book-core/src/platform.rs`)
Provides unified interface for:
- File paths (different on Android/iOS/Desktop)
- Logging (platform-specific)
- UI scaling
- Mobile device detection

### 2. Dependencies Updated
- **Android**: JNI bindings for Java/Rust communication
- **All**: `dirs` crate for cross-platform directories

### 3. Build Configuration (`.cargo/config.toml`)
Defines linker and compiler settings for each target:
- `aarch64-linux-android` - ARM64 (most phones)
- `armv7-linux-android` - ARM 32-bit
- `x86_64-linux-android` - Emulator
- `aarch64-apple-ios` - iPhone/iPad
- `aarch64-apple-ios-sim` - Apple Silicon simulator
- `x86_64-apple-ios` - Intel Mac simulator

## Next Steps

1. **For Android**:
   - Install Android NDK (see `MOBILE_BUILD.md`)
   - Create Android project structure
   - Update `AndroidManifest.xml` with permissions
   - Create Gradle build configuration
   - Build with `build-android.ps1`

2. **For iOS**:
   - Install Xcode
   - Add iOS targets
   - Build with `build-ios.ps1`
   - Create Xcode project if needed
   - Code sign and deploy

3. **Testing**:
   - Use emulators for initial testing
   - Test on real devices
   - Verify database operations
   - Test network requests (books API)
   - Check cover image caching

## Important Notes

- **Database**: Uses bundled SQLite - works on both platforms
- **Networking**: Uses `reqwest` - fully supported on mobile
- **File Storage**: Automatically uses platform-appropriate directories via `book_core::platform`
- **UI**: Slint handles platform differences for desktop
- **Logging**: Different for each platform, all handled in `platform.rs`

## Common Issues

1. **NDK Not Found**: Set `ANDROID_NDK_HOME` environment variable
2. **Build Fails**: Run `cargo clean` first
3. **Target Not Installed**: Use `rustup target add <target>`
4. **Device Not Detected**: Check platform-specific debugging tools (ADB for Android, Xcode for iOS)

## Performance Tips

1. Build in `--release` mode for deployment
2. Debug builds are slower but easier to debug
3. Cover caching is automatic
4. Database is persistent on-device
5. API calls use async/await for responsive UI

## Documentation

- **MOBILE_BUILD.md**: Complete guide with detailed instructions
- **.cargo/config.toml**: Build target configurations
- **book-core/src/platform.rs**: Platform abstraction implementation

## Support

For more detailed information, see `MOBILE_BUILD.md` - it includes:
- Complete setup instructions for both platforms
- Android Gradle project templates
- iOS Xcode setup
- Deployment guides
- Troubleshooting

---

This project is now configured for cross-platform mobile deployment while maintaining the existing desktop functionality.
