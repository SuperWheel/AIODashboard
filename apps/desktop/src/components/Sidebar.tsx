interface NavItem {
  key: string;
  label: string;
  icon: string;
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
  return (
    <aside className="flex w-52 shrink-0 flex-col border-r border-line bg-surface2 px-3 py-5">
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
              <span className="w-4 text-center opacity-80">{item.icon}</span>
              <span className="flex-1">{item.label}</span>
              {item.key === "inbox" && inboxOpen > 0 && (
                <span className="rounded-full bg-violet/15 px-1.5 py-0.5 text-[10px] text-violet">
                  {inboxOpen}
                </span>
              )}
            </button>
          );
        })}
      </nav>

      <div className="mt-auto px-2">
        <button
          onClick={() => window.dispatchEvent(new CustomEvent("open-palette"))}
          className="flex w-full items-center justify-between rounded-lg border border-line px-3 py-2 text-xs text-ink2 hover:bg-hover"
        >
          <span>搜索</span>
          <kbd className="rounded bg-hover px-1.5 py-0.5 font-mono text-[10px]">⌘K</kbd>
        </button>
      </div>
    </aside>
  );
}
