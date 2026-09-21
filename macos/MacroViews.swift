import SwiftUI

// Disambiguate the property wrapper from newer SDKs' State macro.
private typealias ViewState<Value> = SwiftUI.State<Value>

private let accent = Color(red: 0.32, green: 0.86, blue: 0.91)
private let canvasColor = Color(red: 0.065, green: 0.075, blue: 0.09)
private let panel = Color(red: 0.105, green: 0.12, blue: 0.14)
private let subdued = Color(red: 0.55, green: 0.60, blue: 0.66)

struct ActionStyle: ButtonStyle {
    var primary = false
    func makeBody(configuration: Configuration) -> some View {
        configuration.label.font(.system(size: 12, weight: .semibold))
            .padding(.horizontal, 15).padding(.vertical, 10)
            .foregroundColor(primary ? canvasColor : .white)
            .background(primary ? accent : Color.white.opacity(configuration.isPressed ? 0.15 : 0.07))
            .cornerRadius(7).opacity(configuration.isPressed ? 0.7 : 1)
    }
}
struct Milliseconds: View {
    @Binding var value: Int
    var body: some View {
        HStack(spacing: 5) {
            TextField("", text: Binding(get: { String(value) }, set: { raw in
                if let n = Int(raw), (0...600000).contains(n) { value = n }
            }))
            .textFieldStyle(.plain).multilineTextAlignment(.trailing)
            .font(.system(size: 12, design: .monospaced)).frame(width: 55)
            Text("ms").font(.system(size: 10)).foregroundColor(subdued)
        }.padding(.horizontal, 8).padding(.vertical, 7).background(Color.black.opacity(0.22)).cornerRadius(5)
    }
}
struct MacroWorkspace: View {
    @ObservedObject var store: MacroStore
    @ViewState private var confirmDelete = false
    @ViewState private var confirmReload = false
    @ViewState private var showDefaults = false
    var body: some View {
        VStack(spacing: 0) {
            header
            Rectangle().fill(Color.white.opacity(0.07)).frame(height: 1)
            HStack(spacing: 0) {
                sidebar.frame(width: 214)
                Rectangle().fill(Color.white.opacity(0.07)).frame(width: 1)
                if let macro = store.selected {
                    device(macro).frame(width: 300)
                    Rectangle().fill(Color.white.opacity(0.07)).frame(width: 1)
                    editor(macro).frame(maxWidth: .infinity, maxHeight: .infinity)
                } else {
                    VStack(spacing: 18) {
                        Image(systemName: "computermouse").font(.system(size: 58, weight: .ultraLight)).foregroundColor(accent)
                        Text("为你的鼠标添加第一个宏").font(.system(size: 23, weight: .medium))
                        Text("选择按钮，录制按键，让重复操作自动完成。").foregroundColor(subdued)
                        Button("新建宏") { store.addMacro() }.buttonStyle(ActionStyle(primary: true)).disabled(!store.loaded)
                    }.frame(maxWidth: .infinity, maxHeight: .infinity)
                }
            }.frame(maxWidth: .infinity, maxHeight: .infinity)
            footer
        }.background(canvasColor).foregroundColor(.white).preferredColorScheme(.dark)
        .sheet(isPresented: $store.showPermissions) { PermissionPanel(store: store) }
        .alert("无法完成操作", isPresented: Binding(get: { store.error != nil }, set: { if !$0 { store.error = nil } })) {
            Button("知道了") { store.error = nil }
        } message: { Text(store.error ?? "") }
        .alert("删除当前宏？", isPresented: $confirmDelete) {
            Button("取消", role: .cancel) { }
            Button("删除", role: .destructive) { store.deleteMacro() }
        } message: { Text("保存后将移除此宏及其鼠标绑定。") }
        .alert("重新加载已保存的宏？", isPresented: $confirmReload) {
            Button("取消", role: .cancel) { }
            Button("重新加载", role: .destructive) { store.load() }
        } message: { Text("当前未保存的修改将被放弃。") }
    }
    private var header: some View {
        HStack(spacing: 12) {
            Image(systemName: "computermouse.fill").font(.system(size: 22)).foregroundColor(accent)
            Text("MOUSE MACRO").font(.system(size: 15, weight: .bold, design: .rounded)).tracking(2)
            Text("/  按钮分配").font(.system(size: 12)).foregroundColor(subdued).padding(.leading, 15)
            Spacer()
            Circle().fill(store.running ? accent : subdued).frame(width: 6, height: 6)
            Text(store.running ? "已启用" : "未启用").font(.system(size: 11)).foregroundColor(subdued)
            Button(store.running ? "停止宏" : "启用宏") { store.togglePlayback() }
                .buttonStyle(ActionStyle(primary: true)).disabled(!store.loaded || store.isCapturing)
        }.padding(.horizontal, 25).frame(height: 72)
    }
    private var sidebar: some View {
        VStack(alignment: .leading, spacing: 18) {
            HStack {
                Text("我的宏").font(.system(size: 13, weight: .semibold))
                Spacer()
                Text("\(store.document.bindings.count)").font(.system(size: 11, design: .monospaced)).foregroundColor(subdued)
            }.padding(.horizontal, 20).padding(.top, 24)
            Button { store.addMacro() } label: { Label("新建宏", systemImage: "plus").frame(maxWidth: .infinity) }
                .buttonStyle(ActionStyle()).padding(.horizontal, 14).disabled(!store.loaded || store.isCapturing)
            ScrollView {
                LazyVStack(spacing: 6) {
                    ForEach(store.document.bindings) { macro in
                        Button { store.choose(macro.id) } label: {
                            HStack(spacing: 10) {
                                Image(systemName: "repeat").font(.system(size: 15)).foregroundColor(macro.id == store.selectedID ? accent : subdued)
                                VStack(alignment: .leading, spacing: 6) {
                                    Text(macro.title).font(.system(size: 12, weight: .semibold)).lineLimit(1)
                                    Text("\(ButtonNames.title(macro.mouseButton)) · \(macro.keys.count) 个事件").font(.system(size: 10)).foregroundColor(subdued)
                                }
                                Spacer(minLength: 0)
                            }.padding(13).frame(maxWidth: .infinity, alignment: .leading)
                                .background(macro.id == store.selectedID ? accent.opacity(0.10) : Color.clear)
                                .cornerRadius(7)
                                .overlay(RoundedRectangle(cornerRadius: 7).stroke(macro.id == store.selectedID ? accent.opacity(0.3) : Color.clear))
                        }.buttonStyle(.plain).disabled(store.isCapturing)
                    }
                }.padding(.horizontal, 10)
            }
            VStack(alignment: .leading, spacing: 16) {
                Button { showDefaults.toggle() } label: { Label("默认时间", systemImage: "slider.horizontal.3") }
                    .popover(isPresented: $showDefaults) {
                        VStack(alignment: .leading, spacing: 16) {
                            Text("新按键的默认时间").font(.headline)
                            HStack { Text("按住时长"); Spacer(); Milliseconds(value: Binding(get: { store.document.pressDurationMS }, set: { store.document.pressDurationMS = $0; store.dirty = true })) }
                            HStack { Text("按键间隔"); Spacer(); Milliseconds(value: Binding(get: { store.document.intervalMS }, set: { store.document.intervalMS = $0; store.dirty = true })) }
                            Text("已单独设置的按键时间不受影响。").font(.caption).foregroundColor(subdued)
                        }.padding(22).frame(width: 285).preferredColorScheme(.dark)
                    }
                Button { store.openPermissions() } label: { Label("系统权限", systemImage: "lock.shield") }
                Button { if store.dirty { confirmReload = true } else { store.load() } } label: { Label("重新加载", systemImage: "arrow.clockwise") }
            }.font(.system(size: 11)).foregroundColor(subdued).buttonStyle(.plain).padding(20).disabled(store.isCapturing)
        }.background(Color.white.opacity(0.015))
    }
    private func device(_ macro: MacroBinding) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("01  /  选择触发按钮").font(.system(size: 10, weight: .semibold)).tracking(1.5).foregroundColor(subdued)
            HStack(alignment: .firstTextBaseline) {
                Text(ButtonNames.title(macro.mouseButton)).font(.system(size: 24, weight: .semibold))
                Spacer()
                Image(systemName: "link").foregroundColor(accent)
            }.padding(.top, 14)
            Text("点击鼠标示意图上的按钮进行绑定").font(.system(size: 10)).foregroundColor(subdued).padding(.top, 9)
            Spacer(minLength: 12)
            MouseDiagram(selected: macro.mouseButton, occupied: Set(store.document.bindings.filter { $0.id != macro.id }.map { $0.mouseButton }), assign: store.assign)
                .frame(width: 250, height: 310).disabled(store.isCapturing)
            Spacer(minLength: 12)
            VStack(alignment: .leading, spacing: 13) {
                HStack {
                    Circle().fill(accent).frame(width: 5, height: 5)
                    Text("\(ButtonNames.title(macro.mouseButton)) → \(macro.title)").font(.system(size: 11)).lineLimit(2)
                }
                Button { store.detectButton() } label: { Label("识别实际鼠标按钮", systemImage: "cursorarrow.click").frame(maxWidth: .infinity) }
                    .buttonStyle(ActionStyle()).disabled(store.isCapturing)
                Text(store.detecting ? "在窗口空白处按下目标按钮\n按 Esc 取消" : "侧键位置仅为示意，使用“识别”匹配你的鼠标。")
                    .font(.system(size: 10)).foregroundColor(store.detecting ? accent : subdued).lineSpacing(4)
            }.padding(15).background(panel).cornerRadius(9)
        }.padding(25).frame(maxHeight: .infinity)
    }
    private func editor(_ macro: MacroBinding) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text("02  /  编辑宏").font(.system(size: 10, weight: .semibold)).tracking(1.5).foregroundColor(subdued)
                Spacer()
                Button { confirmDelete = true } label: { Image(systemName: "trash").foregroundColor(subdued) }.buttonStyle(.plain).help("删除宏").disabled(store.isCapturing)
            }
            TextField("宏名称", text: Binding(get: { macro.name ?? macro.title }, set: { name in store.edit { $0.name = name } }))
                .textFieldStyle(.plain).font(.system(size: 25, weight: .semibold)).padding(.top, 14).disabled(store.isCapturing)
            HStack(spacing: 10) {
                Image(systemName: "repeat").foregroundColor(accent)
                VStack(alignment: .leading, spacing: 4) {
                    Text("按住时重复").font(.system(size: 12, weight: .semibold))
                    Text("松开鼠标按钮，立即停止").font(.system(size: 10)).foregroundColor(subdued)
                }
                Spacer()
                Image(systemName: "checkmark.circle.fill").foregroundColor(accent)
            }.padding(14).background(accent.opacity(0.06)).cornerRadius(8).padding(.top, 18)
            HStack {
                Text("按键序列").font(.system(size: 12, weight: .semibold))
                Text("\(macro.keys.count) 个事件 · \(store.totalDuration) ms").font(.system(size: 10, design: .monospaced)).foregroundColor(subdued)
                Spacer()
            }.padding(.top, 23).padding(.bottom, 13)
            ScrollViewReader { reader in
                ScrollView {
                    LazyVStack(spacing: 8) {
                        if macro.keys.isEmpty {
                            VStack(spacing: 12) {
                                Image(systemName: "keyboard").font(.system(size: 32, weight: .light)).foregroundColor(subdued)
                                Text("从一个按键开始").font(.system(size: 13, weight: .medium))
                                Text("录制完整操作，或手动添加按键").font(.system(size: 10)).foregroundColor(subdued)
                            }.frame(maxWidth: .infinity).padding(.vertical, 45)
                        }
                        ForEach(Array(macro.keys.enumerated()), id: \.element.id) { index, step in
                            StepRow(store: store, macro: macro, step: step, index: index).id(step.id).disabled(store.isCapturing)
                        }
                        Color.clear.frame(height: 1).id("end")
                    }
                }.onChange(of: macro.keys.count) { _ in if store.recording { reader.scrollTo("end", anchor: .bottom) } }
            }.frame(maxHeight: .infinity)
            VStack(alignment: .leading, spacing: 8) {
                HStack(spacing: 8) {
                    Text("录制间隔").font(.system(size: 10)).foregroundColor(subdued)
                    Milliseconds(value: $store.recordingIntervalMS)
                    Text("按下时间").font(.system(size: 10)).foregroundColor(subdued)
                    Milliseconds(value: $store.recordingPressMS)
                    Spacer(minLength: 0)
                }.disabled(store.useActualRecordingTime)
                Toggle("使用实际录制时间", isOn: $store.useActualRecordingTime)
                    .font(.system(size: 10)).toggleStyle(.checkbox)
            }.padding(.top, 12).disabled(store.isCapturing)
            HStack(spacing: 8) {
                Button { if store.recording { store.finishCapture() } else { store.startRecording() } } label: {
                    Label(store.recording ? "结束录制" : "录制按键", systemImage: store.recording ? "stop.fill" : "record.circle")
                }.buttonStyle(ActionStyle(primary: store.recording)).disabled(store.detecting)
                Button { store.addStep() } label: { Label("添加按键", systemImage: "plus") }.buttonStyle(ActionStyle()).disabled(store.isCapturing)
                Spacer(minLength: 0)
            }.padding(.top, 12)
            Text(store.recording ? "● 正在录制此窗口的按键 · 新事件将追加到末尾" : "录制仅在此窗口进行；组合键会保留按下与松开。")
                .font(.system(size: 10)).foregroundColor(store.recording ? accent : subdued).padding(.top, 10)
            HStack {
                Text("默认间隔").font(.system(size: 11)).foregroundColor(subdued)
                Spacer()
                Milliseconds(value: Binding(get: { macro.intervalMS ?? store.document.intervalMS }, set: { value in store.edit { $0.intervalMS = value } }))
            }.padding(.top, 18).disabled(store.isCapturing).help("用于未单独设置的按键间隔，以及录制序列的循环间隔")
        }.padding(25)
    }
    private var footer: some View {
        HStack(spacing: 10) {
            Image(systemName: store.recording ? "record.circle" : "info.circle").foregroundColor(store.recording ? accent : subdued)
            Text(store.message).font(.system(size: 10)).foregroundColor(subdued).lineLimit(2)
            Spacer(minLength: 10)
            if store.dirty { Text("未保存").font(.system(size: 10)).foregroundColor(accent) }
            Button("保存更改") { store.save() }.buttonStyle(ActionStyle(primary: true)).disabled(!store.loaded || store.isCapturing)
        }.padding(.horizontal, 24).frame(height: 67).background(panel)
    }
}

private struct StepRow: View {
    @ObservedObject var store: MacroStore
    let macro: MacroBinding
    let step: MacroStep
    let index: Int
    var body: some View {
        VStack(spacing: 10) {
            HStack(spacing: 9) {
                Text(String(format: "%02d", index + 1)).font(.system(size: 10, design: .monospaced)).foregroundColor(subdued).frame(width: 22)
                Menu {
                    ForEach(Array(KeyNames.catalog.enumerated()), id: \.offset) { _, item in
                        Button(item.title) { store.updateStep(step.id) { $0.key = item.key } }
                    }
                } label: {
                    Text(KeyNames.title(step.key)).font(.system(size: 12, weight: .semibold)).lineLimit(1)
                        .padding(.horizontal, 9).padding(.vertical, 6).background(Color.white.opacity(0.09)).cornerRadius(5)
                }.menuStyle(.borderlessButton).fixedSize()
                Picker("动作", selection: Binding(get: { step.action ?? "tap" }, set: { action in store.updateStep(step.id) { $0.action = action == "tap" ? nil : action; if action != "tap" && $0.waitMS == nil { $0.waitMS = 0 } } })) {
                    Text("按下并松开").tag("tap"); Text("↓ 按下").tag("down"); Text("↑ 松开").tag("up")
                }.labelsHidden().frame(width: 112).controlSize(.small)
                Spacer(minLength: 0)
                Menu {
                    Button("上移") { store.moveStep(step.id, by: -1) }.disabled(index == 0)
                    Button("下移") { store.moveStep(step.id, by: 1) }.disabled(index + 1 == macro.keys.count)
                    Divider()
                    Button("删除事件") { store.removeStep(step.id) }
                } label: { Image(systemName: "ellipsis").foregroundColor(subdued) }.menuStyle(.borderlessButton).frame(width: 20)
            }
            HStack(spacing: 7) {
                if step.action == nil {
                    Text("按住").font(.system(size: 10)).foregroundColor(subdued)
                    Milliseconds(value: Binding(get: { step.pressMS ?? macro.pressDurationMS ?? store.document.pressDurationMS }, set: { value in store.updateStep(step.id) { $0.pressMS = value } }))
                    Text("之后等待").font(.system(size: 10)).foregroundColor(subdued)
                    Milliseconds(value: Binding(get: { step.delayMS ?? macro.intervalMS ?? store.document.intervalMS }, set: { value in store.updateStep(step.id) { $0.delayMS = value } }))
                } else {
                    Image(systemName: "clock").font(.system(size: 10)).foregroundColor(subdued)
                    Text("执行前等待").font(.system(size: 10)).foregroundColor(subdued)
                    Milliseconds(value: Binding(get: { step.waitMS ?? 0 }, set: { value in store.updateStep(step.id) { $0.waitMS = value } }))
                }
                Spacer(minLength: 0)
            }.padding(.leading, 31)
        }.padding(12).background(panel).cornerRadius(7)
    }
}

private struct MouseDiagram: View {
    let selected: String
    let occupied: Set<String>
    let assign: (String) -> Void
    private func region(_ id: String, number: String, width: CGFloat, height: CGFloat) -> some View {
        Button { assign(id) } label: {
            ZStack {
                RoundedRectangle(cornerRadius: id == "Middle" ? 10 : 17)
                    .fill(selected == id ? accent.opacity(0.23) : Color.white.opacity(0.035))
                RoundedRectangle(cornerRadius: id == "Middle" ? 10 : 17)
                    .stroke(selected == id ? accent : Color.white.opacity(0.17), lineWidth: selected == id ? 1.5 : 1)
                Text(number).font(.system(size: 11, weight: .medium, design: .monospaced)).foregroundColor(selected == id ? accent : subdued)
                if occupied.contains(id) { Circle().fill(subdued).frame(width: 4, height: 4).offset(y: height / 2 - 12) }
            }.frame(width: width, height: height)
        }.buttonStyle(.plain).help(ButtonNames.title(id)).accessibilityLabel(ButtonNames.title(id))
    }
    var body: some View {
        ZStack {
            Ellipse().fill(accent.opacity(0.045)).frame(width: 225, height: 225).blur(radius: 25).offset(y: 24)
            RoundedRectangle(cornerRadius: 84).fill(LinearGradient(colors: [Color(red: 0.17, green: 0.19, blue: 0.22), canvasColor], startPoint: .topLeading, endPoint: .bottomTrailing))
                .overlay(RoundedRectangle(cornerRadius: 84).stroke(Color.white.opacity(0.14), lineWidth: 1))
                .frame(width: 172, height: 276).offset(x: 13, y: 7)
                .shadow(color: .black.opacity(0.5), radius: 18, x: 8, y: 18)
            region("Left", number: "1", width: 60, height: 93).offset(x: -24, y: -67)
            region("Right", number: "2", width: 60, height: 93).offset(x: 50, y: -67)
            region("Middle", number: "3", width: 21, height: 52).offset(x: 13, y: -76)
            region("3", number: "4", width: 23, height: 48).offset(x: -82, y: -15)
            region("4", number: "5", width: 23, height: 48).offset(x: -82, y: 41)
            VStack(spacing: 9) {
                Image(systemName: "bolt.horizontal.fill").font(.system(size: 22)).foregroundColor(accent.opacity(0.7))
                Text("MM").font(.system(size: 9, weight: .bold)).tracking(3).foregroundColor(subdued.opacity(0.6))
            }.offset(x: 13, y: 73)
        }
    }
}


private struct PermissionPanel: View {
    @ObservedObject var store: MacroStore
    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            Label("允许 Mouse Macro 控制输入", systemImage: "lock.shield")
                .font(.system(size: 20, weight: .semibold))
            Text("macOS 需要你亲自确认授权。下面的按钮会调用系统权限请求，并可直接前往对应设置页。")
                .font(.system(size: 12)).foregroundColor(subdued)
            permissionRow("辅助功能", detail: "允许发送模拟按键", kind: "keyboard", granted: store.permissionStatus?.keyboard)
            permissionRow("输入监控", detail: "允许监听全局鼠标按钮", kind: "input", granted: store.permissionStatus?.input_monitoring)
            Text(store.permissionMessage).font(.system(size: 11)).foregroundColor(accent)
            Divider()
            Text("如果列表里没有应用：打开设置，点击 ＋ 添加当前 Mouse Macro，也可从 Finder 将它拖入列表。若旧条目已开启但仍失败，请移除旧条目后重新添加当前版本。")
                .font(.system(size: 11)).foregroundColor(subdued)
            Button("在 Finder 中显示当前应用") { store.revealCurrentApplication() }.buttonStyle(ActionStyle())
            HStack {
                Button("重新检测") { store.refreshPermissions() }.buttonStyle(ActionStyle()).disabled(store.permissionBusy)
                Spacer()
                Button("完成") { store.showPermissions = false }.buttonStyle(ActionStyle(primary: true))
            }
        }.padding(26).frame(width: 510).background(canvasColor).foregroundColor(.white).preferredColorScheme(.dark)
    }
    private func permissionRow(_ title: String, detail: String, kind: String, granted: Bool?) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text(title).font(.system(size: 14, weight: .semibold))
                    Text(detail).font(.system(size: 11)).foregroundColor(subdued)
                }
                Spacer()
                Text(granted == true ? "已允许" : granted == false ? "尚未允许" : "待检测")
                    .font(.system(size: 11)).foregroundColor(granted == true ? accent : subdued)
            }
            HStack {
                Button("请求系统授权") { store.requestPermission(kind) }.buttonStyle(ActionStyle(primary: granted != true))
                    .disabled(store.permissionBusy || granted == true)
                Button("打开系统设置") { store.openPermissionSettings(kind) }.buttonStyle(ActionStyle())
            }
        }.padding(14).background(panel).cornerRadius(8)
    }
}
