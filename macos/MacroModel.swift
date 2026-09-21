import AppKit
import ApplicationServices
import SwiftUI

struct MacroStep: Identifiable, Codable {
    var id = UUID()
    var key: String
    var action: String? = nil
    var pressMS: Int? = nil
    var delayMS: Int? = nil
    var waitMS: Int? = nil
    enum CodingKeys: String, CodingKey { case key, action; case pressMS = "press_ms", delayMS = "delay_ms", waitMS = "wait_ms" }
    init(key: String, action: String? = nil, pressMS: Int? = nil, delayMS: Int? = nil, waitMS: Int? = nil) {
        self.key = key; self.action = action; self.pressMS = pressMS; self.delayMS = delayMS; self.waitMS = waitMS
    }
    init(from decoder: Decoder) throws {
        if let key = try? decoder.singleValueContainer().decode(String.self) { self.key = key; return }
        let c = try decoder.container(keyedBy: CodingKeys.self)
        key = try c.decode(String.self, forKey: .key)
        action = try c.decodeIfPresent(String.self, forKey: .action)
        pressMS = try c.decodeIfPresent(Int.self, forKey: .pressMS)
        delayMS = try c.decodeIfPresent(Int.self, forKey: .delayMS)
        waitMS = try c.decodeIfPresent(Int.self, forKey: .waitMS)
    }
    func encode(to encoder: Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        try c.encode(key, forKey: .key)
        if let action = action {
            try c.encode(action, forKey: .action); try c.encode(waitMS ?? 0, forKey: .waitMS)
        } else {
            try c.encodeIfPresent(pressMS, forKey: .pressMS); try c.encodeIfPresent(delayMS, forKey: .delayMS)
        }
    }
}
struct MacroBinding: Identifiable, Codable {
    var id = UUID()
    var name: String? = nil
    var mouseButton: String
    var keys: [MacroStep]
    var intervalMS: Int? = nil
    var pressDurationMS: Int? = nil
    enum CodingKeys: String, CodingKey { case name, keys; case mouseButton = "mouse_button", intervalMS = "interval_ms", pressDurationMS = "press_duration_ms" }
    var title: String { name?.isEmpty == false ? name! : "\(ButtonNames.title(mouseButton))宏" }
    // UUIDs are view identity only; storage remains compatible with the CLI.
    init(name: String?, mouseButton: String, keys: [MacroStep], intervalMS: Int? = nil, pressDurationMS: Int? = nil) {
        self.name = name; self.mouseButton = mouseButton; self.keys = keys; self.intervalMS = intervalMS; self.pressDurationMS = pressDurationMS
    }
    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        name = try c.decodeIfPresent(String.self, forKey: .name)
        mouseButton = ButtonNames.canonical(try c.decode(String.self, forKey: .mouseButton))
        keys = try c.decode([MacroStep].self, forKey: .keys)
        intervalMS = try c.decodeIfPresent(Int.self, forKey: .intervalMS)
        pressDurationMS = try c.decodeIfPresent(Int.self, forKey: .pressDurationMS)
    }
}
struct MacroDocument: Codable {
    var intervalMS = 100
    var pressDurationMS = 30
    var bindings: [MacroBinding] = []
    enum CodingKeys: String, CodingKey { case bindings; case intervalMS = "interval_ms", pressDurationMS = "press_duration_ms" }
}
enum ButtonNames {
    static let standard = ["Left", "Right", "Middle", "3", "4"]
    static func canonical(_ raw: String) -> String {
        switch raw.trimmingCharacters(in: .whitespaces).lowercased() {
        case "left", "0": return "Left"
        case "right", "1": return "Right"
        case "middle", "2": return "Middle"
        default: return raw
        }
    }
    static func title(_ raw: String) -> String {
        switch canonical(raw) {
        case "Left": return "左键"
        case "Right": return "右键"
        case "Middle": return "滚轮中键"
        case "3": return "侧键 1"
        case "4": return "侧键 2"
        default: return "鼠标按钮 \(raw)"
        }
    }
    static func fromEvent(_ number: Int) -> String { canonical(String(number)) }
}

// Some event sources include aggregate modifier flags on key events without
// separate flagsChanged notifications. Preserve those chords as well.
enum RecordingModifiers {
    static let groups: [(UInt16, UInt16, NSEvent.ModifierFlags)] = [(56, 60, .shift), (59, 62, .control), (58, 61, .option), (55, 54, .command), (63, 63, .function)]
    static func reconcile(_ flags: NSEvent.ModifierFlags, held: Set<UInt16>) -> [(UInt16, Bool)] {
        var events: [(UInt16, Bool)] = []
        for (left, right, mask) in groups {
            let pressed = held.intersection([left, right])
            if flags.contains(mask) && pressed.isEmpty { events.append((left, true)) }
            if !flags.contains(mask) { events += pressed.sorted().map { ($0, false) } }
        }
        return events
    }
}

/// Fixed-time recording keeps chord order and overlap while removing human pauses.
struct RecordingTiming {
    var elapsed = 0
    var pressedAt: [UInt16: Int] = [:]
    var hasEvents = false
    mutating func delay(code: UInt16, down: Bool, interval: Int, press: Int) -> Int {
        let wait: Int
        if down {
            wait = hasEvents && pressedAt.isEmpty ? interval : 0
            elapsed += wait
            pressedAt[code] = elapsed
        } else {
            wait = max(0, (pressedAt.removeValue(forKey: code) ?? elapsed) + press - elapsed)
            elapsed += wait
        }
        hasEvents = true
        return wait
    }
}

struct PermissionStatus: Decodable {
    let keyboard: Bool
    let input_monitoring: Bool
}

final class MacroStore: ObservableObject {
    @Published var showPermissions = false
    @Published var permissionStatus: PermissionStatus?
    @Published var permissionBusy = false
    @Published var permissionMessage = ""
    @Published var document = MacroDocument()
    @Published var selectedID: UUID?
    @Published var dirty = false
    @Published var loaded = false
    @Published var running = false
    @Published var recording = false
    @Published var useActualRecordingTime = false
    @Published var recordingIntervalMS = UserDefaults.standard.object(forKey: "recordingIntervalMS") as? Int ?? 50 {
        didSet { UserDefaults.standard.set(recordingIntervalMS, forKey: "recordingIntervalMS") }
    }
    @Published var recordingPressMS = UserDefaults.standard.object(forKey: "recordingPressMS") as? Int ?? 30 {
        didSet { UserDefaults.standard.set(recordingPressMS, forKey: "recordingPressMS") }
    }
    private var recordingTiming = RecordingTiming()
    @Published var detecting = false
    @Published var message = "正在读取设置…"
    @Published var error: String?
    private var loadedText: String?
    private var process: Process?
    private var control: Pipe?
    private var monitor: Any?
    private var held = Set<UInt16>()
    private var lastTime: TimeInterval = 0
    private var recordingCount = 0
    private var recordingBinding: UUID?
    let configURL: URL
    let engineURL: URL
    init(configURL: URL = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent(".config/mouse-macro/config.toml"), engineURL: URL = Bundle.main.bundleURL.appendingPathComponent("Contents/MacOS/mouse-macro")) {
        self.configURL = configURL; self.engineURL = engineURL
    }
    var selected: MacroBinding? { document.bindings.first { $0.id == selectedID } }
    var isCapturing: Bool { recording || detecting }
    var totalDuration: Int {
        guard let macro = selected else { return 0 }
        return macro.keys.reduce(0) { $0 + ($1.action == nil ? ($1.pressMS ?? macro.pressDurationMS ?? document.pressDurationMS) + ($1.delayMS ?? macro.intervalMS ?? document.intervalMS) : ($1.waitMS ?? 0)) }
    }
    func edit(_ body: (inout MacroBinding) -> Void) {
        guard let index = document.bindings.firstIndex(where: { $0.id == selectedID }) else { return }
        body(&document.bindings[index]); changed()
    }
    private func changed() { dirty = true; NSApp.mainWindow?.isDocumentEdited = true }
    func choose(_ id: UUID) { finishCapture(); selectedID = id }
    func addMacro() {
        finishCapture()
        let used = Set(document.bindings.map { ButtonNames.canonical($0.mouseButton) })
        guard let button = (["3", "4", "Middle", "Right", "Left"] + (5...255).map(String.init)).first(where: { !used.contains($0) }) else { error = "所有鼠标按钮均已有绑定。"; return }
        let macro = MacroBinding(name: "新建宏 \(document.bindings.count + 1)", mouseButton: button, keys: [])
        document.bindings.append(macro); selectedID = macro.id; changed()
        message = "录制按键，或手动添加一个按键。"
    }
    func deleteMacro() {
        finishCapture(); document.bindings.removeAll { $0.id == selectedID }; selectedID = document.bindings.first?.id; changed()
    }
    func assign(_ button: String) {
        let canonical = ButtonNames.canonical(button)
        guard let number = Int(canonical) ?? ["Left": 0, "Right": 1, "Middle": 2][canonical], (0...255).contains(number) else { return }
        if let other = document.bindings.first(where: { $0.id != selectedID && ButtonNames.canonical($0.mouseButton) == canonical }) {
            error = "\(ButtonNames.title(canonical))已绑定“\(other.title)”。请先修改该宏的触发按钮。"; return
        }
        edit { $0.mouseButton = canonical }; message = "已绑定到\(ButtonNames.title(canonical))。"
    }
    func addStep() { edit { $0.keys.append(MacroStep(key: "a")) } }
    func updateStep(_ id: UUID, _ body: (inout MacroStep) -> Void) {
        edit { macro in if let i = macro.keys.firstIndex(where: { $0.id == id }) { body(&macro.keys[i]) } }
    }
    func moveStep(_ id: UUID, by delta: Int) {
        edit { macro in
            guard let i = macro.keys.firstIndex(where: { $0.id == id }), macro.keys.indices.contains(i + delta) else { return }
            macro.keys.swapAt(i, i + delta)
        }
    }
    func removeStep(_ id: UUID) { edit { $0.keys.removeAll { $0.id == id } } }
    func backend(_ arguments: [String], input: Data? = nil) throws -> Data {
        if let input = input, input.count > 4 * 1024 * 1024 { throw failure("宏内容过长，请减少按键事件。") }
        // Files avoid pipe deadlocks when a large recording is being converted.
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: dir) }
        let inURL = dir.appendingPathComponent("input"), outURL = dir.appendingPathComponent("output"), errURL = dir.appendingPathComponent("error")
        try (input ?? Data()).write(to: inURL); try Data().write(to: outURL); try Data().write(to: errURL)
        let stdin = try FileHandle(forReadingFrom: inURL), stdout = try FileHandle(forWritingTo: outURL), stderr = try FileHandle(forWritingTo: errURL)
        defer { try? stdin.close(); try? stdout.close(); try? stderr.close() }
        let p = Process(); p.executableURL = engineURL; p.arguments = arguments
        p.standardInput = stdin; p.standardOutput = stdout; p.standardError = stderr
        try p.run(); p.waitUntilExit()
        if p.terminationStatus != 0 {
            let diagnostic = String(decoding: try Data(contentsOf: errURL), as: UTF8.self)
            throw failure(friendly(diagnostic))
        }
        return try Data(contentsOf: outURL)
    }
    private func failure(_ message: String) -> NSError { NSError(domain: "MouseMacro", code: 1, userInfo: [NSLocalizedDescriptionKey: message]) }
    private func friendly(_ diagnostic: String) -> String {
        if diagnostic.contains("Duplicate mouse") { return "一个鼠标按钮只能绑定一个宏，请选择其他按钮。" }
        if diagnostic.contains("without a release") || diagnostic.contains("without down") || diagnostic.contains("Repeated key down") || diagnostic.contains("already held") { return "按下和松开事件没有正确配对，请检查顺序或重新录制。" }
        if diagnostic.contains("Unknown key") { return "存在无法识别的按键，请重新选择按键。" }
        if diagnostic.contains("Timing") { return "延时必须在 0–600000 毫秒之间。" }
        if diagnostic.contains("has no keys") { return "宏中还没有按键，请录制或添加按键后保存。" }
        return "无法读取或保存宏设置。请检查现有设置是否有效，以及应用是否有文件访问权限。"
    }
    func load() {
        finishCapture()
        do {
            let disk = FileManager.default.fileExists(atPath: configURL.path) ? try String(contentsOf: configURL, encoding: .utf8) : nil
            let input = try disk.map { Data($0.utf8) } ?? backend(["--print-config", "--config", configURL.path])
            let decoded = try JSONDecoder().decode(MacroDocument.self, from: backend(["--ui-decode"], input: input))
            document = decoded; selectedID = document.bindings.first?.id; loadedText = disk
            loaded = true; dirty = false; NSApp.mainWindow?.isDocumentEdited = false
            message = "选择一个宏，设置触发按钮和按键序列。"
        } catch { self.error = error.localizedDescription }
    }
    @discardableResult func save() -> Bool {
        finishCapture()
        guard loaded else { error = "设置尚未成功读取，请重新加载后再试。"; return false }
        if let empty = document.bindings.first(where: { $0.keys.isEmpty }) { selectedID = empty.id; error = "“\(empty.title)”还没有按键，请录制或添加按键。"; return false }
        do {
            let text = String(decoding: try backend(["--ui-encode"], input: JSONEncoder().encode(document)), as: UTF8.self)
            let current = FileManager.default.fileExists(atPath: configURL.path) ? try String(contentsOf: configURL, encoding: .utf8) : nil
            guard current == loadedText else { throw failure("设置已被其他程序修改。当前编辑仍保留，请重新加载最新设置后再编辑。") }
            try FileManager.default.createDirectory(at: configURL.deletingLastPathComponent(), withIntermediateDirectories: true)
            try text.write(to: configURL, atomically: true, encoding: .utf8)
            loadedText = text; dirty = false; NSApp.mainWindow?.isDocumentEdited = false
            message = running ? "已保存。停止后重新启用，以应用修改。" : "所有宏已保存。"
            return true
        } catch { self.error = error.localizedDescription; return false }
    }
    func openPermissions() {
        finishCapture()
        showPermissions = true
        refreshPermissions()
    }
    func refreshPermissions(request: String? = nil) {
        guard !permissionBusy else { return }
        permissionBusy = true
        permissionMessage = request == nil ? "正在检测播放进程的实际权限…" : "已向 macOS 请求权限，请完成系统提示。"
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let self = self else { return }
            do {
                let args = request.map { ["--request-permission", $0] } ?? ["--permissions"]
                let status = try JSONDecoder().decode(PermissionStatus.self, from: self.backend(args))
                DispatchQueue.main.async {
                    self.permissionStatus = status; self.permissionBusy = false
                    self.permissionMessage = status.keyboard && status.input_monitoring ? "系统权限已就绪，可以返回并启用宏。" : "授权后点击“重新检测”。系统不重复弹窗时，可直接打开设置添加当前应用。"
                }
            } catch {
                DispatchQueue.main.async {
                    self.permissionBusy = false
                    self.permissionMessage = "权限检测失败，请重新检测或打开系统设置。"
                }
            }
        }
    }
    func requestPermission(_ kind: String) {
        guard !permissionBusy else { return }
        if kind == "keyboard" {
            let options = [kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: true] as CFDictionary
            _ = AXIsProcessTrustedWithOptions(options)
        }
        refreshPermissions(request: kind)
    }
    func openPermissionSettings(_ kind: String) {
        let pane = kind == "keyboard" ? "Privacy_Accessibility" : "Privacy_ListenEvent"
        NSWorkspace.shared.open(URL(string: "x-apple.systempreferences:com.apple.preference.security?\(pane)")!)
    }
    func revealCurrentApplication() {
        NSWorkspace.shared.activateFileViewerSelecting([Bundle.main.bundleURL])
    }
    func togglePlayback() {
        if running { stopPlayback(); return }
        guard save() else { return }
        guard !document.bindings.isEmpty else { error = "先创建一个宏，再启用鼠标绑定。"; return }
        let p = Process(); p.executableURL = engineURL; p.arguments = ["--config", configURL.path, "--controlled"]
        let pipe = Pipe(), output = Pipe(); control = pipe; p.standardInput = pipe; p.standardOutput = output; p.standardError = output
        p.terminationHandler = { [weak self] ended in
            let data = output.fileHandleForReading.readDataToEndOfFile()
            DispatchQueue.main.async {
                guard let self = self, self.process === ended else { return }
                self.process = nil; self.control = nil; self.running = false
                if ended.terminationStatus != 0 {
                    let diagnostic = String(decoding: data, as: UTF8.self)
                    if diagnostic.contains("required to post keyboard events") {
                        self.showPermissions = true
                        self.requestPermission("keyboard")
                    } else if diagnostic.contains("Cannot listen") {
                        self.showPermissions = true
                        self.requestPermission("input")
                    } else if diagnostic.contains("Accessibility") {
                        self.showPermissions = true
                        self.requestPermission("keyboard")
                    } else {
                        self.error = "宏播放失败，已停止。请检查系统权限后重试。"
                    }
                }
                self.message = "宏已停止。"
            }
        }
        do { try p.run(); process = p; running = true; message = "正在启用鼠标绑定；按住对应按钮循环播放。" } catch { control = nil; self.error = error.localizedDescription }
    }
    func stopPlayback() {
        guard let p = process else { return }
        try? control?.fileHandleForWriting.close(); p.waitUntilExit()
        process = nil; control = nil; running = false; message = "已停止，宏按键已释放。"
    }
    func startRecording() {
        guard selected != nil else { return }
        finishCapture(); stopPlayback(); held = []; lastTime = 0; recordingCount = 0; recordingBinding = selectedID
        recordingTiming = RecordingTiming(hasEvents: !(selected?.keys.isEmpty ?? true))
        recording = true; message = "正在录制 · 在此窗口按下键盘，点击结束录制完成。"
        monitor = NSEvent.addLocalMonitorForEvents(matching: [.keyDown, .keyUp, .flagsChanged]) { [weak self] event in
            guard let self = self else { return event }
            if event.isARepeat { return nil }
            if event.type != .flagsChanged {
                for (code, pressed) in RecordingModifiers.reconcile(event.modifierFlags, held: self.held) {
                    self.record(code, down: pressed, timestamp: event.timestamp)
                }
            }
            let down: Bool
            if event.type == .flagsChanged {
                let flags: [UInt16: UInt] = [54: 0x10, 55: 0x08, 56: 0x02, 60: 0x04, 58: 0x20, 61: 0x40, 59: 0x01, 62: 0x2000]
                if let bit = flags[event.keyCode] { down = event.modifierFlags.rawValue & bit != 0 }
                else if event.keyCode == 63 { down = event.modifierFlags.contains(.function) }
                else if event.keyCode == 57 { down = event.modifierFlags.contains(.capsLock) }
                else { return nil }
            } else { down = event.type == .keyDown }
            self.record(event.keyCode, down: down, timestamp: event.timestamp)
            return nil
        }
    }
    private func record(_ code: UInt16, down: Bool, timestamp: TimeInterval) {
        guard recording, down != held.contains(code) else { return }
        if recordingCount >= 10000 { finishCapture(); return }
        let delay = useActualRecordingTime
            ? (lastTime == 0 ? 0 : max(0, min(600000, Int((timestamp - lastTime) * 1000))))
            : recordingTiming.delay(code: code, down: down, interval: recordingIntervalMS, press: recordingPressMS)
        recordingCount += 1; lastTime = timestamp
        if down { held.insert(code) } else { held.remove(code) }
        edit { $0.keys.append(MacroStep(key: KeyNames.fromCode(code), action: down ? "down" : "up", waitMS: delay)) }
    }
    func detectButton() {
        finishCapture(); stopPlayback(); detecting = true; message = "在窗口空白处按下要绑定的鼠标按钮。"
        monitor = NSEvent.addLocalMonitorForEvents(matching: [.leftMouseDown, .rightMouseDown, .otherMouseDown, .keyDown]) { [weak self] event in
            guard let self = self else { return event }
            if event.type == .keyDown { if event.keyCode == 53 { self.finishCapture() }; return nil }
            let button = ButtonNames.fromEvent(event.buttonNumber)
            self.finishCapture(); self.assign(button); return nil
        }
    }
    func finishCapture() {
        if let monitor = monitor { NSEvent.removeMonitor(monitor) }; monitor = nil
        if recording {
            if selectedID == recordingBinding {
                for code in held.sorted() {
                    let delay = useActualRecordingTime ? 0 : recordingTiming.delay(code: code, down: false, interval: recordingIntervalMS, press: recordingPressMS)
                    edit { $0.keys.append(MacroStep(key: KeyNames.fromCode(code), action: "up", waitMS: delay)) }
                }
            }
            message = "录制完成，已添加到当前宏。"
        } else if detecting { message = "已结束按钮识别。" }
        recording = false; detecting = false; held = []; recordingBinding = nil
    }
}
