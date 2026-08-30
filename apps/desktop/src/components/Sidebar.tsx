import {
  Folder,
  Inbox,
  ListChecks,
  Monitor,
  Moon,
  NotebookPen,
  Puzzle,
  Sun,
  type LucideIcon,
} from "lucide-react";
import { useTheme, type ThemeMode } from "../theme";

interface NavItem {
  key: string;
  label: string;
  /** 插件视图传入的字符/emoji 图标；核心视图走 CORE_ICONS */
  icon: string;
}

/** 核心视图的 lucide 图标；插件视图回退到 manifest 注册的字符图标 */
const CORE_ICONS: Record<string, LucideIcon> = {
  today: Sun,
  tasks: ListChecks,
  projects: Folder,
  notes: NotebookPen,
  inbox: Inbox,
  plugins: Puzzle,
};

const THEME_META: Record<ThemeMode, { label: string; Icon: LucideIcon }> = {
  system: { label: "跟随系统", Icon: Monitor },
  light: { label: "浅色", Icon: Sun },
  dark: { label: "深色", Icon: Moon },
};

function NavGlyph({ item }: { item: NavItem }) {
  const Icon = CORE_ICONS[item.key];
  if (Icon) return <Icon size={15} strokeWidth={1.8} className="shrink-0 opacity-90" />;
  return <span className="w-4 shrink-0 text-center opacity-80">{item.icon}</span>;
}

export default function Sidebar({
  items,
  current,
  onNav,
  inboxOpen,
}: {
  items: NavItem[];
  current: string;
  onNav: (v: string) => void;
  inboxOpen: number;
}) {
  const { mode, cycle } = useTheme();
  const themeMeta = THEME_META[mode];

  return (
    <aside className="flex w-52 shrink-0 flex-col border-r border-line bg-surface2/80 px-3 py-5 backdrop-blur-xl">
      <div className="px-2 pb-6">
        <div className="text-[15px] font-bold tracking-wide text-ink">AIODashboard</div>
        <div className="mt-0.5 text-[11px] text-ink3">Local-First · Rust Core</div>
      </div>

      <nav className="flex flex-col gap-1">
        {items.map((item) => {
          const active = current === item.key;
          return (
            <button
              key={item.key}
              onClick={() => onNav(item.key)}
              className={`flex items-center gap-2.5 rounded-lg px-3 py-2 text-left text-sm transition-colors ${
                active
                  ? "bg-accent/10 font-medium text-accent"
                  : "text-ink2 hover:bg-hover hover:text-ink"
              }`}
            >
              <NavGlyph item={item} />
              <span className="flex-1">{item.label}</span>
              {item.key === "inbox" && inboxOpen > 0 && (
                <span className="rounded-full bg-violet/15 px-1.5 py-0.5 text-[10px] tabular-nums text-violet">
                  {inboxOpen}
                </span>
              )}
            </button>
          );
        })}
      </nav>

      <div className="mt-auto flex flex-col gap-1.5 px-2">
        <button
          onClick={cycle}
          title="切换主题（跟随系统 → 浅色 → 深色）"
          className="flex w-full items-center gap-2 rounded-lg border border-line px-3 py-2 text-xs text-ink2 hover:bg-hover hover:text-ink"
        >
          <themeMeta.Icon size={13} strokeWidth={1.8} />
          <span>{themeMeta.label}</span>
        </button>
        <button
          onClick={() => window.dispatchEvent(new CustomEvent("open-palette"))}
          className="flex w-full items-center justify-between rounded-lg border border-line px-3 py-2 text-xs text-ink2 hover:bg-hover hover:text-ink"
        >
          <span>搜索</span>
          <kbd className="rounded bg-hover px-1.5 py-0.5 font-mono text-[10px]">⌘K</kbd>
        </button>
      </div>
    </aside>
  );
}
