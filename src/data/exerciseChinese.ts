import type { ExerciseDefinition } from "@/src/types";

export const exerciseChineseLabels: Record<string, string> = {
  beginner: "初级", intermediate: "中级", expert: "高级",
  strength: "力量训练", stretching: "拉伸", cardio: "有氧",
  powerlifting: "力量举", "olympic weightlifting": "奥林匹克举重", strongman: "强人训练", plyometrics: "爆发力训练",
  abdominals: "腹肌", abductors: "髋外展肌", adductors: "内收肌", biceps: "肱二头肌", calves: "小腿", chest: "胸部",
  forearms: "前臂", glutes: "臀肌", hamstrings: "腘绳肌", lats: "背阔肌", "lower back": "下背部", "middle back": "中背部",
  neck: "颈部", quadriceps: "股四头肌", shoulders: "肩部", traps: "斜方肌", triceps: "肱三头肌",
  "medicine ball": "药球", dumbbell: "哑铃", "body only": "自重", bands: "弹力带", kettlebells: "壶铃", "foam roll": "泡沫轴",
  cable: "绳索", machine: "固定器械", barbell: "杠铃", "exercise ball": "健身球", "e-z curl bar": "曲杆", other: "其他器械",
  static: "静态", pull: "拉", push: "推", isolation: "孤立", compound: "复合",
};

export const exerciseChineseLabel = (value: string | null | undefined) =>
  value ? exerciseChineseLabels[value] ?? value : "不限";

const nameTerms: [RegExp, string][] = [
  [/\balternate\b|\balternating\b/gi, "交替"], [/\bincline\b/gi, "上斜"], [/\bdecline\b/gi, "下斜"],
  [/\bdumbbell\b/gi, "哑铃"], [/\bbarbell\b/gi, "杠铃"], [/\bkettlebell\b/gi, "壶铃"], [/\bcable\b/gi, "绳索"],
  [/\bmachine\b/gi, "器械"], [/\bband\b/gi, "弹力带"], [/\bbench\b/gi, "卧推凳"], [/\bball\b/gi, "球"],
  [/\bpress\b/gi, "推举"], [/\bbench press\b/gi, "卧推"], [/\bcurl\b/gi, "弯举"], [/\bsquat\b/gi, "深蹲"],
  [/\bdeadlift\b/gi, "硬拉"], [/\brow\b/gi, "划船"], [/\bpull-up\b|\bpullup\b/gi, "引体向上"],
  [/\bpush-up\b|\bpushup\b/gi, "俯卧撑"], [/\blunge\b/gi, "弓步"], [/\braise\b/gi, "抬举"],
  [/\bextension\b/gi, "伸展"], [/\bflexion\b/gi, "屈曲"], [/\bfly\b|\bflye\b/gi, "飞鸟"],
  [/\bcrunch\b/gi, "卷腹"], [/\bplank\b/gi, "平板支撑"], [/\bstretch\b/gi, "拉伸"], [/\bjump\b/gi, "跳跃"],
  [/\brun\b|\brunning\b/gi, "跑步"], [/\bwalk\b|\bwalking\b/gi, "步行"], [/\bcycling\b|\bbike\b/gi, "骑行"],
  [/\bstanding\b/gi, "站姿"], [/\bseated\b/gi, "坐姿"], [/\blying\b/gi, "卧姿"], [/\bkneeling\b/gi, "跪姿"],
  [/\bone-arm\b|\bsingle-arm\b/gi, "单臂"], [/\bone-leg\b|\bsingle-leg\b/gi, "单腿"], [/\breverse\b/gi, "反向"],
  [/\bfront\b/gi, "前侧"], [/\brear\b/gi, "后侧"], [/\blateral\b|\bside\b/gi, "侧向"], [/\boverhead\b/gi, "过顶"],
  [/\bwide\b/gi, "宽距"], [/\bclose\b/gi, "窄距"], [/\bgrip\b/gi, "握距"], [/\bhigh\b/gi, "高位"], [/\blow\b/gi, "低位"],
  [/\bhip\b/gi, "髋部"], [/\bleg\b/gi, "腿部"], [/\bcalf\b/gi, "小腿"], [/\bchest\b/gi, "胸部"],
  [/\bshoulder\b/gi, "肩部"], [/\btriceps\b/gi, "肱三头肌"], [/\bbiceps\b/gi, "肱二头肌"], [/\bback\b/gi, "背部"],
  [/\babdominal\b|\babs\b/gi, "腹部"], [/\bglute\b/gi, "臀部"], [/\bhamstring\b/gi, "腘绳肌"],
  [/\brotation\b|\btwist\b/gi, "转体"], [/\bhold\b/gi, "静止"], [/\broll\b/gi, "滚动"], [/\bclean\b/gi, "翻举"],
  [/\bsnatch\b/gi, "抓举"], [/\bdip\b/gi, "双杠臂屈伸"], [/\bshrug\b/gi, "耸肩"], [/\bswing\b/gi, "摆动"],
];

export function localizeExercise(exercise: ExerciseDefinition, index: number) {
  let translated = exercise.name;
  for (const [pattern, replacement] of nameTerms) translated = translated.replace(pattern, replacement);
  translated = translated.replace(/[()]/g, " ").replace(/[-_/]+/g, " ").replace(/\s+/g, " ").trim();
  const muscle = exercise.primaryMuscles.map(exerciseChineseLabel).join("、") || "全身";
  const equipment = exerciseChineseLabel(exercise.equipment);
  const category = exerciseChineseLabel(exercise.category);
  const nameZh = /[A-Za-z]/.test(translated)
    ? `${muscle}${equipment}${category}动作（${String(index + 1).padStart(3, "0")}）`
    : translated;
  const instructionsZh = [
    `准备合适的${equipment}，根据示意图调整起始姿势，保持躯干稳定。`,
    `以${muscle}为主要发力部位，用平稳、可控制的节奏完成动作。`,
    "在舒适活动范围内完成全程，避免突然借力，并缓慢回到起始位置。",
  ];
  return { nameZh, instructionsZh };
}
