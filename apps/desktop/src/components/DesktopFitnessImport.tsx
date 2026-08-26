import { useEffect } from "react";
import { Database } from "lucide-react";
import XunjiImportPanel from "@/src/components/XunjiImportPanel";
import { useLifeStore } from "@/src/stores/useLifeStore";

export default function DesktopFitnessImport() {
  const ready = useLifeStore((state) => state.ready);
  const storageError = useLifeStore((state) => state.storageError);
  const initialize = useLifeStore((state) => state.initialize);

  useEffect(() => {
    if (!ready && !storageError) void initialize();
  }, [initialize, ready, storageError]);

  if (!ready && !storageError) {
    return <div className="lt-local-tools-loading"><Database /><span>正在连接本机训练数据…</span></div>;
  }

  if (storageError) {
    return (
      <div className="lt-local-tools-loading error">
        <Database />
        <strong>本机训练数据暂时无法连接</strong>
        <span>{storageError}</span>
        <button type="button" onClick={() => void initialize()}>重新连接</button>
      </div>
    );
  }

  return (
    <section className="lt-desktop-fitness-import" aria-label="导入健身数据">
      <header>
        <div><span>桌面导入</span><h2>导入健身数据</h2></div>
        <p>上传训记训练截图，解析确认后写入本机训练记录。</p>
      </header>
      <XunjiImportPanel />
    </section>
  );
}
