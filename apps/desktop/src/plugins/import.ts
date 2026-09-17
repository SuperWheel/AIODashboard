import type {
  PluginImportCheck,
  PluginImportPreview,
  PluginImportSourceKind,
  PluginInstallResult,
} from "./types";

interface ImportGateway {
  pluginPickImportSource(kind: PluginImportSourceKind): Promise<string | null>;
  pluginPreviewImport(source: string): Promise<PluginImportPreview>;
  pluginImport(
    source: string,
    check: PluginImportCheck,
  ): Promise<PluginInstallResult>;
}

interface ImportState {
  phase:
    | "idle"
    | "picking"
    | "checking"
    | "ready"
    | "installing"
    | "success"
    | "error";
  source: string | null;
  preview: PluginImportPreview | null;
  result: PluginInstallResult | null;
  error: string | null;
}

const initial = (): ImportState => ({
  phase: "idle",
  source: null,
  preview: null,
  result: null,
  error: null,
});

/** 导入弹窗的异步交互状态；包内容、版本和授权规则全部由 Core 决定。 */
export class PluginImportFlow {
  private state = initial();
  private generation = 0;
  private listeners = new Set<() => void>();

  constructor(
    private gateway: ImportGateway,
    private onInstalled: (result: PluginInstallResult) => void,
  ) {}

  getSnapshot = () => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };

  private update(state: ImportState) {
    this.state = state;
    this.listeners.forEach((listener) => listener());
  }

  cancel() {
    if (this.state.phase === "installing") return false;
    this.generation++;
    this.update(initial());
    return true;
  }

  async choose(kind: PluginImportSourceKind) {
    if (["picking", "installing", "success"].includes(this.state.phase)) return;
    const generation = ++this.generation;
    this.update({ ...initial(), phase: "picking" });
    try {
      const source = await this.gateway.pluginPickImportSource(kind);
      if (generation !== this.generation) return;
      if (source === null) {
        this.update(initial());
        return;
      }
      await this.inspect(source, generation);
    } catch (e) {
      if (generation === this.generation)
        this.update({ ...initial(), phase: "error", error: String(e) });
    }
  }

  async recheck() {
    if (
      !this.state.source ||
      ["picking", "installing", "success"].includes(this.state.phase)
    )
      return;
    await this.inspect(this.state.source, ++this.generation);
  }

  private async inspect(source: string, generation: number) {
    this.update({ ...initial(), phase: "checking", source });
    try {
      const preview = await this.gateway.pluginPreviewImport(source);
      if (generation === this.generation)
        this.update({ ...initial(), phase: "ready", source, preview });
    } catch (e) {
      if (generation === this.generation)
        this.update({ ...initial(), phase: "error", source, error: String(e) });
    }
  }

  async commit() {
    const { phase, source, preview } = this.state;
    if (phase !== "ready" || !source || !preview) return;
    this.update({ ...this.state, phase: "installing", error: null });
    let result: PluginInstallResult;
    try {
      result = await this.gateway.pluginImport(source, preview.check);
    } catch (e) {
      // 安装失败后必须重新预检，不能反复提交可能过期的确认凭据。
      this.update({ ...initial(), phase: "error", source, error: String(e) });
      return;
    }
    this.update({ ...this.state, phase: "success", result });
    this.onInstalled(result);
  }
}
