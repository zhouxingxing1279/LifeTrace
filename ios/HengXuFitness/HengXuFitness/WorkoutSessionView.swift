import SwiftUI
import SwiftData
import Combine

private struct SessionSet: Identifiable {
    let id = UUID()
    let exerciseName: String
    let setNumber: Int
    let weight: Double
    let reps: Int
}

struct WorkoutSessionView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.modelContext) private var context
    let template: WorkoutTemplateEntity
    @State private var completed = 0
    @State private var elapsed = 0
    @State private var showingExit = false
    private let sets: [SessionSet]
    private let timer = Timer.publish(every: 1, on: .main, in: .common).autoconnect()

    init(template: WorkoutTemplateEntity) {
        self.template = template
        self.sets = template.exercises.sorted { $0.order < $1.order }.flatMap { exercise in
            exercise.sets.enumerated().map { index, set in
                SessionSet(exerciseName: exercise.name, setNumber: index + 1, weight: set.weight, reps: set.reps)
            }
        }
    }

    private var current: SessionSet? {
        completed < sets.count ? sets[completed] : nil
    }

    var body: some View {
        NavigationStack {
            ZStack {
                HXTheme.canvas.ignoresSafeArea()
                ScrollView {
                    VStack(spacing: 18) {
                        VStack(spacing: 10) {
                            Text(template.name).font(.title2.bold()).foregroundStyle(.white)
                            Text(timeText).font(.system(.title3, design: .monospaced).weight(.semibold)).foregroundStyle(HXTheme.lime)
                            ProgressView(value: Double(completed), total: Double(max(sets.count, 1)))
                                .tint(HXTheme.lime)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(24)
                        .background(HXTheme.deep, in: RoundedRectangle(cornerRadius: 24))

                        if let current {
                            VStack(spacing: 18) {
                                Text("当前训练组").font(.caption.weight(.bold)).foregroundStyle(HXTheme.green)
                                Text(current.exerciseName).font(.title2.bold()).multilineTextAlignment(.center)
                                Text("第 \(current.setNumber) 组").foregroundStyle(.secondary)
                                HStack(spacing: 12) {
                                    MetricBox(value: current.weight.formatted(), unit: "千克")
                                    MetricBox(value: "\(current.reps)", unit: "次数")
                                }
                                Button {
                                    withAnimation { completed += 1 }
                                } label: {
                                    Label("完成当前组", systemImage: "checkmark")
                                        .frame(maxWidth: .infinity)
                                }
                                .buttonStyle(.borderedProminent)
                                .controlSize(.large)
                                .tint(HXTheme.green)
                            }
                            .hxCard()
                        } else {
                            VStack(spacing: 18) {
                                Image(systemName: "checkmark.circle.fill")
                                    .font(.system(size: 60)).foregroundStyle(HXTheme.green)
                                Text("本次训练已完成").font(.title2.bold())
                                Text("共完成 \(sets.count) 个训练组").foregroundStyle(.secondary)
                                Button("保存训练记录") { save() }
                                    .buttonStyle(.borderedProminent)
                                    .controlSize(.large)
                                    .tint(HXTheme.green)
                            }
                            .frame(maxWidth: .infinity)
                            .hxCard()
                        }

                        VStack(spacing: 0) {
                            ForEach(Array(sets.enumerated()), id: \.element.id) { index, set in
                                HStack {
                                    Image(systemName: index < completed ? "checkmark.circle.fill" : index == completed ? "circle.inset.filled" : "circle")
                                        .foregroundStyle(index <= completed ? HXTheme.green : .secondary)
                                    VStack(alignment: .leading) {
                                        Text(set.exerciseName).font(.subheadline.weight(.semibold))
                                        Text("第 \(set.setNumber) 组 · \(set.weight.formatted()) 千克 × \(set.reps)")
                                            .font(.caption).foregroundStyle(.secondary)
                                    }
                                    Spacer()
                                }
                                .padding(.vertical, 11)
                                if index < sets.count - 1 { Divider() }
                            }
                        }
                        .hxCard()
                    }
                    .padding()
                }
            }
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    Button("退出") { showingExit = true }
                }
                ToolbarItem(placement: .principal) {
                    Text("\(completed) / \(sets.count) 组").font(.headline)
                }
            }
            .onReceive(timer) { _ in elapsed += 1 }
            .confirmationDialog("确定结束本次训练吗？", isPresented: $showingExit, titleVisibility: .visible) {
                Button("结束且不保存", role: .destructive) { dismiss() }
                Button("继续训练", role: .cancel) {}
            }
        }
    }

    private var timeText: String {
        String(format: "%02d:%02d", elapsed / 60, elapsed % 60)
    }

    private func save() {
        context.insert(WorkoutHistoryEntity(
            templateName: template.name,
            durationSeconds: elapsed,
            exerciseCount: template.exercises.count,
            setCount: sets.count
        ))
        try? context.save()
        dismiss()
    }
}

private struct MetricBox: View {
    let value: String
    let unit: String
    var body: some View {
        VStack(spacing: 4) {
            Text(value).font(.system(size: 32, weight: .bold, design: .rounded))
            Text(unit).font(.caption).foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity)
        .padding()
        .background(HXTheme.mint, in: RoundedRectangle(cornerRadius: 16))
    }
}
