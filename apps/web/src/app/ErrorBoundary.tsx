import { Component, type ErrorInfo, type PropsWithChildren, type ReactNode } from "react";
import { Button, Card, CardContent } from "../components/ui";

interface State {
  error: Error | null;
}

export class ErrorBoundary extends Component<PropsWithChildren, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("[LifeTrace] renderer.error_boundary", error, info);
  }

  render(): ReactNode {
    if (!this.state.error) return this.props.children;
    return (
      <main className="flex min-h-screen items-center justify-center bg-background px-4 py-10">
        <Card className="w-full max-w-lg">
          <CardContent className="pt-5">
            <div className="eyebrow">Application Error</div>
            <h1 className="mt-2 text-xl font-semibold">页面发生了未处理错误</h1>
            <p className="mt-2 text-sm leading-6 text-muted-foreground">LifeTrace 已阻止错误继续扩散。刷新后会重新从 Cloud 加载当前状态。</p>
            <pre className="mt-4 max-h-40 overflow-auto rounded-md bg-muted p-3 text-xs text-destructive">{this.state.error.message}</pre>
            <div className="mt-4 flex gap-2">
              <Button onClick={() => window.location.reload()}>重新加载</Button>
              <Button variant="outline" onClick={() => { this.setState({ error: null }); window.history.replaceState(null, "", "/app/today"); window.location.reload(); }}>返回今日</Button>
            </div>
          </CardContent>
        </Card>
      </main>
    );
  }
}
