import { Component, type ReactNode } from "react";

interface Props {
  /** 用于日志与兜底下提示的插件标识（view key / card id） */
  name: string;
  children: ReactNode;
}

interface State {
  error: string | null;
}

/**
 * 插件渲染错误边界：插件视图/卡片抛错（渲染或 effect 同步抛错）时只降级这一块，
 * 不卸载整个面板——与 onload 看门狗同属「坏插件不炸宿主」的防线。
 */
export default class PluginErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(e: unknown): State {
    return { error: e instanceof Error ? e.message : String(e) };
  }

  componentDidCatch(e: unknown) {
    console.error(`[plugins] 渲染失败: ${this.props.name}`, e);
  }

  render() {
    if (this.state.error !== null) {
      return (
        <div className="rounded-2xl border border-danger/30 bg-danger/5 p-4">
          <div className="text-xs font-medium text-danger">
            插件「{this.props.name}」渲染失败
          </div>
          <div className="mt-1 break-all text-[11px] text-danger/80">{this.state.error}</div>
          <div className="mt-1 text-[11px] text-ink3">
            可在「插件」页停用或重载后重试
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}
