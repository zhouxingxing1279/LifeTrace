import SwiftUI
import SwiftData

struct TemplatesView: View {
    @Environment(\.modelContext) private var context
    @Query(sort: \WorkoutTemplateEntity.createdAt, order: .reverse) private var templates: [WorkoutTemplateEntity]
    @State private var editing: WorkoutTemplateEntity?
    @State private var creating = false
    @State private var active: WorkoutTemplateEntity?

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                TrainingHero()
                if templates.isEmpty {
                    ContentUnavailableView("还没有训练模板", systemImage: "dumbbell", description: Text("创建模板并从动作库选择训练动作。"))
                        .frame(minHeight: 320)
                } else {
                    ForEach(templates) { template in
                        VStack(alignment: .leading, spacing: 14) {
                            HStack {
                                Text(template.icon).font(.title2.bold())
                                    .frame(width: 46, height: 46)
                                    .background(HXTheme.mint, in: RoundedRectangle(cornerRadius: 14))
                                VStack(alignment: .leading) {
                                    Text(template.name).font(.headline)
                                    Text("\(template.exercises.count) 个动作 · \(template.exercises.reduce(0) { $0 + $1.sets.count }) 组 · \(template.estimatedMinutes) 分钟")
                                        .font(.caption).foregroundStyle(.secondary)
                                }
                                Spacer()
                                Menu {
                                    Button("编辑") { editing = template }
                                    Button("删除", role: .destructive) { context.delete(template); try? context.save() }
                                } label: {
                                    Image(systemName: "ellipsis.circle").font(.title3)
                                }
                            }
                            if !template.note.isEmpty {
                                Text(template.note).font(.subheadline).foregroundStyle(.secondary)
                            }
                            Button {
                                active = template
                            } label: {
                                Label("开始训练", systemImage: "play.fill")
                                    .frame(maxWidth: .infinity)
                            }
                            .buttonStyle(.borderedProminent)
                            .tint(HXTheme.green)
                        }
                        .hxCard()
                    }
                }
            }
            .padding()
        }
        .background(HXTheme.canvas)
        .navigationTitle("健身训练")
        .toolbar {
            Button { creating = true } label: { Image(systemName: "plus") }
        }
        .sheet(isPresented: $creating) { TemplateEditorView() }
        .sheet(item: $editing) { TemplateEditorView(template: $0) }
        .fullScreenCover(item: $active) { WorkoutSessionView(template: $0) }
    }
}

private struct TrainingHero: View {
    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 8) {
                Text("Life trace 健身").font(.caption.weight(.bold)).foregroundStyle(HXTheme.lime)
                Text("按计划训练，\n让每一组都有记录。").font(.title2.bold()).foregroundStyle(.white)
            }
            Spacer()
            Image(systemName: "figure.strengthtraining.traditional")
                .font(.system(size: 48)).foregroundStyle(HXTheme.lime)
        }
        .padding(22)
        .background(HXTheme.deep, in: RoundedRectangle(cornerRadius: 24))
    }
}

private struct DraftExercise: Identifiable {
    let id: UUID
    let sourceID: String
    var name: String
    var restSeconds: Int
    var sets: [WorkoutSetValue]

    init(sourceID: String, name: String, restSeconds: Int = 90, sets: [WorkoutSetValue] = [
        WorkoutSetValue(weight: 0, reps: 10),
        WorkoutSetValue(weight: 0, reps: 10),
        WorkoutSetValue(weight: 0, reps: 10)
    ]) {
        self.id = UUID()
        self.sourceID = sourceID
        self.name = name
        self.restSeconds = restSeconds
        self.sets = sets
    }
}

struct TemplateEditorView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.modelContext) private var context
    let template: WorkoutTemplateEntity?
    @State private var name: String
    @State private var note: String
    @State private var icon: String
    @State private var minutes: Int
    @State private var drafts: [DraftExercise]
    @State private var selecting = false

    init(template: WorkoutTemplateEntity? = nil) {
        self.template = template
        _name = State(initialValue: template?.name ?? "")
        _note = State(initialValue: template?.note ?? "")
        _icon = State(initialValue: template?.icon ?? "训")
        _minutes = State(initialValue: template?.estimatedMinutes ?? 50)
        _drafts = State(initialValue: (template?.exercises ?? []).sorted { $0.order < $1.order }.map {
            DraftExercise(sourceID: $0.sourceID, name: $0.name, restSeconds: $0.restSeconds, sets: $0.sets)
        })
    }

    var body: some View {
        NavigationStack {
            Form {
                Section("模板信息") {
                    TextField("模板名称", text: $name)
                    TextField("模板说明", text: $note, axis: .vertical)
                    HStack {
                        TextField("标识", text: $icon).frame(width: 70)
                        Stepper("预计 \(minutes) 分钟", value: $minutes, in: 5...240, step: 5)
                    }
                }
                Section {
                    Button { selecting = true } label: {
                        Label("从动作库选择", systemImage: "square.grid.2x2")
                    }
                    if drafts.isEmpty {
                        Text("请至少选择一个训练动作。").foregroundStyle(.secondary)
                    }
                } header: {
                    Text("训练动作")
                } footer: {
                    Text("动作只能从资料库选择，选择后再设置训练组。")
                }
                ForEach($drafts) { $draft in
                    Section {
                        Stepper("组间休息 \(draft.restSeconds) 秒", value: $draft.restSeconds, in: 0...600, step: 15)
                        ForEach(Array(draft.sets.indices), id: \.self) { index in
                            HStack {
                                Text("第 \(index + 1) 组")
                                Spacer()
                                TextField("重量", value: $draft.sets[index].weight, format: .number)
                                    .keyboardType(.decimalPad).frame(width: 58)
                                Text("千克").font(.caption).foregroundStyle(.secondary)
                                TextField("次数", value: $draft.sets[index].reps, format: .number)
                                    .keyboardType(.numberPad).frame(width: 44)
                            }
                        }
                        HStack {
                            Button("增加一组") {
                                let last = draft.sets.last ?? WorkoutSetValue(weight: 0, reps: 10)
                                draft.sets.append(WorkoutSetValue(weight: last.weight, reps: last.reps))
                            }
                            Spacer()
                            if draft.sets.count > 1 {
                                Button("删除一组", role: .destructive) { draft.sets.removeLast() }
                            }
                        }
                        Button("移除动作", role: .destructive) {
                            drafts.removeAll { $0.id == draft.id }
                        }
                    } header: {
                        Text(draft.name)
                    }
                }
            }
            .navigationTitle(template == nil ? "新建模板" : "编辑模板")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) { Button("取消") { dismiss() } }
                ToolbarItem(placement: .confirmationAction) {
                    Button("保存") { save() }
                        .disabled(name.trimmingCharacters(in: .whitespaces).isEmpty || drafts.isEmpty)
                }
            }
            .sheet(isPresented: $selecting) {
                ExerciseSelectionView(selectedIDs: Set(drafts.map(\.sourceID))) { exercise in
                    guard !drafts.contains(where: { $0.sourceID == exercise.sourceID }) else { return }
                    drafts.append(DraftExercise(sourceID: exercise.sourceID, name: exercise.name))
                }
            }
        }
    }

    private func save() {
        let target = template ?? WorkoutTemplateEntity(name: name)
        target.name = name
        target.note = note
        target.icon = String(icon.prefix(2))
        target.estimatedMinutes = minutes
        target.exercises.forEach { context.delete($0) }
        target.exercises = drafts.enumerated().map { index, draft in
            TemplateExerciseEntity(sourceID: draft.sourceID, name: draft.name, restSeconds: draft.restSeconds, order: index, sets: draft.sets)
        }
        if template == nil { context.insert(target) }
        try? context.save()
        dismiss()
    }
}

private struct ExerciseSelectionView: View {
    @Environment(\.dismiss) private var dismiss
    @Query(sort: \ExerciseEntity.name) private var exercises: [ExerciseEntity]
    let selectedIDs: Set<String>
    let onSelect: (ExerciseEntity) -> Void
    @State private var category = "力量训练"
    @State private var muscle = "全部"
    @State private var search = ""

    private var categories: [String] { Set(exercises.map(\.category)).sorted() }
    private var muscles: [String] { ["全部"] + Set(exercises.flatMap(\.muscles)).sorted() }
    private var filtered: [ExerciseEntity] {
        exercises.filter {
            (category.isEmpty || $0.category == category)
            && (muscle == "全部" || $0.muscles.contains(muscle))
            && (search.isEmpty || $0.name.localizedCaseInsensitiveContains(search))
        }
    }

    var body: some View {
        NavigationStack {
            List {
                Section {
                    ScrollView(.horizontal, showsIndicators: false) {
                        HStack {
                            ForEach(categories, id: \.self) { item in
                                Button(item) { category = item }
                                    .buttonStyle(.borderedProminent)
                                    .tint(category == item ? HXTheme.green : Color.gray.opacity(0.25))
                                    .foregroundStyle(category == item ? .white : .primary)
                            }
                        }
                    }
                    Picker("目标肌群", selection: $muscle) {
                        ForEach(muscles, id: \.self) { Text($0).tag($0) }
                    }
                }
                Section("共 \(filtered.count) 个动作") {
                    ForEach(filtered) { exercise in
                        Button {
                            onSelect(exercise)
                        } label: {
                            HStack {
                                VStack(alignment: .leading, spacing: 4) {
                                    Text(exercise.name).foregroundStyle(.primary)
                                    Text("\(exercise.muscles.joined(separator: "、")) · \(exercise.equipment)")
                                        .font(.caption).foregroundStyle(.secondary)
                                }
                                Spacer()
                                Image(systemName: selectedIDs.contains(exercise.sourceID) ? "checkmark.circle.fill" : "plus.circle")
                                    .foregroundStyle(HXTheme.green)
                            }
                        }
                        .disabled(selectedIDs.contains(exercise.sourceID))
                    }
                }
            }
            .searchable(text: $search, prompt: "搜索动作名称")
            .navigationTitle("选择训练动作")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { Button("完成") { dismiss() } }
        }
    }
}
