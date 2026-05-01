# Mobile Build Guide - Book App

## ⚡ QUICK START FOR WINDOWS USERS

**Don't have Xcode? Stay on Windows!**

Use **GitHub Actions** to build for iOS automatically:

1. **Push code to GitHub**
   ```bash
   git push origin main
   ```

2. **GitHub Actions builds automatically** (takes 5-10 minutes)

3. **Download artifacts** from Actions tab

✨ No Xcode, no NDK setup needed - just git push!

**→ See [WINDOWS_DEVELOPER_GUIDE.md](WINDOWS_DEVELOPER_GUIDE.md) for complete instructions**

**→ See [GITHUB_ACTIONS_GUIDE.md](GITHUB_ACTIONS_GUIDE.md) for workflow details**

---

## This guide explains traditional local builds

For developers on **Linux/macOS** or those who want local builds, follow below.

## Prerequisites

### For Both Platforms
- Rust 1.70+ installed with `rustup`
- Cargo build tools

### For Android
- Android NDK (Native Development Kit)
- Android SDK with API level 21+
- Gradle build system
- Android Studio (recommended)

**Setup Android NDK:**
```bash
# Add Android targets to Rust
rustup target add aarch64-linux-android
rustup target add armv7-linux-android
rustup target add x86_64-linux-android
rustup target add i686-linux-android

# Set NDK path (adjust version as needed)
$env:ANDROID_NDK_HOME = "C:\Users\<YourUsername>\AppData\Local\Android\Sdk\ndk\25.2.9519653"
```

### For iOS
- Xcode 14+ with Command Line Tools
- Apple Developer account
- iOS deployment target: 13.0+

**Setup iOS targets:**
```bash
# Add iOS targets to Rust
rustup target add aarch64-apple-ios
rustup target add aarch64-apple-ios-sim
rustup target add x86_64-apple-ios

# Install iOS build tools
cargo install cargo-xcode
```

## Android Build

### Method 1: Direct Cargo Build (Testing)

```bash
# Build for Android ARM64 (most common)
cargo build --target aarch64-linux-android --release

# Build for ARM (32-bit)
cargo build --target armv7-linux-android --release

# Build for x86_64 (emulator)
cargo build --target x86_64-linux-android --release
```

### Method 2: Using Android Gradle Project (Recommended)

Create Android project structure:
```
android/
├── app/
│   ├── src/
│   │   ├── main/
│   │   │   ├── AndroidManifest.xml
│   │   │   ├── java/com/bookapp/
│   │   │   │   └── BookAppActivity.java
│   │   │   └── res/
│   │   └── AndroidManifest.xml
│   └── build.gradle
├── build.gradle
└── settings.gradle
```

**build.gradle example:**
```gradle
android {
    compileSdkVersion 34

    defaultConfig {
        applicationId "com.bookapp"
        minSdkVersion 21
        targetSdkVersion 34
        versionCode 1
        versionName "1.0"

        ndk {
            abiFilters 'arm64-v8a', 'armeabi-v7a', 'x86_64'
        }
    }

    externalNativeBuild {
        cmake {
            path "CMakeLists.txt"
        }
    }
}
```

**AndroidManifest.xml:**
```xml
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="com.bookapp">

    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" />
    <uses-permission android:name="android.permission.READ_EXTERNAL_STORAGE" />
    <uses-permission android:name="android.permission.WRITE_EXTERNAL_STORAGE" />

    <application
        android:label="@string/app_name"
        android:theme="@style/Theme.AppCompat.NoActionBar">

        <activity android:name=".BookAppActivity"
            android:exported="true">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
```

**BookAppActivity.java:**
```java
package com.bookapp;

import android.app.Activity;
import android.os.Bundle;
import android.view.WindowManager;

public class BookAppActivity extends Activity {

    static {
        System.loadLibrary("book_slint");
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        // Set fullscreen and hide system UI
        getWindow().setFlags(
            WindowManager.LayoutParams.FLAG_FULLSCREEN,
            WindowManager.LayoutParams.FLAG_FULLSCREEN
        );
        getWindow().addFlags(
            WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN |
            WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS |
            WindowManager.LayoutParams.FLAG_DRAWS_SYSTEM_BAR_BACKGROUNDS
        );

        // Start the native application
        nativeStart();
    }

    private native void nativeStart();
}
```

### Deploy to Android Device

```bash
# Build APK
cd android
gradle build

# Install on connected device
adb install app/build/outputs/apk/release/app-release.apk

# Install on emulator
adb install -r app/build/outputs/apk/release/app-release.apk

# View logs
adb logcat -s "BookApp"
```

## iOS Build

### Method 1: Xcode Project Generation

```bash
# Generate Xcode project
cargo install cargo-xcode
cargo xcode

# Open in Xcode
open BookApp.xcodeproj
```

### Method 2: Manual Build

```bash
# Build for iOS device
cargo build --target aarch64-apple-ios --release

# Build for iOS simulator (Apple Silicon)
cargo build --target aarch64-apple-ios-sim --release

# Build for iOS simulator (Intel)
cargo build --target x86_64-apple-ios --release
```

### Create iOS App Bundle (Manual)

Create app structure:
```
BookApp.app/
├── Info.plist
├── BookApp (executable)
└── Assets/
```

**Info.plist:**
```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>BookApp</string>
    <key>CFBundleIdentifier</key>
    <string>com.yourcompany.bookapp</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>BookApp</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>NSMainNibFile</key>
    <string></string>
    <key>NSPrincipalClass</key>
    <string></string>
    <key>UIMainStoryboardFile</key>
    <string></string>
    <key>UIRequiredDeviceCapabilities</key>
    <array>
        <string>arm64</string>
    </array>
    <key>UISupportedInterfaceOrientations</key>
    <array>
        <string>UIInterfaceOrientationPortrait</string>
        <string>UIInterfaceOrientationLandscapeRight</string>
    </array>
    <key>NSBonjourServices</key>
    <array/>
    <key>NSLocalNetworkUsageDescription</key>
    <string>This app needs access to your local network</string>
    <key>NSBonjourServices</key>
    <array/>
    <key>NSBrowserDomainIsLocal</key>
    <true/>
</dict>
</plist>
```

### Deploy to iOS Device

```bash
# Using Xcode CLI
xcodebuild -project BookApp.xcodeproj -scheme BookApp -configuration Release

# Install on device via Xcode
xcode
# Then: Product > Run (⌘R) to test on simulator
# Or: Product > Scheme > Edit Scheme to configure device

# Using Apple command line tools
codesign -fs - BookApp.app
```

## Platform-Specific Features

### File Storage
The app automatically uses platform-appropriate directories for storing databases and cache:
- **Android**: Apps cache directory (`/data/data/com.bookapp/cache/`)
- **iOS**: Documents directory in app sandbox
- **Desktop**: Platform-specific data directory

### Permissions Required

**Android (AndroidManifest.xml):**
- `INTERNET` - Download book data and covers
- `READ_EXTERNAL_STORAGE` - Read stored books
- `WRITE_EXTERNAL_STORAGE` - Cache book data

**iOS (Info.plist):**
- Network access (automatic)
- File access (automatic for app sandbox)

### Screen Orientation
The app supports:
- Portrait (primary)
- Landscape Right
- Can be configured in manifests/Info.plist

## Troubleshooting

### Android Issues

**NDK not found:**
```bash
# Verify NDK path
echo $env:ANDROID_NDK_HOME

# Set correct path
$env:ANDROID_NDK_HOME = "C:\Users\<User>\AppData\Local\Android\Sdk\ndk\<version>"
```

**Build fails with "linker error":**
```bash
# Try adding NDK linker
cargo build --target aarch64-linux-android --release -- -Clink-arg=-fuse-ld=lld
```

**App crashes on startup:**
```bash
# Check logcat
adb logcat -s "rust" "*Crash*"
```

### iOS Issues

**Xcode symbol not found:**
```bash
# Clean and rebuild
cargo clean
cargo build --target aarch64-apple-ios --release
```

**Code signing issues:**
```bash
# Disable code signing for testing
defaults write com.apple.dt.Xcode IDESourceTreeDisplayNames -dict-add DEVELOPER_DIR /Applications/Xcode.app/Contents/Developer
```

## Performance Optimization Tips

1. **Use release builds** for deployment (slower compile, faster runtime)
2. **Minimize cover downloads** on initial load (implemented in discover page)
3. **Cache aggressively** - database is on device storage
4. **Reduce animation complexity** on older devices
5. **Monitor network usage** - large chapter batches

## Testing on Devices

### Android Emulator
```bash
# List available emulators
emulator -list-avds

# Start emulator
emulator -avd <emulator_name>

# Run app
cargo run --target x86_64-linux-android
```

### iOS Simulator
```bash
# Open Simulator
open /Applications/Simulator.app

# Build and run
cargo run --target aarch64-apple-ios-sim
```

## Distribution

### Android Play Store
1. Sign APK with release key
2. Optimize with App Bundle
3. Create Play Console account
4. Upload AAB/APK with store listing

### iOS App Store
1. Create App ID in Apple Developer
2. Create provisioning profile
3. Archive in Xcode
4. Upload with Application Loader
5. Submit for review

## Configuration Files

The build is configured via:
- `.cargo/config.toml` - Build targets and linker settings
- `Cargo.toml` - Dependencies and platform-specific features
- `book-core/src/platform.rs` - Platform abstraction layer

## Additional Resources

- [Rust on Android](https://rust-lang.github.io/rustup/cross-compilation.html)
- [Rust on iOS](https://github.com/TheMelody/rust_ios_demo)
- [Android NDK Documentation](https://developer.android.com/ndk)
- [iOS Development Guide](https://developer.apple.com/ios/)
