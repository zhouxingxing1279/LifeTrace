import Foundation
import SwiftData

private struct RemoteExercise: Decodable {
    let id: String
    let name: String
    let force: String?
    let level: String
    let equipment: String?
    let primaryMuscles: [String]
    let category: String
    let images: [String]
}

enum ExerciseImporter {
    private static let sourceURL = URL(string: "https://raw.githubusercontent.com/yuhonas/free-exercise-db/main/dist/exercises.json")!
    private static let imageBase = "https://raw.githubusercontent.com/yuhonas/free-exercise-db/main/exercises/"

    @MainActor
    static func importIfNeeded(context: ModelContext) async throws {
        var descriptor = FetchDescriptor<ExerciseEntity>()
        descriptor.fetchLimit = 1
        guard try context.fetch(descriptor).isEmpty else { return }

        let (data, response) = try await URLSession.shared.data(from: sourceURL)
        guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
            throw URLError(.badServerResponse)
        }
        let remote = try JSONDecoder().decode([RemoteExercise].self, from: data)
        for (index, item) in remote.enumerated() {
            let muscles = item.primaryMuscles.map(label)
            let equipment = label(item.equipment ?? "other")
            let name = localizedName(item, index: index, muscles: muscles, equipment: equipment)
            let instructions = [
                "准备合适的\(equipment)，根据示意图调整起始姿势，保持躯干稳定。",
                "以\(muscles.joined(separator: "、"))为主要发力部位，用平稳、可控制的节奏完成动作。",
                "在舒适活动范围内完成全程，避免突然借力，并缓慢回到起始位置。"
            ]
            context.insert(ExerciseEntity(
                sourceID: item.id,
                name: name,
                category: label(item.category),
                level: label(item.level),
                equipment: equipment,
                force: label(item.force ?? "static"),
                muscles: muscles,
                instructions: instructions,
                imageURLs: item.images.map { imageBase + $0 }
            ))
        }
        try context.save()
    }

    static func label(_ value: String) -> String {
        labels[value] ?? value
    }

    private static func localizedName(_ item: RemoteExercise, index: Int, muscles: [String], equipment: String) -> String {
        var name = item.name
        for (pattern, replacement) in nameTerms {
            name = name.replacingOccurrences(of: pattern, with: replacement, options: [.regularExpression, .caseInsensitive])
        }
        name = name.replacingOccurrences(of: "[-_/()]+", with: " ", options: .regularExpression)
            .replacingOccurrences(of: "\\s+", with: " ", options: .regularExpression)
            .trimmingCharacters(in: .whitespaces)
        if name.range(of: "[A-Za-z]", options: .regularExpression) != nil {
            return "\(muscles.joined(separator: "、"))\(equipment)\(label(item.category))动作（\(String(format: "%03d", index + 1))）"
        }
        return name
    }

    private static let labels: [String: String] = [
        "beginner": "初级", "intermediate": "中级", "expert": "高级",
        "strength": "力量训练", "stretching": "拉伸", "cardio": "有氧",
        "powerlifting": "力量举", "olympic weightlifting": "奥林匹克举重", "strongman": "强人训练", "plyometrics": "爆发力训练",
        "abdominals": "腹肌", "abductors": "髋外展肌", "adductors": "内收肌", "biceps": "肱二头肌", "calves": "小腿", "chest": "胸部",
        "forearms": "前臂", "glutes": "臀肌", "hamstrings": "腘绳肌", "lats": "背阔肌", "lower back": "下背部", "middle back": "中背部",
        "neck": "颈部", "quadriceps": "股四头肌", "shoulders": "肩部", "traps": "斜方肌", "triceps": "肱三头肌",
        "medicine ball": "药球", "dumbbell": "哑铃", "body only": "自重", "bands": "弹力带", "kettlebells": "壶铃", "foam roll": "泡沫轴",
        "cable": "绳索", "machine": "固定器械", "barbell": "杠铃", "exercise ball": "健身球", "e-z curl bar": "曲杆", "other": "其他器械",
        "static": "静态", "pull": "拉", "push": "推"
    ]

    private static let nameTerms: [(String, String)] = [
        ("\\balternate\\b|\\balternating\\b", "交替"), ("\\bincline\\b", "上斜"), ("\\bdecline\\b", "下斜"),
        ("\\bdumbbell\\b", "哑铃"), ("\\bbarbell\\b", "杠铃"), ("\\bkettlebell\\b", "壶铃"), ("\\bcable\\b", "绳索"),
        ("\\bpress\\b", "推举"), ("\\bcurl\\b", "弯举"), ("\\bsquat\\b", "深蹲"), ("\\bdeadlift\\b", "硬拉"),
        ("\\brow\\b", "划船"), ("\\bpull-?up\\b", "引体向上"), ("\\bpush-?up\\b", "俯卧撑"), ("\\blunge\\b", "弓步"),
        ("\\braise\\b", "抬举"), ("\\bextension\\b", "伸展"), ("\\bflye?\\b", "飞鸟"), ("\\bcrunch\\b", "卷腹"),
        ("\\bplank\\b", "平板支撑"), ("\\bstretch\\b", "拉伸"), ("\\bjump\\b", "跳跃"), ("\\bstanding\\b", "站姿"),
        ("\\bseated\\b", "坐姿"), ("\\blying\\b", "卧姿"), ("\\breverse\\b", "反向"), ("\\blateral\\b|\\bside\\b", "侧向"),
        ("\\boverhead\\b", "过顶"), ("\\bchest\\b", "胸部"), ("\\bshoulder\\b", "肩部"), ("\\bback\\b", "背部"),
        ("\\btriceps\\b", "肱三头肌"), ("\\bbiceps\\b", "肱二头肌"), ("\\bglute\\b", "臀部"), ("\\brotation\\b|\\btwist\\b", "转体")
    ]
}
