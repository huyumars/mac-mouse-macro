# 应用图标

`AppIcon.icns` 由打包脚本复制进应用，`AppIcon.png` 用于 README 预览。
修改矢量绘图源码后，在仓库根目录重新生成：

```sh
swift -module-cache-path target/swift-module-cache scripts/generate-icon.swift
iconutil -c icns target/AppIcon.iconset -o macos/Assets/AppIcon.icns
cp target/AppIcon.iconset/icon_512x512@2x.png macos/Assets/AppIcon.png
```

图标包含 16–1024 px 的标准与 Retina 尺寸。
