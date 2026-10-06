// 键盘的状态与按键语义，行为对齐 iOS 自带简体拼音：拼音以 marked text 写在宿主光标处，
// 空格上屏首选，换行原样上屏字母（打英文就靠它），组字中敲标点先上屏首选。视图只读状态、转发点击。
// 配了素笺云 时：大模型的候选异步插进候选栏（控制器定时调 poll），没在组字时可以用当前技能改写光标前的一段话
// （工具栏右边那颗按钮写着技能名，点开是技能排）、插入别的设备刚复制的文字、把本机剪贴板发出去。
// 验证码 / 密码这类输入框里这些都停（privateField）。
// 本地记忆：对象牌子、候选栏上方的提示行、对象卡、「记一笔」都经 MemoryBridge 调桥；名单读 App Group 里的 memory/。
// 名单是一张平铺的人，人数不限；切人就调桥（ScopePick），桥回话后按新的对象重读名单与提示。
// 所有输出都经 OutputRouter：手写记一笔时它把上屏、退格改道到草稿，宿主一个字都不碰（不持有宿主，就绕不过去）。

import Foundation
import Observation
import os
import QingjianBridge

@MainActor
@Observable
final class KeyboardModel {
    private static let log = Logger(subsystem: "sujian.synon.ai.keyboard", category: "memory")

    private(set) var preedit = ""

    private(set) var candidates: [CandidateItem] = []

    private(set) var rewrite = RewriteState.idle

    /// 别的设备刚复制、还没处理的文字。
    private(set) var clipOffer: ClipOffer?

    /// 本机剪贴板自上次处理后变过（有文字）：候选栏给「发到其他设备」。
    private(set) var pasteboardChanged = false

    /// 焦点在验证码、密码、信用卡号这类输入框。
    private(set) var privateField = false

    /// 记忆的提示行：恋爱或日常、选了对象、碰上卡片里的词或日子快到时有。
    private(set) var hint: MemoryHint?

    /// 当前对象（只能用户自己切）。
    private(set) var scope = MemoryScope()

    /// 名单上的人（App 或键盘里建的）；开了完全访问才读得到 App Group。
    private(set) var contacts: [MemoryContact] = []

    /// 点了牌子：工具栏里横着列其他人与「不指定」（ScopeDisplay.quickPicks）。
    private(set) var quickOpen = false

    /// 没开完全访问时点牌子展开的说明（改写与记忆都要完全访问）。
    private(set) var showsFullAccessNote = false

    /// 随包的改写技能（键盘起来、换引擎时各读一次，运行中不变）。
    private(set) var rewriteSkills: [Skill] = []

    /// 改写用的全局默认技能（设置里的 `[rewrite] skill`）；读不出来时按缺省（`polish`）。
    private(set) var defaultRewriteSkill = RewriteDefault.fallbackSkill

    /// 点了工具栏那颗技能按钮：露出 / 收起技能排（与 `showsFullAccessNote` 一样的观察状态）。
    private(set) var showsRewriteSkills = false

    /// 确认条里待记的几条素材（剪贴板拆出来的）；nil 时不出确认条。
    private(set) var noteDraft: [String]?

    /// 记一笔条上的一行短提示：「记下了，明早整理」，或没记上的原因（满了、补写被拒绝），过一会儿消失。
    private(set) var noteToast: NoteToast?

    /// 记一笔条正显示短提示（确认条换成这一行）。
    var noteDone: Bool { noteToast != nil }

    /// 草稿卡（1e-2）与冲突屏（1e-3）的状态：原话、草稿项、冲突的那一对（NoteDraftFlow）。
    private(set) var noteFlow = NoteDraftFlow()

    /// 草稿卡的几项。
    var draftFields: [DraftField] { noteFlow.fields }

    /// 草稿卡与冲突屏顶部那条原话，以及它展没展开。
    var draftSource: String { noteFlow.source }
    var draftSourceExpanded: Bool { noteFlow.sourceExpanded }

    /// 冲突屏的那一对；没冲突时为 nil。
    var conflict: NoteConflict? { noteFlow.conflict }

    /// 面板里的一行短提示（键盘扩展打不开 App，「全部记忆」「去开启」只能这样告诉用户），2 秒后消失。
    private(set) var notice: String?

    /// 所有人的卡片，按卡片 id 查（提示行加粗关键词、来源标签用）。
    @ObservationIgnored private var cardIndex: [String: MemoryCard] = [:]

    /// 对象卡面板里的卡片。
    private(set) var panelCards: [MemoryCard] = []

    /// 开了完全访问：读 App Group 里的名单、「记一笔」读剪贴板都要它。控制器每次出现时设。
    var fullAccess = false

    private(set) var layer = KeyLayer.letters

    private(set) var panel = KeyboardPanel.keys

    private(set) var shifted = false

    /// 正按着的格子（KeyboardLayout.slots 的下标），键帽据此变色、弹放大字样。
    private(set) var pressedSlots: Set<Int> = []

    /// 按住 ⌫ 连删的任务，按格子记。
    @ObservationIgnored private var repeats: [Int: Task<Void, Never>] = [:]

    /// 按住空格拖动挪光标：哪一格进了这个模式、已经挪了几步。抬起时不出空格。
    @ObservationIgnored private var cursorDrag: (slot: Int, steps: Int)?


    /// 引擎打不开（数据缺失）时为 nil，字母直接输出。
    @ObservationIgnored private var engine: Engine?

    /// 宿主（控制器）。只经 `sink` 用，不直接调。
    var output: TextOutput? {
        get { sink.host }
        set { sink.host = newValue }
    }

    @ObservationIgnored let sink = OutputRouter()

    /// 确认条里那段剪贴板文字的摘要与变化计数；记下或忽略后存成「处理过的」（NoteEntry）。
    @ObservationIgnored private var noteSource: (digest: String, changeCount: Int)?

    /// 每次按键按下时调（键盘音与震动），由控制器接上。
    @ObservationIgnored var onKeyDown: (() -> Void)?

    init(engine: Engine?) {
        self.engine = engine
    }

    /// 换一个引擎（完全访问开关或连接配置变了，学习数据目录与云端都要重开）；旧的先落盘。
    func replaceEngine(_ engine: Engine?) {
        self.engine?.flush()
        self.engine = engine
        if privateField { engine?.setPrivate(true) }
        let shown = EngineDisplay.afterReplace(hasEngine: engine != nil, preedit: preedit, candidates: candidates)
        if shown.preedit != preedit { sink.setMarked(shown.preedit) }
        preedit = shown.preedit
        candidates = shown.candidates
        refresh()
        syncScope()
        reloadRewriteSkills()
    }

    /// 键盘扩展的内存上限紧（jetsam 按 phys_footprint 杀，社区实测 48–60MB）：余量低于 8MB 就卸掉英文表腾地方，
    /// 下次像英文的输入会自动再加载。
    ///
    /// 本地整句模型（含章·通变）不在键盘里用：桥没有驱动异步重打分的调用，加载了也从不出结果（有无模型候选完全一样），
    /// 却占 65–120MB 内存（CPU 推理，f16 / f32 权重），把键盘推到上限边上。以后有了合适的接法再加回来。
    private func guardMemoryPressure() {
        guard let engine else { return }
        let available = Engine.availableMemoryMB
        if available >= 0, available < 8 {
            engine.unloadEnglish()
        }
    }

    var composing: Bool { !preedit.isEmpty }

    /// 改写按钮出不出（`ScopeDisplay.canRewrite`：技能包在、开了完全访问、有改写器、不在私密输入框）。
    /// 没开完全访问时那颗按钮不出现——那时没有网络，按下去必然得到「检查网络」；说明入口在牌子上。
    var rewriteAvailable: Bool {
        ScopeDisplay.canRewrite(
            fullAccess: fullAccess, privateField: privateField,
            hasSkills: !rewriteSkills.isEmpty, hasRewriter: engine?.rewriteAvailable ?? false)
    }

    /// 此刻生效的技能：选中的人身上指定了就用它 → 设置里的默认 → 列表第一个。
    var rewriteSkill: Skill? {
        RewriteSkill.resolve(
            skills: rewriteSkills, contactSkill: currentContact?.skill,
            defaultSkill: defaultRewriteSkill)
    }

    /// 技能排上高亮的那个：选中的人身上指定的（nil = 用默认）。没选人时永远是「用默认」。
    var rewriteSkillPick: String? { currentContact?.skill }

    /// 技能排与牌子展开的人占同一行，开一个就收起另一个。
    func toggleRewriteSkills() {
        showsRewriteSkills.toggle()
        if showsRewriteSkills { quickOpen = false }
    }

    /// 换技能：选了人写进这个人，没选人就把全局默认写进设置（经桥的 `qj_rewrite_default_set`）；记下来返回 true。
    /// 没选人时「用默认」等于保持现状——那时现在的默认就是它。
    @discardableResult
    func setRewriteSkill(_ id: String?) -> Bool {
        showsRewriteSkills = false
        if let contact = currentContact {
            guard engine?.setContactSkill(contact.id, skillId: id) == nil else { return false }
            reloadContacts()
        } else if let id {
            defaultRewriteSkill = id
            saveDefaultRewriteSkill(id)
        }
        return true
    }

    /// 技能排上选了一个（工具栏那颗按钮点开的那一排，与改写条上的那一排是同一排）：记下来，并用它改写光标前那一段。
    /// 与改写条上「点另一个就用它重改」是同一件事——所以工具栏上选技能就等于改写。
    func pickRewriteSkill(_ id: String?) {
        guard setRewriteSkill(id) else { return }
        startRewrite()
    }

    /// 全局默认技能记在设置里的 `[rewrite] skill` 里，经桥的 `qj_rewrite_default_set` 只改这一项（键盘不碰配置文件）。
    private func saveDefaultRewriteSkill(_ id: String) {
        if let failure = engine?.setRewriteDefaultSkill(id) {
            Self.log.error("默认技能没写进设置：\(failure.code.rawValue, privacy: .public)")
        }
    }

    /// 随包的技能与设置里的默认：键盘起来、换引擎时各读一次（技能包在运行中不变）。
    /// 设置里的默认读不出来时按缺省走，不提示（读不是用户刚做的动作）。
    private func reloadRewriteSkills() {
        rewriteSkills = engine?.rewriteSkills ?? []
        defaultRewriteSkill = engine?.rewriteDefaultSkill ?? RewriteDefault.fallbackSkill
    }

    func tap(_ key: Key) {
        // 又开始打字了：没用上的润色作废
        if rewrite != .idle, key != .shift, key != .globe { dismissRewrite() }
        switch key {
        case .letter(let letter): typeLetter(letter)
        case .symbol(let text): typeSymbol(text)
        case .shift: shifted.toggle()
        case .backspace: backspace()
        case .layer(let target): layer = target
        case .globe: sink.switchToNextKeyboard()
        case .emoji: panel = .emoji
        case .space: space()
        case .returnKey: returnKey()
        }
    }

    /// 按下：高亮、反馈；⌫ 按下即删，按住 0.4 秒后每 0.08 秒连删一次。别的键抬起才出字（与系统键盘一致）。
    func press(slot: Int, key: Key) {
        guard !pressedSlots.contains(slot) else { return }
        pressedSlots.insert(slot)
        onKeyDown?()
        guard key == .backspace else { return }
        tap(key)
        repeats[slot] = Task { @MainActor [weak self] in
            try? await Task.sleep(for: .milliseconds(400))
            while !Task.isCancelled {
                self?.onKeyDown?()
                self?.tap(.backspace)
                try? await Task.sleep(for: .milliseconds(80))
            }
        }
    }

    /// 抬起出字。被系统取消的触摸（屏幕边缘手势抢的）也按抬起算：用户确实按了这个键。
    func release(slot: Int, key: Key, cancelled: Bool) {
        guard pressedSlots.remove(slot) != nil else { return }
        repeats.removeValue(forKey: slot)?.cancel()
        if cursorDrag?.slot == slot {
            cursorDrag = nil
            return
        }
        if key != .backspace { tap(key) }
    }

    /// 按住空格横向拖：挪过 [`Self.cursorDragStart`] 进挪光标模式，之后每 [`Self.cursorStep`] 挪一个字，每步轻震一下。
    /// 组字中不挪（光标在拼音里没意义），照常当空格；手写记一笔时也不挪（草稿的光标只在末尾）。
    func drag(slot: Int, key: Key, dx: CGFloat) {
        guard key == .space, pressedSlots.contains(slot), !composing, !sink.isComposingNote else { return }
        if cursorDrag == nil {
            guard abs(dx) >= Self.cursorDragStart else { return }
            cursorDrag = (slot, 0)
        }
        guard var drag = cursorDrag, drag.slot == slot else { return }
        let travelled = dx - (dx > 0 ? Self.cursorDragStart : -Self.cursorDragStart)
        let steps = Int(travelled / Self.cursorStep)
        guard steps != drag.steps else { return }
        sink.moveCursor(by: steps - drag.steps)
        onKeyDown?()
        drag.steps = steps
        cursorDrag = drag
    }

    /// 横向挪多远才算拖（pt），免得点空格时手指一抖就挪了光标。
    private static let cursorDragStart: CGFloat = 12

    /// 拖多远挪一个字（pt）。
    private static let cursorStep: CGFloat = 9

    func selectCandidate(_ index: Int) {
        guard let engine, let text = engine.commit(index) else { return }
        sink.commit(text)
        refresh()
        // 候选吃完了拼音就收起展开的候选面板；还剩拼音时留着接着选
        if !composing, panel == .candidates { panel = .keys }
    }

    func toggleCandidatePanel() {
        panel = panel == .candidates ? .keys : .candidates
    }

    func closePanel() {
        panel = .keys
    }

    func typeEmoji(_ emoji: String) {
        commitFirst()
        sink.commit(emoji)
    }

    /// 工具栏右端的向下箭头：收起键盘（之后系统会走 viewWillDisappear → dismiss）。
    func dismissKeyboard() {
        sink.dismissKeyboard()
    }

    /// 键盘收起：没上屏的拼音丢掉，学习数据落盘并催一轮同步。
    func dismiss() {
        endComposedNote()
        quickOpen = false
        showsRewriteSkills = false
        dismissRewrite()
        noteDraft = nil
        noteToast = nil
        notice = nil
        engine?.clear()
        engine?.flush()
        panel = .keys
        refresh()
    }

    /// 键盘出现：先同步一轮，别的设备学到的词尽快过来；拉一次别的设备的剪贴板，看本机剪贴板变没变。
    /// 上次排队的记一笔补写时被拒绝了，在这里提示一次（桥取一次就清零）。
    func appear() {
        engine?.syncNow()
        engine?.refreshClipboard()
        checkPasteboard()
        syncScope()
        reloadRewriteSkills()
        if fullAccess, let dropped = engine?.memoryDropped() {
            let lines = NoteBarText.dropped(dropped)
            if !lines.isEmpty { showNoteToast(.problem(lines.joined(separator: "\n"))) }
        }
    }

    /// 焦点换到了（不）是验证码 / 密码这类输入框。
    func setPrivateField(_ value: Bool) {
        guard value != privateField else { return }
        privateField = value
        engine?.setPrivate(value)
        if value {
            endComposedNote()
            dismissRewrite()
            clipOffer = nil
            pasteboardChanged = false
            noteDraft = nil
        } else {
            checkPasteboard()
        }
        refreshHint()
    }

    func insertClip() {
        guard let offer = clipOffer, !sink.isComposingNote else { return }
        commitFirst()
        sink.commit(offer.text)
        engine?.clipHandled()
        clipOffer = nil
    }

    func dismissClip() {
        engine?.clipHandled()
        clipOffer = nil
    }

    /// 读本机剪贴板（可能弹系统的粘贴授权提示）发给别的设备。
    func pushPasteboard() {
        guard let engine, !privateField else { return }
        if let text = sink.readPasteboard(), !text.isEmpty {
            engine.pushClip(text)
        }
        markPasteboardSeen()
    }

    func dismissPasteboard() {
        markPasteboardSeen()
    }

    private func checkPasteboard() {
        guard let engine, engine.clipboardEnabled, output != nil, !privateField else { return }
        let count = sink.pasteboardChangeCount
        // 第一次用：装键盘之前就在剪贴板里的不算新复制的
        guard let seen = Self.seenPasteboardCount else {
            Self.seenPasteboardCount = count
            return
        }
        pasteboardChanged = count != seen && sink.pasteboardHasText
    }

    private func markPasteboardSeen() {
        if output != nil { Self.seenPasteboardCount = sink.pasteboardChangeCount }
        pasteboardChanged = false
    }

    /// 上次处理过的剪贴板变化计数，存在扩展自己的 UserDefaults 里（键盘进程随时被杀）。
    private static var seenPasteboardCount: Int? {
        get { UserDefaults.standard.object(forKey: "seenPasteboardCount") as? Int }
        set { UserDefaults.standard.set(newValue, forKey: "seenPasteboardCount") }
    }

    /// 控制器定时调：取大模型候选、合并别的设备的学习数据、看润色有没有回来。
    func poll() {
        guard let engine else { return }
        refreshHint()
        // App 删了当前对象时桥会退回不指定
        if let next = engine.scope, next != scope {
            scope = next
            reloadContacts()
        }
        if engine.poll() { candidates = engine.candidates }
        guardMemoryPressure()
        if !privateField {
            let offer = engine.clipOffer
            if offer != clipOffer { clipOffer = offer }
        }
        guard case .pending(let span, let skill) = rewrite else { return }
        switch engine.rewriteStatus {
        case 2:
            if let result = engine.takeRewrite() {
                rewrite = .ready(span: span, skill: skill, result: result)
            }
        case 3:
            rewrite = .failed
        case 4:
            rewrite = .rejected
        default:
            break
        }
    }

    /// 键盘收起时踢一脚后台上传（素材与输入日志；没开完全访问就没有上传器，白踢）。
    func uploadKick() {
        if let dir = engine?.userDirectory {
            dir.path.withCString { qj_upload_kick($0) }
        }
    }

    /// 改写用此刻生效的技能。选哪段见 [`RewriteSpan.select`]：选中优先，否则当前整句。
    func startRewrite() {
        guard let engine, let skill = rewriteSkill, !composing, !sink.isComposingNote,
              output != nil
        else { return }
        guard var span = RewriteSpan.select(
            before: sink.contextBefore,
            selection: sink.selectedText
        ) else { return }
        if span.original.count > 300 {
            span = RewriteSpan(
                original: String(span.original.suffix(300)),
                tail: span.tail,
                isSelection: span.isSelection
            )
        }
        engine.startRewrite(span.original, skillId: skill.id)
        rewrite = .pending(span: span, skill: skill.name)
    }

    /// 用改写结果替换原文。校验与还原规则都在 [`RewriteSpan`] 里；不满足就放弃，不乱删。
    func applyRewrite() {
        guard case .ready(let span, _, let result) = rewrite, output != nil else { return }
        rewrite = .idle
        guard span.applies(before: sink.contextBefore, selection: sink.selectedText) else { return }
        if span.isSelection {
            // 退格一次删掉整个选中
            sink.deleteBackward()
        } else {
            for _ in 0..<span.deleteCount { sink.deleteBackward() }
        }
        sink.commit(span.committed(result))
    }

    func dismissRewrite() {
        engine?.cancelRewrite()
        rewrite = .idle
    }

    /// 提示行这一行在不在：选了人且有提示，或有记一笔 / 起名字的输入条时才在，键盘高度跟着加减一行（ScopeDisplay.hasHintRow）。
    /// 对象卡打开时这一行不画（设计稿 1b），高度让给对象卡，键盘总高不变；草稿卡与冲突屏打开时这一行收起（设计稿 1e-2、1e-3）。
    var hasHintRow: Bool {
        ScopeDisplay.hasHintRow(
            hasContact: currentContact != nil, hasHint: hint != nil,
            hasNoteBar: noteDraft != nil || noteDone || sink.isComposingNote,
            noteCardOpen: panel == .draft || panel == .conflict)
    }

    /// 提示行这一行实际画出来的高度：没有时为 0；记一笔确认条比提示行高（KeyStyle.clipRowHeight）。
    private var liveRowHeight: CGFloat {
        guard hasHintRow else { return 0 }
        return noteDraft != nil && !sink.isComposingNote ? KeyStyle.clipRowHeight : KeyStyle.hintRowHeight
    }

    /// 打开草稿卡那一刻的行高：草稿卡、冲突屏打开期间键盘总高按它，宿主界面不跳。
    @ObservationIgnored private var heldRowHeight: CGFloat = 0

    private var rowHeights: (total: CGFloat, inset: CGFloat) {
        ScopeDisplay.rowHeights(
            live: liveRowHeight, held: heldRowHeight,
            noteCardOpen: panel == .draft || panel == .conflict, contactCardOpen: panel == .contactCard)
    }

    /// 提示行这一行占的高度（键盘总高按它加减）：草稿卡、冲突屏、对象卡打开时这一行不画，高度让给面板。
    var hintRowHeight: CGFloat { rowHeights.total }

    /// 候选栏与键区往下挪多少：画出来的提示行有多高。
    var hintRowInset: CGFloat { rowHeights.inset }

    /// 首选候选用强调色（ScopeDisplay.accentFirstCandidate）。
    var accentFirstCandidate: Bool {
        ScopeDisplay.accentFirstCandidate(hasContact: currentContact != nil)
    }

    /// 宿主换了输入框（控制器按 documentIdentifier 判断）：丢掉没上屏的拼音，清掉最近上屏的字与提示。
    func hostChanged() {
        // 没上屏的拼音也丢掉：留着的话，在新输入框按空格会把旧拼音的首选上屏到这里
        endComposedNote()
        dismissRewrite()
        engine?.clear()
        engine?.resetContext()
        panel = .keys
        quickOpen = false
        refresh()
    }

    /// 牌子展开时列的人（nil 是「不指定」）：名单平铺后人数不限，只列排在前面的几个。
    var quickPicks: [String?] {
        ScopeDisplay.quickPicks(people: contacts, current: scope.contactId, used: scope.used)
    }

    /// 当前对象（名单里找得到的）。
    var currentContact: MemoryContact? {
        guard let id = scope.contactId else { return nil }
        return contacts.first { $0.id == id }
    }

    /// 「记一笔」按钮：开了完全访问、选了对象、不在私密输入框（ScopeDisplay.canNote）。
    var canNote: Bool {
        ScopeDisplay.canNote(fullAccess: fullAccess, privateField: privateField, hasContact: currentContact != nil)
    }

    /// 没开完全访问时点牌子：展开 / 收起那段说明（键盘里只有这里能说清为什么要开）。
    func toggleFullAccessNote() { showsFullAccessNote.toggle() }

    /// 点牌子：列出 / 收起其他人。没开完全访问时读不到名单，不做（牌子那时是说明入口）。
    func toggleQuickPicks() {
        guard fullAccess else { return }
        if !quickOpen { reloadContacts() }
        quickOpen.toggle()
        if quickOpen { showsRewriteSkills = false }
    }

    /// 换一个对象（nil 是不指定）：让桥定完，工具栏里列的人收起。
    func chooseContact(_ contactId: String?) {
        applyScope(pick: contactId.map(ScopePick.contact) ?? .nobody)
        quickOpen = false
        panel = .keys
    }

    /// 换当前对象：先让桥在锁里读名单定对象，再按回来的状态重读名单与提示。
    private func applyScope(pick: ScopePick) {
        engine?.setScope(pick: pick)
        let next = engine?.scope ?? scope
        // 手写的草稿是记给原来那个人的，换了人就丢掉
        if next.contactId != scope.contactId { endComposedNote() }
        scope = next
        reloadContacts()
        refreshHint()
    }

    func openContactCard() {
        guard let id = scope.contactId else { return }
        panelCards = engine?.memoryCards(id) ?? []
        panel = .contactCard
    }

    /// 日子提醒的「知道了」：当天不再出。
    func acknowledgeHint() {
        guard let hint else { return }
        engine?.dismissHint(hint.cardId, today: hint.reason == .today)
        refreshHint()
    }

    /// 点「记一笔」：剪贴板有没处理过的文字就读出来（可能弹系统的粘贴授权）显示确认条，否则进手写（NoteEntry）。
    /// 变化计数和上次处理时一样就不读，免得同一段剪贴板反复弹授权。
    func startNote() {
        guard canNote, noteDraft == nil, !sink.isComposingNote else { return }
        let changeCount = sink.pasteboardChangeCount
        let clipboard = sink.pasteboardHasText && changeCount != Self.handledNoteChangeCount
            ? sink.readPasteboard() : nil
        switch NoteEntry.decide(clipboard: clipboard, lastHandledDigest: Self.handledNoteDigest) {
        case .clipboard(let cards):
            noteDraft = cards
            noteSource = clipboard.map { (NoteEntry.digest($0), changeCount) }
        case .compose:
            beginComposedNote()
        case .ignore:
            // 剪贴板有字但不值得记：什么都不出（设计稿 1e：也不提示「未识别」）。不改状态，免得
            // 把这段标成「已处理」——用户下次点「记一笔」还是这个判断，直到剪贴板换成别的。
            break
        }
    }

    /// 点「记到 X」：进草稿卡（1e-2）。
    ///
    /// **现在只是 UI 壳**：草稿内容用样例（照设计稿那条「下个月想去厦门」），
    /// 真正的抽取要等 2C 的云端整理——Task 10 的本地抽取已被审计会话正式暂缓。
    func confirmNote() {
        guard let cards = noteDraft else { return }
        heldRowHeight = liveRowHeight
        noteDraft = nil
        markNoteSourceHandled()
        noteFlow.open(cards: cards)
        panel = .draft
    }

    func toggleDraftSource() {
        noteFlow.toggleSource()
    }

    func updateDraftField(index: Int, value: String) {
        noteFlow.updateField(index: index, value: value)
    }

    func removeDraftField(index: Int) {
        noteFlow.removeField(index: index)
    }

    /// 「不记」：草稿卡或冲突屏收起，什么都不写。
    func discardDraft() {
        noteFlow.discard()
        panel = .keys
    }

    /// 「记下 n 条」：样例里有冲突，先进冲突屏（1e-3），原话留着给冲突屏显示，选完才存——**这条链现在也是壳**，
    /// 等 2C 换成真的冲突比对。冲突落在当前对象身上，称呼用 chipName。
    func saveDraft() {
        noteFlow.save(contactName: currentContact?.chipName ?? "")
        panel = .conflict
    }

    /// 冲突屏上选完之后：都按「存一条素材」存原话，两条怎么合留给 2C 的整理去判。
    func resolveConflict(_ decision: ConflictDecision) {
        let text = noteFlow.resolve(decision)
        panel = .keys
        guard !text.isEmpty else { return }
        saveNote([text], source: "clipboard")
    }

    /// 「忽略」也算处理过：不然剪贴板不变时再点「记一笔」永远是这条，进不了手写。
    func cancelNote() {
        noteDraft = nil
        markNoteSourceHandled()
    }

    /// 手写的草稿（只读，视图用）。
    var composedNote: NoteComposer? { sink.composer }

    /// 「记到」：存成待整理素材（桥的 qj_memory_note），成败都退出手写；没上屏的拼音不算进去，直接丢掉。
    /// 起名字时是「好了」：建对象并切过去，没建成（锁被 App 占着、满 8 个）就留在输入条里显示原因。
    func confirmComposedNote() {
        guard let composer = sink.composer, composer.canSave else { return }
        if namingContact {
            confirmNewContact(composer.text)
            return
        }
        endComposedNote()
        saveNote([composer.text], source: "typed")
    }

    /// 点「新对象」：在提示行的位置打名字，键区照常打字，字只进输入条、不进宿主（同手写记一笔）。
    func startNamingContact() {
        guard fullAccess, !sink.isComposingNote else { return }
        namingContact = true
        namingError = nil
        beginComposedNote()
    }

    private func confirmNewContact(_ draft: String) {
        guard let name = ContactAdd.name(draft), let engine else { return }
        switch engine.addContact(name: name) {
        case .success(let id):
            endComposedNote()
            reloadContacts()
            chooseContact(id)
        case .failure(let failure):
            // 通用文案「键盘正在写记忆」是给 App 看的；这里占锁的是 App
            namingError = failure.code == .lockTimeout ? "素笺 App 正在保存，请再点一次「好了」" : failure.userMessage
        }
    }

    /// 「取消」、换输入框、键盘收起、进私密输入框、换了对象：草稿直接丢掉。
    func cancelComposedNote() {
        endComposedNote()
    }

    /// 输入条是在给新对象起名字（不是手写记一笔）。
    private(set) var namingContact = false

    /// 新建对象没成功的原因，显示在输入条里，再按一个键就消失。
    private(set) var namingError: String?

    private func beginComposedNote() {
        dismissRewrite()
        // 入口在没组字时的工具栏，这里不该有拼音；万一有，先在宿主那边清干净再改道
        if composing {
            engine?.clear()
            refresh()
        }
        if panel != .keys { panel = .keys }
        sink.beginNote()
    }

    /// 先在改道状态下清掉拼音（setMarked 被吞掉），再退出手写；反过来的话清拼音会把 setMarked("") 写进宿主。
    private func endComposedNote() {
        guard sink.isComposingNote else { return }
        if composing {
            engine?.clear()
            refresh()
        }
        sink.endNote()
        namingContact = false
        namingError = nil
    }

    /// 整次交给桥存成素材：确认条里的几条用空行拼回一段，桥按同样的规则再切开（ClipMessages 与桥的 split_note 同一套），
    /// 这样「这次的条数加上没整理的超过 200 就整次不记」对整张确认条成立，不会记一半。拿不到锁时桥进待办、当成功。
    /// 记下了在条上写「记下了 n 条，明早整理」；这个人的待整理装不下就写清原因，不静默；别的失败（App Group 不可写、对象刚被删）不打断打字。
    private func saveNote(_ cards: [String], source: String) {
        guard let id = scope.contactId, !cards.isEmpty else { return }
        let failure = engine?.memoryNote(id, text: cards.joined(separator: "\n\n"), source: source)
        // 只记成败与错误码，不记原话
        let outcome = failure.map { "失败 \($0.code.rawValue)" } ?? "已接受"
        Self.log.info("记一笔 \(cards.count, privacy: .public) 条 \(outcome, privacy: .public)")
        if let failure {
            if failure.code == .materialLimit { showNoteToast(.problem(failure.userMessage)) }
        } else {
            showNoteToast(.done(count: cards.count, cloud: CloudStatus.memoryReady()))
        }
        reloadContacts()
        refreshHint()
    }

    private func showNoteToast(_ toast: NoteToast) {
        noteToast = toast
        noteToastTask?.cancel()
        noteToastTask = Task { @MainActor [weak self] in
            try? await Task.sleep(for: toast.duration)
            guard !Task.isCancelled else { return }
            self?.noteToast = nil
        }
    }

    private func markNoteSourceHandled() {
        guard let source = noteSource else { return }
        noteSource = nil
        Self.handledNoteDigest = source.digest
        Self.handledNoteChangeCount = source.changeCount
    }

    /// 上次记下或忽略的剪贴板（摘要与变化计数），存在扩展自己的 UserDefaults 里（键盘进程随时被杀）。
    private static var handledNoteDigest: String? {
        get { UserDefaults.standard.string(forKey: "handledNoteDigest") }
        set { UserDefaults.standard.set(newValue, forKey: "handledNoteDigest") }
    }

    private static var handledNoteChangeCount: Int? {
        get { UserDefaults.standard.object(forKey: "handledNoteChangeCount") as? Int }
        set { UserDefaults.standard.set(newValue, forKey: "handledNoteChangeCount") }
    }

    /// 提示行里要加粗的词。
    var hintEmphasis: [String] {
        guard let hint else { return [] }
        return HintText.emphasis(for: hint, card: cardIndex[hint.cardId])
    }

    /// 提示对应卡片的来源（手动卡显示「你写的」）。
    var hintSource: String? { hint.flatMap { cardIndex[$0.cardId]?.source } }

    /// 面板里显示一行 2 秒的短提示。
    func showNotice(_ text: String) {
        notice = text
        noticeTask?.cancel()
        noticeTask = Task { @MainActor [weak self] in
            try? await Task.sleep(for: .seconds(2))
            guard !Task.isCancelled else { return }
            self?.notice = nil
        }
    }

    @ObservationIgnored private var noticeTask: Task<Void, Never>?
    @ObservationIgnored private var noteToastTask: Task<Void, Never>?

    /// 换了引擎、键盘出现时：从桥取当前对象，重读名单与提示。
    private func syncScope() {
        scope = engine?.scope ?? MemoryScope()
        reloadContacts()
        refreshHint()
    }

    private func reloadContacts() {
        guard fullAccess, let directory = SharedStore.directory,
              let snapshot = MemoryFiles.read(userDirectory: directory)
        else {
            contacts = []
            cardIndex = [:]
            return
        }
        contacts = snapshot.contacts
        cardIndex = Dictionary(
            snapshot.cards.values.joined().map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first })
    }

    private func refreshHint() {
        let next = privateField ? nil : engine?.memoryHint
        // 提示的卡是刚记下的、名单缓存里还没有时，重读一次再显示
        if let next, cardIndex[next.cardId] == nil { reloadContacts() }
        if next != hint { hint = next }
    }

    private func typeLetter(_ letter: Character) {
        let upper = shifted
        shifted = false
        guard let engine else {
            sink.commit(upper ? letter.uppercased() : String(letter))
            return
        }
        // Shift 字母是临时打英文：组到一半的拼音先原样上屏
        if upper {
            if composing { sink.commit(engine.takeRaw()) }
            sink.commit(letter.uppercased())
            refresh()
            return
        }
        // 光标前后文只在开始组字时取一次：每次都问宿主是一次跨进程往返，每个键都要等
        if !composing {
            engine.setContext(before: sink.contextBefore, after: sink.contextAfter)
        }
        engine.push(letter)
        refresh()
    }

    /// 敲了断句的标点就回字母层，接着打拼音（与系统键盘一致）；数字与 `- / : . @` 这类常夹在数字里的符号留在原层，连着敲。
    private static let returnsToLetters: Set<String> = ["。", "，", "、", "？", "！", "；", "…", "”", "’", "^_^"]

    private func typeSymbol(_ text: String) {
        commitFirst()
        sink.commit(text)
        if Self.returnsToLetters.contains(text) { layer = .letters }
    }

    private func backspace() {
        if composing, let engine {
            engine.backspace()
            refresh()
        } else {
            sink.deleteBackward()
        }
    }

    private func space() {
        if composing {
            commitFirst()
        } else {
            engine?.notePassthrough(" ")
            sink.commit(" ")
        }
        layer = .letters
    }

    /// 手写记一笔时，没在组字的换行等于「记到」（草稿只有一行）；组字中照常把字母原样上屏（进草稿）。
    private func returnKey() {
        if composing, let engine {
            sink.commit(engine.takeRaw())
            refresh()
        } else if sink.isComposingNote {
            confirmComposedNote()
        } else {
            engine?.notePassthrough("\n")
            sink.commit("\n")
        }
    }

    /// 上屏首选；拼不成音节（没有候选）时原样上屏。候选只吃掉一部分拼音时，剩下的接着组。
    private func commitFirst() {
        guard composing, let engine else { return }
        let text = candidates.isEmpty ? engine.takeRaw() : (engine.commit(0) ?? engine.takeRaw())
        sink.commit(text)
        refresh()
    }

    private func refresh() {
        guard let engine else { return }
        namingError = nil
        let next = engine.preedit
        if next != preedit { sink.setMarked(next) }
        preedit = next
        candidates = engine.candidates
        if !composing, panel == .candidates { panel = .keys }
        if composing {
            quickOpen = false
            showsRewriteSkills = false
        }
        refreshHint()
    }
}
