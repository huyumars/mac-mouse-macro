#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --offline
mkdir -p target/swift-module-cache
swiftc -target "$(uname -m)-apple-macosx12.0" -module-cache-path target/swift-module-cache macos/MacroModel.swift macos/KeyNames.swift macos/Tests.swift -o target/macos-model-tests -framework AppKit
target/macos-model-tests "$PWD/target/debug/mouse-macro"
