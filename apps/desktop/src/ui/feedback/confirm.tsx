
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { AlertTriangle } from "lucide-react";

export interface ConfirmOptions {
  title: string;
  description: string;
  confirmLabel?: string;
  cancelLabel?: string;
}

interface ConfirmRequest extends ConfirmOptions {
  id: number;
  resolve: (value: boolean) => void;
}

const listeners = new Set<(request: ConfirmRequest) => void>();
let nextId = 1;

export function confirmAction(options: ConfirmOptions): Promise<boolean> {
  if (typeof window === "undefined") return Promise.resolve(false);
  if (listeners.size === 0) return Promise.resolve(window.confirm(`${options.title}\n\n${options.description}`));
  return new Promise((resolve) => {
    const request: ConfirmRequest = { ...options, id: nextId++, resolve };
    listeners.forEach((listener) => listener(request));
  });
}

export function ConfirmDialogHost() {
  const [request, setRequest] = useState<ConfirmRequest | null>(null);
  const cancelRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    const listener = (next: ConfirmRequest) => setRequest(next);
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
    };
  }, []);

  useEffect(() => {
    if (!request) return;
    cancelRef.current?.focus();
    const handler = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        request.resolve(false);
        setRequest(null);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [request]);

  if (!request || typeof document === "undefined") return null;

  const finish = (result: boolean) => {
    request.resolve(result);
    setRequest(null);
  };

  return createPortal(
    <div className="ui-confirm-backdrop" role="presentation" onMouseDown={(event) => {
      if (event.target === event.currentTarget) finish(false);
    }}>
      <section className="ui-confirm-dialog" role="alertdialog" aria-modal="true" aria-labelledby={`confirm-title-${request.id}`}>
        <header>
          <span><AlertTriangle aria-hidden="true" /></span>
          <div>
            <h2 id={`confirm-title-${request.id}`}>{request.title}</h2>
            <p>{request.description}</p>
          </div>
        </header>
        <footer>
          <button ref={cancelRef} type="button" className="hx-btn secondary" onClick={() => finish(false)}>
            {request.cancelLabel ?? "取消"}
          </button>
          <button type="button" className="ui-confirm-danger" onClick={() => finish(true)}>
            {request.confirmLabel ?? "确认"}
          </button>
        </footer>
      </section>
    </div>,
    document.body,
  );
}
