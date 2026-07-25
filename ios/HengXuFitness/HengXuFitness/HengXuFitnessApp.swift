import SwiftUI
import SwiftData

@main
struct HengXuFitnessApp: App {
    private let container: ModelContainer = {
        let schema = Schema([
            ExerciseEntity.self,
            WorkoutTemplateEntity.self,
            TemplateExerciseEntity.self,
            WorkoutHistoryEntity.self
        ])
        let configuration = ModelConfiguration("HengXuFitness", schema: schema)
        do {
            return try ModelContainer(for: schema, configurations: [configuration])
        } catch {
            fatalError("无法初始化本地数据库：\(error.localizedDescription)")
        }
    }()

    var body: some Scene {
        WindowGroup {
            RootView()
        }
        .modelContainer(container)
    }
}
