import {
  currentMonitor,
  getCurrentWindow,
  PhysicalPosition,
  PhysicalSize,
} from "@tauri-apps/api/window";

/** 窗口边缘与屏幕可用区域之间保留的间距（物理像素）。 */
const WINDOW_MARGIN = 30;
const MIN_WIDTH = 1200;
const MIN_HEIGHT = 800;

/**
 * 启动时把窗口尺寸与位置收敛到当前显示器的工作区（避开任务栏）。
 * 桌面端“底边超出屏幕”的兜底修复：无论屏幕尺寸如何，都保证窗口完整可见。
 */
export async function fitWindowToWorkArea(): Promise<void> {
  try {
    const appWindow = getCurrentWindow();
    const monitor = await currentMonitor();
    if (!monitor) return;

    const scale = monitor.scaleFactor || 1;
    const work = monitor.workArea; // 物理像素
    const margin = Math.round(WINDOW_MARGIN * scale);
    const outer = await appWindow.outerSize(); // 物理像素
    const targetWidth = Math.min(
      outer.width,
      Math.max(Math.round(MIN_WIDTH * scale), work.size.width - margin * 2),
    );
    const targetHeight = Math.min(
      outer.height,
      Math.max(Math.round(MIN_HEIGHT * scale), work.size.height - margin * 2),
    );

    await appWindow.setSize(new PhysicalSize(targetWidth, targetHeight));
    const fitted = await appWindow.outerSize();
    await appWindow.setPosition(
      new PhysicalPosition(
        work.position.x + Math.round((work.size.width - fitted.width) / 2),
        work.position.y + Math.round((work.size.height - fitted.height) / 2),
      ),
    );
  } catch (error) {
    // 窗口适配属于尽力而为：失败时保持配置尺寸，不影响应用启动。
    console.warn("[LifeTrace] window fit skipped", error);
  }
}
