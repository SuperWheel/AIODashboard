import { useCallback, useEffect, useState } from "react";

/**
 * 主题三态：跟随系统（默认，由 prefers-color-scheme 媒体查询驱动）/ 浅色 / 深色。
 * 手动选择写 data-theme 到 <html>（覆盖媒体查询），并存 localStorage；
 * index.html 内联脚本在首帧前还原，避免闪烁。
 */
export type ThemeMode = "system" | "light" | "dark";

const STORAGE_KEY = "aio-theme";

export function readThemeMode(): ThemeMode {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    return v === "light" || v === "dark" ? v : "system";
  } catch {
    return "system";
  }
}

export function useTheme() {
  const [mode, setMode] = useState<ThemeMode>(readThemeMode);

  useEffect(() => {
    const root = document.documentElement;
    if (mode === "system") {
      root.removeAttribute("data-theme");
    } else {
      root.setAttribute("data-theme", mode);
    }
    try {
      localStorage.setItem(STORAGE_KEY, mode);
    } catch {
      // 隐私模式等场景存不进去就用会话内状态
    }
  }, [mode]);

  const cycle = useCallback(() => {
    setMode((m) => (m === "system" ? "light" : m === "light" ? "dark" : "system"));
  }, []);

  return { mode, cycle };
}
