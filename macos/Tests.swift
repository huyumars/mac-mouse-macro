import AppKit

@main
struct ModelTests {
    static func main() throws {
        _ = NSApplication.shared
        var timing = RecordingTiming()
        precondition(timing.delay(code: 0, down: true, interval: 50, press: 30) == 0)
        precondition(timing.delay(code: 0, down: false, interval: 50, press: 30) == 30)
        precondition(timing.delay(code: 11, down: true, interval: 50, press: 30) == 50)
        precondition(timing.delay(code: 11, down: false, interval: 50, press: 30) == 30)
        var chord = RecordingTiming()
        precondition(chord.delay(code: 56, down: true, interval: 50, press: 30) == 0)
        precondition(chord.delay(code: 11, down: true, interval: 50, press: 30) == 0)
        precondition(chord.delay(code: 11, down: false, interval: 50, press: 30) == 30)
        precondition(chord.delay(code: 56, down: false, interval: 50, press: 30) == 0)
        var appended = RecordingTiming(hasEvents: true)
        precondition(appended.delay(code: 0, down: true, interval: 80, press: 45) == 80)
        precondition(appended.delay(code: 0, down: false, interval: 80, press: 45) == 45)
        let syntheticShift = RecordingModifiers.reconcile(.shift, held: [])
        precondition(syntheticShift.count == 1 && syntheticShift[0].0 == 56 && syntheticShift[0].1)
        precondition(RecordingModifiers.reconcile(.shift, held: [60]).isEmpty)
        let released = RecordingModifiers.reconcile([], held: [56, 60])
        precondition(released.count == 2 && released.allSatisfy { !$0.1 })
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("mouse-macro-tests-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: dir) }
        let path = dir.appendingPathComponent("config.toml")
        let engine = URL(fileURLWithPath: CommandLine.arguments[1])
        let legacy = """
        interval_ms = 79
        press_duration_ms = 31
        mouse_button = "Left"
        keys = ["a", { key = "b", press_ms = 42, delay_ms = 91 }]
        [[bindings]]
        mouse_button = "Right"
        interval_ms = 27
        press_duration_ms = 14
        keys = ["c"]
        """
        try legacy.write(to: path, atomically: true, encoding: .utf8)
        let store = MacroStore(configURL: path, engineURL: engine)
        store.load()
        precondition(store.loaded && store.error == nil)
        precondition(store.document.bindings.count == 2)
        precondition(store.document.bindings[0].keys[1].pressMS == 42)
        precondition(store.document.bindings[1].pressDurationMS == 14)
        precondition(store.document.intervalMS == 79)
        store.edit { $0.name = "连招 \"A\"\n测试" }
        precondition(store.save())
        store.load()
        precondition(store.selected?.name == "连招 \"A\"\n测试")
        precondition(store.selected?.keys[0].pressMS == nil)
        precondition(store.selected?.keys[1].delayMS == 91)
        store.assign("Right")
        precondition(store.selected?.mouseButton == "Left" && store.error != nil)
        store.error = nil
        store.addMacro()
        precondition(store.selected?.mouseButton == "3")
        precondition(!store.save()) // Empty drafts cannot overwrite saved macros.
        store.error = nil
        store.edit { macro in
            macro.name = "组合键"
            macro.keys = [MacroStep(key: "cmd", action: "down", waitMS: 0), MacroStep(key: "a", action: "down", waitMS: 12), MacroStep(key: "a", action: "up", waitMS: 36), MacroStep(key: "cmd", action: "up", waitMS: 0)]
        }
        let id = store.selectedID!
        precondition(store.save())
        let good = try String(contentsOf: path, encoding: .utf8)
        store.removeStep(store.selected!.keys[0].id)
        precondition(!store.save())
        let preserved = try String(contentsOf: path, encoding: .utf8)
        precondition(preserved == good)
        store.error = nil; store.load()
        let restored = store.document.bindings.last!
        precondition(restored.keys.count == 4 && restored.keys[2].waitMS == 36)
        precondition(restored.id != id) // Transient UI identities never leak to disk.
        store.choose(restored.id)
        store.edit { $0.name = "外部修改检测" }
        try (good + "\n# external edit\n").write(to: path, atomically: true, encoding: .utf8)
        precondition(!store.save())
        precondition(store.dirty)
        let external = try String(contentsOf: path, encoding: .utf8)
        precondition(external == good + "\n# external edit\n")
        // Every key offered in the visual menu must be accepted by the backend.
        for key in KeyNames.catalog {
            let doc = MacroDocument(bindings: [MacroBinding(name: "Key test", mouseButton: "Left", keys: [MacroStep(key: key.key)])])
            _ = try store.backend(["--ui-encode"], input: JSONEncoder().encode(doc))
        }
        print("macOS model checks passed: legacy migration, timings, names, assignments, recording roundtrip, invalid-event protection, conflict protection, key catalog.")
    }
}
