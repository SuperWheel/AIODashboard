function renderIcons(root) {
  for (const placeholder of root.querySelectorAll("i[data-lucide]")) {
    const svg = ICONS[placeholder.dataset.lucide];
    if (svg) placeholder.outerHTML = svg;
  }
}

let dispose = null;

export async function onload(api) {
  await onunload();
  let controller = null;
  let active = true;
  const registrations = [];
  const { createElement: h, useRef, useLayoutEffect } = api.react;
  const component = (mode) => function TrialSurface() {
    const container = useRef(null);
    useLayoutEffect(() => {
      if (!active || !container.current) return;
      controller ??= createTrialController();
      const current = controller;
      const element = container.current;
      current.mount(element, mode);
      return () => current.detach(element);
    }, []);
    return h("div", { ref: container, "data-job-tracker-trial-surface": mode });
  };
  dispose = () => {
    if (!active) return;
    active = false;
    controller?.destroy();
    controller = null;
    for (const unregister of registrations.splice(0).reverse()) unregister();
  };
  try {
    registrations.push(api.ui.registerView({
      id: "view", title: "求职台·试用", icon: "💼", component: component("view"),
    }));
    registrations.push(api.ui.registerTodayCard({
      id: "card", title: "求职台·试用", size: "lg", component: component("card"),
    }));
  } catch (error) {
    await onunload();
    throw error;
  }
}

export async function onunload() {
  const cleanup = dispose;
  dispose = null;
  cleanup?.();
}
