"use client";

import { useEffect, useMemo, useState } from "react";
import { BookPlus, Volume2, X } from "lucide-react";
import type { DictionaryLookup, EnglishArticle, UserVocabulary, VocabularySettings } from "@/src/types/english";
import { speakEnglish, stopSpeech } from "@/src/services/pronunciation";

export function DictionaryPopover({ lookup, article, settings, onClose, onAdded, onMessage }: {
  lookup: DictionaryLookup;
  article: EnglishArticle;
  settings: VocabularySettings;
  onClose: () => void;
  onAdded: (item: UserVocabulary) => void;
  onMessage: (message: string) => void;
}) {
  const meanings = useMemo(() => lookup.partsOfSpeech?.flatMap((part) => part.translation) ?? [], [lookup]);
  const [selected, setSelected] = useState<string[]>(settings.defaultFirstMeaning && meanings[0] ? [meanings[0]] : []);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    const close = (event: KeyboardEvent) => event.key === "Escape" && onClose();
    window.addEventListener("keydown", close);
    return () => { window.removeEventListener("keydown", close); stopSpeech(); };
  }, [onClose]);
  useEffect(() => {
    if (settings.autoPronounce) void speakEnglish(lookup.queryWord, "word", settings)
      .catch((error) => onMessage(error instanceof Error ? error.message : "朗读失败"));
  }, [lookup.queryWord, onMessage, settings]);

  const speak = async (text: string, kind: "word" | "sentence") => {
    try { await speakEnglish(text, kind, settings); }
    catch (error) { onMessage(error instanceof Error ? error.message : "朗读失败"); }
  };
  const toggle = (meaning: string) => setSelected((items) => items.includes(meaning) ? items.filter((item) => item !== meaning) : [...items, meaning]);
  const save = async () => {
    const chosen = selected.length ? selected : meanings.slice(0, 1);
    if (!chosen.length) return onMessage("词典中没有可保存的中文释义");
    setSaving(true);
    try {
      const response = await fetch("/api/english/vocabulary", {
        method: "POST", headers: { "content-type": "application/json" },
        body: JSON.stringify({
          word: lookup.queryWord, normalizedWord: lookup.normalizedWord, lemma: lookup.lemma ?? lookup.normalizedWord,
          dictionaryWordId: lookup.dictionaryWordId, phonetic: lookup.phonetic, selectedMeanings: chosen,
          partOfSpeech: lookup.partsOfSpeech?.[0]?.type, sourceArticleId: article.id, sourceArticleTitle: article.title,
          sourceSentence: lookup.sourceSentence, frequencyRank: lookup.frequencyRank, tags: lookup.tags,
        }),
      });
      const payload = await response.json() as UserVocabulary & { error?: string };
      if (!response.ok) throw new Error(payload.error || "加入生词本失败");
      onAdded(payload); onMessage(`“${payload.word}”已加入生词本`); onClose();
    } catch (error) { onMessage(error instanceof Error ? error.message : "加入生词本失败"); }
    finally { setSaving(false); }
  };

  return <div className="en-dictionary-popover" role="dialog" aria-modal="false" aria-label={`${lookup.queryWord} 查词结果`} onMouseDown={(event) => event.stopPropagation()}>
    <button className="en-icon-button en-dictionary-close" aria-label="关闭查词卡片" onClick={onClose}><X /></button>
    <header><span>OFFLINE DICTIONARY</span><h3>{lookup.queryWord}</h3>
      {lookup.lemma && lookup.lemma !== lookup.normalizedWord && <small>原形：{lookup.lemma}</small>}
      <div><strong>/{lookup.phonetic || "暂无音标"}/</strong><button onClick={() => void speak(lookup.queryWord, "word")}><Volume2 />播放单词</button></div>
    </header>
    {!lookup.found ? <p className="en-dictionary-empty">本地词典暂未收录这个词。导入完整 ECDICT 后可扩充词库。</p> : <>
      <div className="en-dictionary-tags">
        {lookup.partsOfSpeech?.map((part) => <b key={part.type}>{part.type}</b>)}
        {lookup.oxford && <span>牛津核心</span>}{lookup.tags?.map((tag) => <span key={tag}>{tag.toUpperCase()}</span>)}
      </div>
      <fieldset><legend>选择需要记忆的释义</legend>
        {meanings.map((meaning, index) => <label key={`${meaning}-${index}`}>
          <input type="checkbox" checked={selected.includes(meaning)} onChange={() => toggle(meaning)} /><span>{meaning}</span>
        </label>)}
        {!meanings.length && <p>暂无中文释义</p>}
      </fieldset>
      {lookup.partsOfSpeech?.flatMap((part) => part.definition).slice(0, 2).map((definition) => <p className="en-dictionary-definition" key={definition}>{definition}</p>)}
      <button className="primary" disabled={saving || !meanings.length} onClick={() => void save()}><BookPlus />{saving ? "正在加入…" : selected.length ? `加入生词本（${selected.length}）` : "以第一条释义加入"}</button>
    </>}
  </div>;
}
