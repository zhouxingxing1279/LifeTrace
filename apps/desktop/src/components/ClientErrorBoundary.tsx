import { Component, type ErrorInfo, type ReactNode } from "react";
import { clientLogger, createErrorId } from "../services/clientObservability";

interface ClientErrorBoundaryProps {
  children: ReactNode;
}

interface ClientErrorBoundaryState {
  failed: boolean;
  errorId: string | null;
}

export default class ClientErrorBoundary extends Component<
  ClientErrorBoundaryProps,
  ClientErrorBoundaryState
> {
  state: ClientErrorBoundaryState = {
    failed: false,
    errorId: null,
  };

  static getDerivedStateFromError(): ClientErrorBoundaryState {
    return {
      failed: true,
      errorId: createErrorId(),
    };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    clientLogger.fatal(
      "react.render.failed",
      {
        errorId: this.state.errorId,
        componentStack: info.componentStack,
        route: typeof window !== "undefined" ? window.location.pathname : undefined,
      },
      error,
    );
  }

  render(): ReactNode {
    if (!this.state.failed) return this.props.children;

    return (
      <main className="hx-loading" role="alert">
        <span>!</span>
        <h1>LifeTrace 界面发生错误</h1>
        <p>错误已写入诊断日志，请重新加载应用。</p>
        {this.state.errorId ? <p>错误编号：{this.state.errorId}</p> : null}
        <button className="hx-btn primary" type="button" onClick={() => window.location.reload()}>
          重新加载
        </button>
      </main>
    );
  }
}
