"use client";

import { useEffect } from "react";
import { X } from "lucide-react";

type ToastProps = {
  message: string;
  kind?: "info" | "error";
  autoDismissMs?: number;
  onClose: () => void;
};

/** 右下角轻提示，几秒后自动消失，也可手动关闭。 */
export default function Toast({ message, kind = "info", autoDismissMs = 4000, onClose }: ToastProps) {
  useEffect(() => {
    if (!autoDismissMs) return;
    const timer = window.setTimeout(onClose, autoDismissMs);
    return () => window.clearTimeout(timer);
  }, [message, autoDismissMs, onClose]);

  return (
    <div className={`app-toast ${kind}`} role="status" aria-live="polite">
      <span>{message}</span>
      <button type="button" aria-label="关闭提示" onClick={onClose}>
        <X />
      </button>
    </div>
  );
}
