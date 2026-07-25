import type {
  CEFRLevel,
  EnglishAIAnalysis,
  EnglishArticle,
  EnglishMistake,
} from "@/src/types/english";

export interface EnglishAnalysisInput {
  article: EnglishArticle;
  summary: string;
  userLevel: CEFRLevel;
  recordId: string;
  userId: string;
}

export interface EnglishAnalysisService {
  analyze(input: EnglishAnalysisInput): Promise<EnglishAIAnalysis>;
}

const clamp = (value: number) => Math.max(0, Math.min(100, Math.round(value)));
const words = (value: string) => value.trim().split(/\s+/).filter(Boolean);

// Mock 评分保持确定性，便于本地测试；接口形状与未来 DeepSeek 返回值完全一致。
export class MockEnglishAnalysisService implements EnglishAnalysisService {
  async analyze(input: EnglishAnalysisInput): Promise<EnglishAIAnalysis> {
    const stamp = new Date().toISOString();
    const summaryWords = words(input.summary);
    const articleWords = new Set(words(input.article.content.toLowerCase().replace(/[^\w\s']/g, "")));
    const overlap = summaryWords.filter((word) => articleWords.has(word.toLowerCase().replace(/[^\w']/g, ""))).length;
    const connectors = ["although", "however", "therefore", "because", "while", "first", "finally"];
    const connectorCount = connectors.filter((word) => input.summary.toLowerCase().includes(word)).length;
    const sentences = input.summary.split(/[.!?]+/).filter((sentence) => sentence.trim().length > 4);
    const mistakes: EnglishMistake[] = [];

    if (/\bi\b/.test(input.summary)) {
      mistakes.push({ original: "i", correction: "I", reason: "英语中的第一人称代词 I 必须大写。" });
    }
    if (/\bpeople is\b/i.test(input.summary)) {
      mistakes.push({ original: "people is", correction: "people are", reason: "People 是复数名词，需要搭配 are。" });
    }
    if (/\bexercise make\b/i.test(input.summary)) {
      mistakes.push({ original: "exercise make", correction: "exercise makes", reason: "单数主语 exercise 后的动词需要使用第三人称单数形式。" });
    }
    if (sentences.length && !/^[A-Z]/.test(input.summary.trim())) {
      mistakes.push({ original: input.summary.trim().slice(0, 20), correction: "以大写字母开始句子", reason: "完整英文句子应以大写字母开头。" });
    }

    const lengthFit = summaryWords.length >= 100 && summaryWords.length <= 200 ? 18 : Math.max(5, 18 - Math.abs(140 - summaryWords.length) / 8);
    const contentScore = clamp(58 + Math.min(26, overlap / Math.max(1, summaryWords.length) * 120) + lengthFit);
    const grammarScore = clamp(88 - mistakes.length * 9 + Math.min(5, sentences.length));
    const vocabularyScore = clamp(68 + Math.min(18, new Set(summaryWords.map((word) => word.toLowerCase())).size / Math.max(1, summaryWords.length) * 30) + connectorCount * 3);
    const structureScore = clamp(64 + Math.min(18, sentences.length * 3) + connectorCount * 5);
    const score = clamp(contentScore * 0.35 + grammarScore * 0.25 + vocabularyScore * 0.2 + structureScore * 0.2);
    const weakPoints = [
      grammarScore < 82 ? "语法准确性" : "",
      vocabularyScore < 82 ? "词汇丰富度" : "",
      structureScore < 82 ? "段落结构与连接词" : "",
      contentScore < 82 ? "关键信息覆盖" : "",
    ].filter(Boolean);

    const suggestions = [
      summaryWords.length < 100 ? "把总结扩展到 100–200 词，补充文章的原因、过程和结论。" : "",
      summaryWords.length > 200 ? "删除重复细节，把总结压缩到 200 词以内。" : "",
      connectorCount < 2 ? "使用 however、therefore、although 等连接词增强逻辑。" : "",
      sentences.length < 4 ? "用 4–7 个完整句子组织主要观点，避免把所有信息挤在一个长句中。" : "",
      mistakes.length ? "提交前检查主谓一致、大小写和句末标点。" : "继续保持准确表达，并尝试使用更具体的动词。",
    ].filter(Boolean);

    return {
      id: `analysis-${input.recordId}`,
      userId: input.userId,
      recordId: input.recordId,
      articleId: input.article.id,
      provider: "mock",
      score,
      contentScore,
      grammarScore,
      vocabularyScore,
      structureScore,
      mistakes,
      suggestions,
      improvedSummary: buildImprovedSummary(input.article),
      weakPoints,
      createdAt: stamp,
      updatedAt: stamp,
    };
  }
}

// 参考总结来自文章本身而非用户原文，避免 Mock 服务伪造逐句改写。
const buildImprovedSummary = (article: EnglishArticle) => {
  const sentences = article.content.match(/[^.!?]+[.!?]+/g) ?? [article.content];
  return sentences.slice(0, 3).map((sentence) => sentence.trim()).join(" ");
};

export const englishAnalysisService: EnglishAnalysisService = new MockEnglishAnalysisService();
