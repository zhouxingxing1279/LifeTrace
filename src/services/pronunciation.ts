import type { VocabularySettings } from "@/src/types/english";

export type SpeechKind = "word" | "sentence";

export function speakEnglish(text: string, kind: SpeechKind, settings: VocabularySettings) {
  if (typeof window === "undefined" || !("speechSynthesis" in window)) {
    throw new Error("当前系统没有可用的英文朗读功能");
  }
  window.speechSynthesis.cancel();
  const utterance = new SpeechSynthesisUtterance(text);
  const voices = window.speechSynthesis.getVoices();
  utterance.voice = voices.find((voice) => voice.lang.toLowerCase() === settings.preferredAccent.toLowerCase())
    ?? voices.find((voice) => voice.lang.toLowerCase().startsWith("en"))
    ?? null;
  utterance.lang = utterance.voice?.lang ?? settings.preferredAccent;
  utterance.rate = kind === "word" ? settings.wordSpeechRate : settings.sentenceSpeechRate;
  return new Promise<void>((resolve, reject) => {
    utterance.onend = () => resolve();
    utterance.onerror = () => reject(new Error("朗读失败，请检查 Windows 英语语音设置"));
    window.speechSynthesis.speak(utterance);
  });
}

export function stopSpeech() {
  if (typeof window !== "undefined" && "speechSynthesis" in window) window.speechSynthesis.cancel();
}
