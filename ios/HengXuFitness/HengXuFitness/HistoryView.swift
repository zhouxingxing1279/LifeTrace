import SwiftUI
import SwiftData

struct HistoryView: View {
    @Query(sort: \WorkoutHistoryEntity.completedAt, order: .reverse) private var history: [WorkoutHistoryEntity]

    var body: some View {
        Group {
            if history.isEmpty {
                ContentUnavailableView("还没有训练记录", systemImage: "clock.arrow.circlepath", description: Text("完成一次训练后，记录会显示在这里。"))
            } else {
                List {
                    Section {
                        HStack {
                            HistoryMetric(value: "\(history.count)", label: "训练次数")
                            HistoryMetric(value: "\(history.reduce(0) { $0 + $1.setCount })", label: "完成组数")
                            HistoryMetric(value: "\(history.reduce(0) { $0 + $1.durationSeconds } / 60)", label: "训练分钟")
                        }
                        .listRowInsets(EdgeInsets())
                        .listRowBackground(Color.clear)
                    }
                    Section("全部记录") {
                        ForEach(history) { item in
                            HStack(spacing: 14) {
                                VStack {
                                    Text(item.completedAt, format: .dateTime.month(.twoDigits))
                                    Text(item.completedAt, format: .dateTime.day(.twoDigits))
                                }
                                .font(.caption.bold())
                                .frame(width: 48, height: 48)
                                .background(HXTheme.mint, in: RoundedRectangle(cornerRadius: 13))

                                VStack(alignment: .leading, spacing: 5) {
                                    Text(item.templateName).font(.headline)
                                    Text("\(item.exerciseCount) 个动作 · \(item.setCount) 组 · \(max(1, item.durationSeconds / 60)) 分钟")
                                        .font(.caption).foregroundStyle(.secondary)
                                }
                            }
                            .padding(.vertical, 4)
                        }
                    }
                }
            }
        }
        .navigationTitle("训练历史")
    }
}

private struct HistoryMetric: View {
    let value: String
    let label: String
    var body: some View {
        VStack(spacing: 5) {
            Text(value).font(.title2.bold())
            Text(label).font(.caption2).foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 16)
        .background(.white, in: RoundedRectangle(cornerRadius: 16))
    }
}
