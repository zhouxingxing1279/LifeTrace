import SwiftUI
import SwiftData

struct RootView: View {
    @Environment(\.modelContext) private var context
    @Query private var exercises: [ExerciseEntity]
    @State private var importError: String?
    @State private var importing = false

    var body: some View {
        TabView {
            NavigationStack { TemplatesView() }
                .tabItem { Label("训练", systemImage: "dumbbell.fill") }
            NavigationStack { ExerciseLibraryView() }
                .tabItem { Label("动作库", systemImage: "square.grid.2x2.fill") }
            NavigationStack { HistoryView() }
                .tabItem { Label("历史", systemImage: "clock.arrow.circlepath") }
        }
        .tint(HXTheme.green)
        .overlay {
            if importing {
                ZStack {
                    Color.black.opacity(0.16).ignoresSafeArea()
                    VStack(spacing: 14) {
                        ProgressView()
                        Text("正在导入中文动作库…")
                            .font(.subheadline.weight(.semibold))
                    }
                    .padding(24)
                    .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 20))
                }
            }
        }
        .task {
            guard exercises.isEmpty, !importing else { return }
            importing = true
            do {
                try await ExerciseImporter.importIfNeeded(context: context)
            } catch {
                importError = "动作库导入失败，请检查网络后重试。"
            }
            importing = false
        }
        .alert("无法导入动作库", isPresented: Binding(
            get: { importError != nil },
            set: { if !$0 { importError = nil } }
        )) {
            Button("知道了", role: .cancel) {}
        } message: {
            Text(importError ?? "")
        }
    }
}
