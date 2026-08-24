import { useCallback, useEffect, useState } from "react";
import QRCodeGenerator from "qrcode";
import {
  Copy,
  LoaderCircle,
  QrCode,
  Smartphone,
  Wifi,
  WifiOff,
} from "lucide-react";
import { notify } from "@/src/ui/feedback/toastBus";

type MobileUploadState = {
  available: boolean;
  active: boolean;
  managed: boolean;
  port: number;
  urls: string[];
};

export default function MobileUploadControl({
  context,
}: {
  context: "fitness" | "bills";
}) {
  const [mobileUpload, setMobileUpload] = useState<MobileUploadState | null>(
    null,
  );
  const [mobileUploadBusy, setMobileUploadBusy] = useState(false);
  const [mobileUploadError, setMobileUploadError] = useState("");
  const [selectedMobileUrl, setSelectedMobileUrl] = useState("");
  const [mobileUploadQr, setMobileUploadQr] = useState("");

  const applyMobileUploadStatus = useCallback((status: MobileUploadState) => {
    setMobileUpload(status);
    if (!status.active) setMobileUploadQr("");
    setSelectedMobileUrl((current) =>
      status.active && status.urls.length
        ? status.urls.includes(current)
          ? current
          : status.urls[0]
        : "",
    );
  }, []);

  const loadMobileUpload = useCallback(async () => {
    if (!window.mobileUploadApi) return;
    const result = await window.mobileUploadApi.status();
    if (result.status) applyMobileUploadStatus(result.status);
    if (!result.ok) setMobileUploadError(result.error ?? "无法读取手机上传状态");
  }, [applyMobileUploadStatus]);

  useEffect(() => {
    const timer = window.setTimeout(() => void loadMobileUpload(), 0);
    return () => window.clearTimeout(timer);
  }, [loadMobileUpload]);

  useEffect(() => {
    if (!mobileUpload?.active) return;
    const timer = window.setInterval(() => void loadMobileUpload(), 5000);
    return () => window.clearInterval(timer);
  }, [loadMobileUpload, mobileUpload?.active]);

  useEffect(() => {
    let cancelled = false;
    if (!mobileUpload?.active || !selectedMobileUrl) {
      setMobileUploadQr("");
      return;
    }
    QRCodeGenerator.toDataURL(selectedMobileUrl, {
      width: 168,
      margin: 2,
      errorCorrectionLevel: "M",
      color: { dark: "#1f6f56", light: "#ffffff" },
    })
      .then((value) => {
        if (!cancelled) setMobileUploadQr(value);
      })
      .catch(() => {
        if (!cancelled) setMobileUploadQr("");
      });
    return () => {
      cancelled = true;
    };
  }, [mobileUpload?.active, selectedMobileUrl]);

  const toggleMobileUpload = async () => {
    if (!window.mobileUploadApi || !mobileUpload) return;
    setMobileUploadBusy(true);
    setMobileUploadError("");
    try {
      const result = mobileUpload.active
        ? await window.mobileUploadApi.stop()
        : await window.mobileUploadApi.start();
      if (!result.ok) throw new Error(result.error ?? "手机上传入口操作失败");
      if (result.status) applyMobileUploadStatus(result.status);
    } catch (error) {
      setMobileUploadError(
        error instanceof Error ? error.message : "手机上传入口操作失败",
      );
    } finally {
      setMobileUploadBusy(false);
    }
  };

  if (!mobileUpload) return null;

  const purpose = context === "fitness" ? "训练图片" : "账单文件";

  return (
    <section
      className={`hx-mobile-upload-control ${mobileUpload.active ? "active" : ""}`}
      aria-label={`手机上传${purpose}`}
    >
      <span className="hx-mobile-upload-icon">
        {mobileUpload.active ? <Wifi /> : <WifiOff />}
      </span>
      <div className="hx-mobile-upload-copy">
        <strong>
          {mobileUpload.active ? "手机上传已开放" : "手机上传当前关闭"}
        </strong>
        <p>
          {mobileUpload.active
            ? `现在可以从手机上传${purpose}；完成后请及时关闭。`
            : `需要上传${purpose}时临时开启，训练与账单共用同一个安全入口。`}
        </p>
        {mobileUpload.active && selectedMobileUrl ? (
          <div className="hx-mobile-upload-address">
            <Smartphone />
            <code>{selectedMobileUrl}</code>
            <button
              type="button"
              aria-label="复制手机访问地址"
              onClick={() =>
                void navigator.clipboard
                  ?.writeText(selectedMobileUrl)
                  .then(() => notify("手机访问地址已复制"))
              }
            >
              <Copy />
            </button>
          </div>
        ) : null}
        {mobileUpload.active && mobileUpload.urls.length > 1 ? (
          <label className="hx-mobile-upload-select">
            <span>备用地址</span>
            <select
              value={selectedMobileUrl}
              onChange={(event) => setSelectedMobileUrl(event.target.value)}
              aria-label="选择手机访问地址"
            >
              {mobileUpload.urls.map((url, index) => (
                <option key={url} value={url}>
                  {index === 0 ? "推荐地址" : "备用地址"} {index + 1} ·{" "}
                  {url.replace(/^https:\/\//, "")}
                </option>
              ))}
            </select>
          </label>
        ) : null}
        {mobileUploadError ? (
          <small role="alert">{mobileUploadError}</small>
        ) : null}
      </div>
      {mobileUpload.active && selectedMobileUrl ? (
        <div className="hx-mobile-upload-qr" aria-label="手机扫码访问地址">
          {mobileUploadQr ? (
            <i
              className="hx-mobile-upload-qr-image"
              role="img"
              aria-label="手机上传地址二维码"
              style={{ backgroundImage: `url(${mobileUploadQr})` }}
            />
          ) : (
            <QrCode aria-hidden="true" />
          )}
          <span>
            <QrCode />手机扫码打开
          </span>
        </div>
      ) : null}
      <button
        type="button"
        className={`hx-btn ${mobileUpload.active ? "secondary" : "primary"}`}
        disabled={mobileUploadBusy || !mobileUpload.available}
        onClick={() => void toggleMobileUpload()}
      >
        {mobileUploadBusy ? (
          <>
            <LoaderCircle className="spin" />
            正在处理…
          </>
        ) : mobileUpload.active ? (
          "关闭手机上传"
        ) : (
          "开启手机上传"
        )}
      </button>
    </section>
  );
}
