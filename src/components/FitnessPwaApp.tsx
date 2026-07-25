"use client";

import { useRef, useState } from "react";
import { Check, FileSpreadsheet, ImagePlus, LoaderCircle, Send, ShieldCheck, UploadCloud, WifiOff } from "lucide-react";

type UploadKind = "training" | "bill";
type UploadResult = {
  kind: UploadKind;
  filename: string;
  detail: string;
  completedAt: string;
};

const readError = async (response: Response, fallback: string) => {
  try {
    const payload = await response.json() as { error?: string };
    return payload.error ?? fallback;
  } catch {
    return fallback;
  }
};

// 手机端只负责选择与上传文件；不读取训练库、不保存业务数据，也不做本地解析。
export default function FitnessPwaApp() {
  const trainingInput = useRef<HTMLInputElement>(null);
  const billInput = useRef<HTMLInputElement>(null);
  const [uploading, setUploading] = useState<UploadKind | null>(null);
  const [message, setMessage] = useState("");
  const [results, setResults] = useState<UploadResult[]>([]);

  const uploadTraining = async (file?: File) => {
    if (!file) return;
    setUploading("training");
    setMessage("");
    try {
      const form = new FormData();
      form.set("image", file);
      const response = await fetch("/api/xunji/parse", { method: "POST", body: form });
      if (!response.ok) throw new Error(await readError(response, "训练数据解析失败"));
      const payload = await response.json() as { workout?: { title?: string; exercises?: unknown[] } };
      setResults((items) => [{
        kind: "training",
        filename: file.name,
        detail: `电脑已解析${payload.workout?.title ? `“${payload.workout.title}”` : "训练"}，等待电脑端确认`,
        completedAt: new Date().toISOString(),
      }, ...items]);
      setMessage("训练数据已发送到电脑，并进入待确认队列。");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "上传失败，请确认手机与电脑在同一网络。");
    } finally {
      setUploading(null);
      if (trainingInput.current) trainingInput.current.value = "";
    }
  };

  const uploadBills = async (files: FileList | null) => {
    if (!files?.length) return;
    setUploading("bill");
    setMessage("");
    try {
      for (const file of Array.from(files)) {
        const form = new FormData();
        form.set("kind", "bill");
        form.set("file", file);
        const response = await fetch("/api/imports", { method: "POST", body: form });
        if (!response.ok) throw new Error(await readError(response, "账单上传失败"));
        setResults((items) => [{
          kind: "bill",
          filename: file.name,
          detail: "已发送到电脑账单导入队列",
          completedAt: new Date().toISOString(),
        }, ...items]);
      }
      setMessage(`${files.length} 个账单文件已发送到电脑。`);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "上传失败，请确认手机与电脑在同一网络。");
    } finally {
      setUploading(null);
      if (billInput.current) billInput.current.value = "";
    }
  };

  return <main className="fit-import-app">
    <header className="fit-import-header">
      <span className="fit-import-mark">LT</span>
      <div><small>手机数据入口</small><h1>发送到电脑</h1></div>
      <i><ShieldCheck /></i>
    </header>

    <section className="fit-import-hero">
      <span><UploadCloud /> 电脑端解析</span>
      <h2>手机只负责上传，<br />数据处理交给电脑。</h2>
      <p>这里不保存训练、账单或动作数据。请保持手机与电脑处于同一网络。</p>
    </section>

    <section className="fit-import-actions">
      <button disabled={uploading !== null} onClick={() => trainingInput.current?.click()}>
        <i className="training">{uploading === "training" ? <LoaderCircle className="spinning" /> : <ImagePlus />}</i>
        <span><b>导入训练数据</b><small>上传训记分享图片，电脑读取二维码并解析</small></span>
        <em>{uploading === "training" ? "处理中" : "选择图片"} <Send /></em>
      </button>
      <input ref={trainingInput} hidden type="file" accept="image/jpeg,image/png,image/webp,image/bmp" onChange={(event) => void uploadTraining(event.target.files?.[0])} />

      <button disabled={uploading !== null} onClick={() => billInput.current?.click()}>
        <i className="bill">{uploading === "bill" ? <LoaderCircle className="spinning" /> : <FileSpreadsheet />}</i>
        <span><b>导入账单数据</b><small>上传微信 Excel 或 CSV，电脑端解析与确认</small></span>
        <em>{uploading === "bill" ? "上传中" : "选择文件"} <Send /></em>
      </button>
      <input ref={billInput} hidden type="file" multiple accept=".xlsx,.csv,text/csv,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" onChange={(event) => void uploadBills(event.target.files)} />
    </section>

    {message && <p className="fit-import-message" role="status">{message}</p>}

    <section className="fit-import-history">
      <header><div><small>本次打开期间</small><h2>发送记录</h2></div><span>{results.length}</span></header>
      {results.map((result, index) => <article key={`${result.completedAt}-${index}`}>
        <i><Check /></i><div><b>{result.filename}</b><small>{result.detail}</small></div>
        <time>{new Date(result.completedAt).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" })}</time>
      </article>)}
      {!results.length && <div className="fit-import-empty"><WifiOff /><p>选择文件后，发送状态会显示在这里。</p></div>}
    </section>

    <footer>文件解析和最终入库都在电脑端完成</footer>
  </main>;
}
