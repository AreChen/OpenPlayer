import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";

export function loadSource(path, imports = {}, globals = {}) {
  const source = readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
  const { outputText } = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
  });
  const exports = {};
  vm.runInNewContext(outputText, {
    exports, console, ...globals,
    require(name) {
      if (!(name in imports)) throw new Error(`Unconfigured import: ${name} in ${path}`);
      return imports[name];
    },
  }, { filename: path });
  return exports;
}

export function hookHarness() {
  const slots = [];
  let cursor = 0;
  let effects = [];
  const timers = new Map();
  let timerId = 0;
  return {
    react: {
      useRef(value) { return slots[cursor++] ??= { current: value }; },
      useState(initial) {
        const index = cursor++;
        const slot = slots[index] ??= { value: typeof initial === "function" ? initial() : initial };
        return [slot.value, (next) => { slot.value = typeof next === "function" ? next(slot.value) : next; }];
      },
      useEffect(effect, dependencies) {
        const index = cursor++;
        const previous = slots[index];
        if (previous && dependencies.every((value, i) => Object.is(value, previous.dependencies[i]))) return;
        effects.push(() => {
          previous?.cleanup?.();
          slots[index] = { dependencies, cleanup: effect() };
        });
      },
    },
    window: {
      setInterval(callback) { timers.set(++timerId, callback); return timerId; },
      clearInterval(id) { timers.delete(id); },
    },
    render(callback) {
      cursor = 0;
      const result = callback();
      effects.forEach((effect) => effect());
      effects = [];
      return result;
    },
    tick() { [...timers.values()].forEach((callback) => callback()); },
    unmount() { slots.forEach((slot) => slot?.cleanup?.()); },
  };
}

export const settle = () => new Promise((resolve) => setImmediate(resolve));
export function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
