import AppKit
import SwiftUI

final class AppDelegate: NSObject, NSApplicationDelegate, NSWindowDelegate {
    private var window: NSWindow!
    let store = MacroStore()
    func applicationDidFinishLaunching(_ notification: Notification) {
        let menu = NSMenu()
        let applicationMenu = NSMenu()
        applicationMenu.addItem(withTitle: "退出 Mouse Macro", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
        let appItem = NSMenuItem(); appItem.submenu = applicationMenu; menu.addItem(appItem)
        let editMenu = NSMenu(title: "编辑")
        for (title, action, key) in [("剪切", "cut:", "x"), ("复制", "copy:", "c"), ("粘贴", "paste:", "v"), ("全选", "selectAll:", "a")] {
            editMenu.addItem(withTitle: title, action: Selector(action), keyEquivalent: key)
        }
        let editItem = NSMenuItem(title: "编辑", action: nil, keyEquivalent: ""); editItem.submenu = editMenu; menu.addItem(editItem)
        NSApp.mainMenu = menu
        window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 1080, height: 760), styleMask: [.titled, .closable, .miniaturizable, .resizable], backing: .buffered, defer: false)
        window.title = "Mouse Macro"; window.minSize = NSSize(width: 1050, height: 700)
        window.delegate = self; window.appearance = NSAppearance(named: .darkAqua)
        window.backgroundColor = NSColor(calibratedRed: 0.065, green: 0.075, blue: 0.09, alpha: 1)
        window.contentView = NSHostingView(rootView: MacroWorkspace(store: store))
        window.center(); window.makeKeyAndOrderFront(nil); NSApp.activate(ignoringOtherApps: true)
        store.load()
    }
    func windowDidResignKey(_ notification: Notification) { store.finishCapture() }
    func windowShouldClose(_ sender: NSWindow) -> Bool { NSApp.terminate(nil); return false }
    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        store.finishCapture()
        if store.dirty {
            let alert = NSAlert(); alert.messageText = "保存对宏的修改？"
            alert.addButton(withTitle: "保存并退出"); alert.addButton(withTitle: "取消"); alert.addButton(withTitle: "不保存")
            switch alert.runModal() {
            case .alertFirstButtonReturn: if !store.save() { return .terminateCancel }
            case .alertSecondButtonReturn: return .terminateCancel
            default: break
            }
        }
        store.stopPlayback(); return .terminateNow
    }
}

@main
struct MouseMacroApplication {
    static func main() {
        let app = NSApplication.shared
        let delegate = AppDelegate()
        app.delegate = delegate; app.setActivationPolicy(.regular)
        withExtendedLifetime(delegate) { app.run() }
    }
}
