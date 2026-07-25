import Foundation
import SwiftData

enum JSONText {
    static func encode(_ values: [String]) -> String {
        guard let data = try? JSONEncoder().encode(values) else { return "[]" }
        return String(data: data, encoding: .utf8) ?? "[]"
    }

    static func decode(_ value: String) -> [String] {
        guard let data = value.data(using: .utf8) else { return [] }
        return (try? JSONDecoder().decode([String].self, from: data)) ?? []
    }
}

struct WorkoutSetValue: Codable, Identifiable, Hashable {
    var id = UUID()
    var weight: Double
    var reps: Int
}

@Model
final class ExerciseEntity {
    @Attribute(.unique) var sourceID: String
    var name: String
    var category: String
    var level: String
    var equipment: String
    var force: String
    var musclesJSON: String
    var instructionsJSON: String
    var imageURLsJSON: String

    init(sourceID: String, name: String, category: String, level: String, equipment: String, force: String, muscles: [String], instructions: [String], imageURLs: [String]) {
        self.sourceID = sourceID
        self.name = name
        self.category = category
        self.level = level
        self.equipment = equipment
        self.force = force
        self.musclesJSON = JSONText.encode(muscles)
        self.instructionsJSON = JSONText.encode(instructions)
        self.imageURLsJSON = JSONText.encode(imageURLs)
    }

    var muscles: [String] { JSONText.decode(musclesJSON) }
    var instructions: [String] { JSONText.decode(instructionsJSON) }
    var imageURLs: [String] { JSONText.decode(imageURLsJSON) }
}

@Model
final class WorkoutTemplateEntity {
    @Attribute(.unique) var id: UUID
    var name: String
    var note: String
    var estimatedMinutes: Int
    var icon: String
    var createdAt: Date
    @Relationship(deleteRule: .cascade, inverse: \TemplateExerciseEntity.template)
    var exercises: [TemplateExerciseEntity]

    init(name: String, note: String = "", estimatedMinutes: Int = 50, icon: String = "训", exercises: [TemplateExerciseEntity] = []) {
        self.id = UUID()
        self.name = name
        self.note = note
        self.estimatedMinutes = estimatedMinutes
        self.icon = icon
        self.createdAt = Date()
        self.exercises = exercises
    }
}

@Model
final class TemplateExerciseEntity {
    @Attribute(.unique) var id: UUID
    var sourceID: String
    var name: String
    var restSeconds: Int
    var order: Int
    var setsJSON: String
    var template: WorkoutTemplateEntity?

    init(sourceID: String, name: String, restSeconds: Int = 90, order: Int, sets: [WorkoutSetValue] = [
        WorkoutSetValue(weight: 0, reps: 10),
        WorkoutSetValue(weight: 0, reps: 10),
        WorkoutSetValue(weight: 0, reps: 10)
    ]) {
        self.id = UUID()
        self.sourceID = sourceID
        self.name = name
        self.restSeconds = restSeconds
        self.order = order
        self.setsJSON = Self.encodeSets(sets)
    }

    var sets: [WorkoutSetValue] {
        get { Self.decodeSets(setsJSON) }
        set { setsJSON = Self.encodeSets(newValue) }
    }

    private static func encodeSets(_ sets: [WorkoutSetValue]) -> String {
        guard let data = try? JSONEncoder().encode(sets) else { return "[]" }
        return String(data: data, encoding: .utf8) ?? "[]"
    }

    private static func decodeSets(_ value: String) -> [WorkoutSetValue] {
        guard let data = value.data(using: .utf8) else { return [] }
        return (try? JSONDecoder().decode([WorkoutSetValue].self, from: data)) ?? []
    }
}

@Model
final class WorkoutHistoryEntity {
    @Attribute(.unique) var id: UUID
    var templateName: String
    var completedAt: Date
    var durationSeconds: Int
    var exerciseCount: Int
    var setCount: Int

    init(templateName: String, durationSeconds: Int, exerciseCount: Int, setCount: Int) {
        self.id = UUID()
        self.templateName = templateName
        self.completedAt = Date()
        self.durationSeconds = durationSeconds
        self.exerciseCount = exerciseCount
        self.setCount = setCount
    }
}
