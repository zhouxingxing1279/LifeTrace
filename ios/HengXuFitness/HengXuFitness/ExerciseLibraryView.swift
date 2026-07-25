import SwiftUI
import SwiftData

struct ExerciseLibraryView: View {
    @Query(sort: \ExerciseEntity.name) private var exercises: [ExerciseEntity]
    @State private var search = ""
    @State private var category = "全部"
    @State private var muscle = "全部"
    @State private var selected: ExerciseEntity?

    private var categories: [String] {
        ["全部"] + Set(exercises.map(\.category)).sorted()
    }

    private var muscles: [String] {
        ["全部"] + Set(exercises.flatMap(\.muscles)).sorted()
    }

    private var filtered: [ExerciseEntity] {
        exercises.filter { exercise in
            (search.isEmpty || exercise.name.localizedCaseInsensitiveContains(search))
            && (category == "全部" || exercise.category == category)
            && (muscle == "全部" || exercise.muscles.contains(muscle))
        }
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                Picker("训练分类", selection: $category) {
                    ForEach(categories, id: \.self) { Text($0).tag($0) }
                }
                .pickerStyle(.menu)

                Picker("目标肌群", selection: $muscle) {
                    ForEach(muscles, id: \.self) { Text($0).tag($0) }
                }
                .pickerStyle(.menu)

                Text("找到 \(filtered.count) 个动作")
                    .font(.caption)
                    .foregroundStyle(.secondary)

                LazyVStack(spacing: 12) {
                    ForEach(filtered) { exercise in
                        Button {
                            selected = exercise
                        } label: {
                            ExerciseRow(exercise: exercise)
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
            .padding()
        }
        .background(HXTheme.canvas)
        .navigationTitle("动作资料库")
        .searchable(text: $search, prompt: "搜索中文动作名称")
        .sheet(item: $selected) { ExerciseDetailView(exercise: $0) }
    }
}

struct ExerciseRow: View {
    let exercise: ExerciseEntity

    var body: some View {
        HStack(spacing: 14) {
            AsyncImage(url: exercise.imageURLs.first.flatMap(URL.init(string:))) { image in
                image.resizable().scaledToFill()
            } placeholder: {
                RoundedRectangle(cornerRadius: 14).fill(HXTheme.mint)
                    .overlay(Image(systemName: "figure.strengthtraining.traditional").foregroundStyle(HXTheme.green))
            }
            .frame(width: 76, height: 76)
            .clipShape(RoundedRectangle(cornerRadius: 14))

            VStack(alignment: .leading, spacing: 6) {
                Text(exercise.name)
                    .font(.headline)
                    .foregroundStyle(.primary)
                    .lineLimit(2)
                Text("\(exercise.category) · \(exercise.level)")
                    .font(.caption)
                    .foregroundStyle(HXTheme.green)
                Text("\(exercise.muscles.joined(separator: "、")) · \(exercise.equipment)")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            Image(systemName: "chevron.right")
                .font(.caption.weight(.bold))
                .foregroundStyle(.tertiary)
        }
        .hxCard()
    }
}

struct ExerciseDetailView: View {
    @Environment(\.dismiss) private var dismiss
    let exercise: ExerciseEntity

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 18) {
                    TabView {
                        ForEach(exercise.imageURLs, id: \.self) { value in
                            AsyncImage(url: URL(string: value)) { image in
                                image.resizable().scaledToFit()
                            } placeholder: {
                                Rectangle().fill(HXTheme.mint)
                            }
                        }
                    }
                    .frame(height: 280)
                    .tabViewStyle(.page)

                    Text(exercise.name).font(.title2.bold())
                    Text("\(exercise.category) · \(exercise.level) · \(exercise.equipment)")
                        .font(.subheadline)
                        .foregroundStyle(HXTheme.green)
                    Text("主要肌群").font(.headline)
                    Text(exercise.muscles.joined(separator: "、"))
                    Text("动作要点").font(.headline)
                    ForEach(Array(exercise.instructions.enumerated()), id: \.offset) { index, instruction in
                        HStack(alignment: .top, spacing: 10) {
                            Text("\(index + 1)").font(.caption.bold()).foregroundStyle(.white)
                                .frame(width: 24, height: 24).background(HXTheme.green, in: Circle())
                            Text(instruction).font(.subheadline).foregroundStyle(.secondary)
                        }
                    }
                }
                .padding()
            }
            .navigationTitle("动作详情")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { Button("完成") { dismiss() } }
        }
    }
}
