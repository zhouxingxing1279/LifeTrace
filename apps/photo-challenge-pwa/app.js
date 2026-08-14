const STORAGE_KEY = "lifetrace.photoChallenge.accessKey";
const MAX_ORIGINAL_BYTES = 64 * 1024 * 1024;

const elements = {
  keyPanel: document.querySelector("#key-panel"),
  uploadPanel: document.querySelector("#upload-panel"),
  accessKey: document.querySelector("#access-key"),
  enterButton: document.querySelector("#enter-button"),
  cameraButton: document.querySelector("#camera-button"),
  libraryButton: document.querySelector("#library-button"),
  cameraInput: document.querySelector("#camera-input"),
  libraryInput: document.querySelector("#library-input"),
  previewWrap: document.querySelector("#preview-wrap"),
  previewImage: document.querySelector("#preview-image"),
  fileName: document.querySelector("#file-name"),
  fileSize: document.querySelector("#file-size"),
  clearButton: document.querySelector("#clear-button"),
  scoreButton: document.querySelector("#score-button"),
  scoreButtonIcon: document.querySelector("#score-button-icon"),
  scoreButtonLabel: document.querySelector("#score-button-label"),
  resultPanel: document.querySelector("#result-panel"),
  resultTitle: document.querySelector("#result-title"),
  scoreValue: document.querySelector("#score-value"),
  feedback: document.querySelector("#feedback"),
  scoreComposition: document.querySelector("#score-composition"),
  scoreLight: document.querySelector("#score-light"),
  scoreStory: document.querySelector("#score-story"),
  scoreTechnical: document.querySelector("#score-technical"),
  scoreOriginality: document.querySelector("#score-originality"),
  message: document.querySelector("#message"),
  highCount: document.querySelector("#high-count"),
  targetCount: document.querySelector("#target-count"),
  progressBar: document.querySelector("#progress-bar"),
  progressCopy: document.querySelector("#progress-copy"),
};

let selectedFile = null;
let selectedPreviewUrl = "";
let busy = false;

bootstrap();

function bootstrap() {
  if ("serviceWorker" in navigator) {
    navigator.serviceWorker.register("./sw.js", { scope: "./" }).catch(() => undefined);
  }
  elements.enterButton.addEventListener("click", enterChallenge);
  elements.accessKey.addEventListener("keydown", (event) => {
    if (event.key === "Enter") enterChallenge();
  });
  elements.cameraButton.addEventListener("click", () => elements.cameraInput.click());
  elements.libraryButton.addEventListener("click", () => elements.libraryInput.click());
  elements.cameraInput.addEventListener("change", () => chooseFile(elements.cameraInput.files?.[0]));
  elements.libraryInput.addEventListener("change", () => chooseFile(elements.libraryInput.files?.[0]));
  elements.clearButton.addEventListener("click", clearSelection);
  elements.scoreButton.addEventListener("click", submitPhoto);

  const savedKey = localStorage.getItem(STORAGE_KEY) || "";
  elements.accessKey.value = savedKey;
  if (savedKey) {
    loadStats(savedKey).then((stats) => {
      showChallenge(stats);
    }).catch(() => {
      localStorage.removeItem(STORAGE_KEY);
    });
  }
}

async function enterChallenge() {
  if (busy) return;
  const key = elements.accessKey.value.trim();
  if (!key) {
    showMessage("请输入挑战口令。", true);
    return;
  }
  setBusy(true, "正在验证…");
  try {
    const stats = await loadStats(key);
    localStorage.setItem(STORAGE_KEY, key);
    showChallenge(stats);
    hideMessage();
  } catch (error) {
    showMessage(readError(error, "挑战口令无效。"), true);
  } finally {
    setBusy(false);
  }
}

function showChallenge(stats) {
  elements.keyPanel.hidden = true;
  elements.uploadPanel.hidden = false;
  renderStats(stats);
}

function chooseFile(file) {
  hideMessage();
  hideResult();
  if (!file) return;
  if (!/^image\/(jpeg|png|webp)$/.test(file.type)) {
    showMessage("请选择 JPEG、PNG 或 WebP 照片。", true);
    return;
  }
  if (file.size <= 0 || file.size > MAX_ORIGINAL_BYTES) {
    showMessage("原图不能为空且不能超过 64 MiB。", true);
    return;
  }
  selectedFile = file;
  if (selectedPreviewUrl) URL.revokeObjectURL(selectedPreviewUrl);
  selectedPreviewUrl = URL.createObjectURL(file);
  elements.previewImage.src = selectedPreviewUrl;
  elements.fileName.textContent = file.name || "照片";
  elements.fileSize.textContent = formatBytes(file.size);
  elements.previewWrap.hidden = false;
  elements.scoreButton.disabled = false;
}

function clearSelection() {
  selectedFile = null;
  elements.cameraInput.value = "";
  elements.libraryInput.value = "";
  elements.previewWrap.hidden = true;
  elements.scoreButton.disabled = true;
  if (selectedPreviewUrl) {
    URL.revokeObjectURL(selectedPreviewUrl);
    selectedPreviewUrl = "";
  }
  hideResult();
  hideMessage();
}

async function submitPhoto() {
  if (busy || !selectedFile) return;
  const key = localStorage.getItem(STORAGE_KEY) || elements.accessKey.value.trim();
  if (!key) {
    showMessage("挑战口令已失效，请重新进入。", true);
    elements.keyPanel.hidden = false;
    elements.uploadPanel.hidden = true;
    return;
  }
  setBusy(true, "正在上传原图并评分…");
  showMessage("正在生成评分预览。原图保持原始质量，只在 LifeTrace 云端临时中转。", false);
  try {
    const [modelPreview, thumbnail] = await Promise.all([
      resizeImage(selectedFile, 1600, 0.86),
      resizeImage(selectedFile, 360, 0.68),
    ]);
    const form = new FormData();
    form.append("file", selectedFile, selectedFile.name || "photo.jpg");
    form.append("previewDataUrl", modelPreview);
    form.append("thumbnailDataUrl", thumbnail);
    if (selectedFile.lastModified) form.append("capturedAt", new Date(selectedFile.lastModified).toISOString());

    showMessage("原图正在上传到临时中转区，随后由 GLM-4V-Flash 按固定量表评分。", false);
    const response = await fetch("/api/v1/photo-challenge/score", {
      method: "POST",
      headers: { "x-photo-challenge-key": key },
      body: form,
    });
    const payload = await readJson(response);
    if (!response.ok) throw new Error(errorMessage(payload, `评分失败 (${response.status})`));
    renderResult(payload);
    renderStats(payload.stats);
    localStorage.setItem(STORAGE_KEY, key);
    if (payload.duplicate) {
      showMessage("这张原图以前已经提交过，本次不会重复计数。", false);
    } else {
      showMessage("评分完成。原图已进入云端中转队列；LifeTrace 电脑端成功写入本地相册后，云端原图会自动删除。", false);
    }
  } catch (error) {
    showMessage(readError(error, "照片评分失败，请稍后重试。"), true);
  } finally {
    setBusy(false);
  }
}

async function loadStats(key) {
  const response = await fetch("/api/v1/photo-challenge/summary", {
    headers: { "x-photo-challenge-key": key },
  });
  const payload = await readJson(response);
  if (!response.ok) throw new Error(errorMessage(payload, `无法读取挑战进度 (${response.status})`));
  return payload;
}

function renderStats(stats) {
  const high = Number(stats?.highScoreCount || 0);
  const target = Number(stats?.target || 501);
  const remaining = Number(stats?.remaining ?? Math.max(0, target - high));
  elements.highCount.textContent = String(high);
  elements.targetCount.textContent = String(target);
  elements.progressBar.style.width = `${Math.min(100, target ? (high / target) * 100 : 0)}%`;
  elements.progressCopy.textContent = stats?.achieved ? "约定已达成 🎉" : `还差 ${remaining} 张超过 90 分的照片`;
}

function renderResult(result) {
  elements.resultPanel.hidden = false;
  elements.resultPanel.classList.toggle("qualified", Boolean(result.qualified));
  elements.scoreValue.textContent = String(result.score ?? 0);
  elements.resultTitle.textContent = result.qualified ? "🏆 超过 90 分，计入约定" : "✅ 这次还没超过 90 分";
  elements.feedback.textContent = result.feedback || "";
  elements.scoreComposition.textContent = String(result.breakdown?.composition ?? 0);
  elements.scoreLight.textContent = String(result.breakdown?.lightColor ?? 0);
  elements.scoreStory.textContent = String(result.breakdown?.subjectStory ?? 0);
  elements.scoreTechnical.textContent = String(result.breakdown?.technical ?? 0);
  elements.scoreOriginality.textContent = String(result.breakdown?.originality ?? 0);
  elements.resultPanel.scrollIntoView({ behavior: matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth", block: "nearest" });
}

function hideResult() {
  elements.resultPanel.hidden = true;
  elements.resultPanel.classList.remove("qualified");
}

function setBusy(value, label) {
  busy = value;
  elements.enterButton.disabled = value;
  elements.cameraButton.disabled = value;
  elements.libraryButton.disabled = value;
  elements.clearButton.disabled = value;
  elements.scoreButton.disabled = value || !selectedFile;
  elements.scoreButton.classList.toggle("busy", value);
  elements.scoreButtonIcon.textContent = value ? "⏳" : "✨";
  elements.scoreButtonLabel.textContent = value ? (label || "处理中…") : "提交并评分";
}

function showMessage(text, error) {
  elements.message.hidden = false;
  elements.message.textContent = text;
  elements.message.classList.toggle("error", Boolean(error));
}

function hideMessage() {
  elements.message.hidden = true;
  elements.message.textContent = "";
  elements.message.classList.remove("error");
}

async function resizeImage(file, maxEdge, quality) {
  const source = await loadImageSource(file);
  try {
    const width = source.width || source.naturalWidth;
    const height = source.height || source.naturalHeight;
    if (!width || !height) throw new Error("无法读取照片尺寸。");
    const scale = Math.min(1, maxEdge / Math.max(width, height));
    const outputWidth = Math.max(1, Math.round(width * scale));
    const outputHeight = Math.max(1, Math.round(height * scale));
    const canvas = document.createElement("canvas");
    canvas.width = outputWidth;
    canvas.height = outputHeight;
    const context = canvas.getContext("2d");
    if (!context) throw new Error("浏览器无法处理这张照片。");
    context.fillStyle = "#fff";
    context.fillRect(0, 0, outputWidth, outputHeight);
    context.drawImage(source, 0, 0, outputWidth, outputHeight);
    return canvas.toDataURL("image/jpeg", quality);
  } finally {
    if (typeof source.close === "function") source.close();
    if (source.__objectUrl) URL.revokeObjectURL(source.__objectUrl);
  }
}

async function loadImageSource(file) {
  if (typeof createImageBitmap === "function") {
    try { return await createImageBitmap(file, { imageOrientation: "from-image" }); } catch (_) { /* fallback below */ }
  }
  const url = URL.createObjectURL(file);
  const image = new Image();
  image.__objectUrl = url;
  image.decoding = "async";
  image.src = url;
  await image.decode();
  return image;
}

async function readJson(response) {
  const text = await response.text();
  if (!text) return null;
  try { return JSON.parse(text); } catch (_) { return { message: text }; }
}

function errorMessage(payload, fallback) {
  if (!payload || typeof payload !== "object") return fallback;
  if (typeof payload.message === "string" && payload.message.trim()) return payload.message;
  if (payload.error && typeof payload.error === "object" && typeof payload.error.message === "string") return payload.error.message;
  return fallback;
}

function readError(error, fallback) {
  return error instanceof Error && error.message ? error.message : fallback;
}

function formatBytes(bytes) {
  if (bytes < 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024))} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}
