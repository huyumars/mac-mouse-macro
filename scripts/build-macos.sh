#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ "$(uname -s)" != Darwin ]]; then
  echo "This script requires macOS." >&2
  exit 1
fi
cargo build --release
app="target/Mouse Macro.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
# Keep compiler caches inside the project for reproducible sandboxed builds.
mkdir -p target/swift-module-cache
swiftc -O -target "$(uname -m)-apple-macosx12.0" -module-cache-path target/swift-module-cache macos/MouseMacro.swift macos/MacroModel.swift macos/KeyNames.swift macos/MacroViews.swift -o "$app/Contents/MacOS/MouseMacro" -framework AppKit
cp target/release/mouse-macro "$app/Contents/MacOS/mouse-macro"
cp macos/Info.plist "$app/Contents/Info.plist"
cp macos/Assets/AppIcon.icns "$app/Contents/Resources/AppIcon.icns"
codesign --force --deep --sign - "$app"
echo "Built $app"
