import { useLayoutEffect, useRef } from "react";

export function usePluginPanelInsets(enabled: boolean) {
  const ref = useRef<HTMLElement>(null);
  useLayoutEffect(() => {
    const shell = ref.current;
    const stage = shell?.parentElement;
    if (!enabled || !shell || !stage) return;
    const chrome = stage.querySelector<HTMLElement>(".window-controls");
    const transport = stage.querySelector<HTMLElement>(".transport");
    const measure = () => {
      // Layout coordinates deliberately ignore chrome hide/show transforms.
      const top = chrome ? chrome.offsetTop + chrome.offsetHeight + 12 : 12;
      const bottom = transport ? stage.clientHeight - transport.offsetTop + 12 : 24;
      shell.style.setProperty("--plugin-panel-top", `${top}px`);
      shell.style.setProperty("--plugin-panel-bottom", `${bottom}px`);
    };
    const observer = new ResizeObserver(measure);
    for (const element of [stage, chrome, transport]) if (element) observer.observe(element);
    measure();
    return () => observer.disconnect();
  }, [enabled]);
  return ref;
}
