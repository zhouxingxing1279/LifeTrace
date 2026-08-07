export type ToastPayload = {
  message: string;
  type: "success" | "info" | "warning" | "error";
  duration: number;
};

/** 全局轻提示：不永久占据页面，几秒后自动消失。 */
export function notify(message: string, type: ToastPayload["type"] = "success") {
  window.dispatchEvent(
    new CustomEvent("hengxu-toast", {
      detail: { message, type, duration: type === "error" ? 4500 : 2500 },
    }),
  );
}
