import { useEffect, useState } from "react";
import { ArrowLeft, Database, FileUp, Images, NotebookPen, ShieldCheck } from "lucide-react";
import PhotoSyncModule from "@/src/components/PhotoSyncModule";
import NotesModule from "@/src/components/NotesModule";
import ImportBills from "@/src/components/feature/finance/ImportBills";
import { useLifeStore } from "@/src/stores/useLifeStore";

type LocalTool = "photos" | "import" | "notes";

const TOOLS: Array<{ id: LocalTool; label: string; description: string; icon: typeof Images }> = [
  { id: "photos", label: "照片与私密相册", description: "本机照片同步、导入与加密私密相册", icon: Images },
  { id: "import", label: "文件与账单导入", description: "从本机文件或手机上传入口导入训练和账单", icon: FileUp },
  { id: "notes", label: "本地笔记", description: "直接访问 SQLite 中的笔记与本地内容", icon: NotebookPen },
];

export default function DesktopLocalToolsCenter({ onClose }: { onClose: () => void }) {
  const ready = useLifeStore((state) => state.ready);
  const storageError = useLifeStore((state) => state.storageError);
  const initialize = useLifeStore((state) => state.initialize);
  const [tool, setTool] = useState<LocalTool>("photos");

  useEffect(() => {
    if (!ready && !storageError) void initialize();
  }, [initialize, ready, storageError]);

  if (!ready && !storageError) {
    return <div className="lt-local-tools-loading"><Database/><span>正在连接本机 SQLite…</span></div>;
  }

  if (storageError) {
    return (
      <div className="lt-local-tools-loading error">
        <Database/>
        <strong>本机数据暂时无法连接</strong>
        <span>{storageError}</span>
        <button type="button" onClick={() => void initialize()}>重新连接</button>
      </div>
    );
  }

  return (
    <section className="lt-local-tools-center">
      <header className="lt-local-tools-head">
        <div>
          <button type="button" className="lt-local-tools-back" onClick={onClose}><ArrowLeft/>返回工作台</button>
          <h1>本机工具</h1>
          <p>这里放桌面端独有的本地能力，不再作为另一套应用导航存在。</p>
        </div>
        <div className="lt-local-tools-status"><ShieldCheck/><span><strong>本机数据</strong><small>SQLite / 文件系统</small></span></div>
      </header>

      <div className="lt-local-tools-layout">
        <nav className="lt-local-tools-nav" aria-label="本机工具">
          {TOOLS.map((item) => {
            const Icon = item.icon;
            return (
              <button
                key={item.id}
                type="button"
                className={tool === item.id ? "active" : ""}
                aria-current={tool === item.id ? "page" : undefined}
                onClick={() => setTool(item.id)}
              >
                <Icon/>
                <span><strong>{item.label}</strong><small>{item.description}</small></span>
              </button>
            );
          })}
        </nav>

        <div className="lt-local-tools-content">
          {tool === "photos" ? <PhotoSyncModule/> : null}
          {tool === "import" ? <ImportBills/> : null}
          {tool === "notes" ? <NotesModule/> : null}
        </div>
      </div>
    </section>
  );
}
