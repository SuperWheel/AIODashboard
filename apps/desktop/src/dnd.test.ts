import { describe, expect, it } from "vitest";
import {
  exceedsDragThreshold,
  liftFromSequence,
  nearestRectIndex,
  neighborsInBand,
  placeInSequence,
  pointInExpandedRect,
  resolveDropBefore,
} from "./dnd";

describe("任务卡拖拽预览序列", () => {
  it("起拖时立即移除被拖卡，不在原位置保留占位", () => {
    expect(liftFromSequence(["a", "b", "c", "d"], "b")).toEqual([
      "a",
      "c",
      "d",
    ]);
  });

  it("可从无占位的起拖序列插入目标卡之前", () => {
    expect(placeInSequence(["a", "c", "d"], "b", "d", true)).toEqual([
      "a",
      "c",
      "b",
      "d",
    ]);
  });

  it("反复经过目标时始终只有一个占位，并可改到目标卡之后", () => {
    expect(placeInSequence(["a", "b", "c", "d"], "b", "d", false)).toEqual([
      "a",
      "c",
      "d",
      "b",
    ]);
  });

  it("目标不存在时保持当前预览，避免意外跳位", () => {
    const current = ["a", "c", "b", "d"];
    expect(placeInSequence(current, "b", "missing", true)).toEqual(current);
  });

  it("轻微移动仍是点击，达到五像素后才进入拖拽", () => {
    expect(exceedsDragThreshold(10, 10, 13, 13)).toBe(false);
    expect(exceedsDragThreshold(10, 10, 13, 14)).toBe(true);
  });

  it("卡片大部分区域直接占位，只有底部区域表示放在目标之后", () => {
    const rect = { top: 100, bottom: 200 };
    expect(resolveDropBefore(105, rect, null)).toBe(true);
    expect(resolveDropBefore(150, rect, null)).toBe(true);
    expect(resolveDropBefore(159, rect, false)).toBe(true);
    expect(resolveDropBefore(168, rect, false)).toBe(false);
    expect(resolveDropBefore(176, rect, true)).toBe(true);
    expect(resolveDropBefore(177, rect, true)).toBe(false);
  });

  it("当前占位周围有稳定区，擦过边缘不会立即跳到相邻卡片", () => {
    const rect = { left: 100, right: 200, top: 80, bottom: 140 };
    expect(pointInExpandedRect(95, 75, rect, 8)).toBe(true);
    expect(pointInExpandedRect(208, 148, rect, 8)).toBe(true);
    expect(pointInExpandedRect(91, 75, rect, 8)).toBe(false);
    expect(pointInExpandedRect(150, 149, rect, 8)).toBe(false);
  });

  it("双列命中先锁定水平方向所在列，再按纵向选择落点", () => {
    const rects = [
      { left: 0, right: 100, top: 0, bottom: 80 },
      { left: 0, right: 100, top: 300, bottom: 380 },
      { left: 120, right: 220, top: 120, bottom: 200 },
    ];
    // 光标位于右列，即使左列某卡片在纵向更近，也只能命中右列。
    expect(nearestRectIndex(rects, 170, 20)).toBe(2);
    // 左列内部再按纵向距离选择第二张卡片。
    expect(nearestRectIndex(rects, 50, 250)).toBe(1);
  });

  it("跨星级提交只使用目标档内邻居", () => {
    const ordered = [
      { id: "five-a", priority: 5 },
      { id: "drag", priority: 2 },
      { id: "four-a", priority: 4 },
      { id: "four-b", priority: 4 },
      { id: "three-a", priority: 3 },
    ];
    const neighbors = neighborsInBand(ordered, 1, 4);
    expect(neighbors.before).toBeNull();
    expect(neighbors.after?.id).toBe("four-a");
  });

  it("档内移动可同时取得同档上下邻居", () => {
    const ordered = [
      { id: "five-a", priority: 5 },
      { id: "five-b", priority: 5 },
      { id: "drag", priority: 5 },
      { id: "five-c", priority: 5 },
      { id: "four-a", priority: 4 },
    ];
    const neighbors = neighborsInBand(ordered, 2, 5);
    expect(neighbors.before?.id).toBe("five-b");
    expect(neighbors.after?.id).toBe("five-c");
  });
});
