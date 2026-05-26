import type { PluginRuntimeSource } from "../types";
import { pluginWorkerApiSource } from "./workerSource/api";
import { pluginWorkerBootstrapSource } from "./workerSource/bootstrap";
import { pluginWorkerMessageSource } from "./workerSource/messages";
import { pluginWorkerSandboxSource } from "./workerSource/sandbox";

export function buildPluginWorkerSource(source: PluginRuntimeSource) {
  const pluginLabel = JSON.stringify(`${source.name} (${source.pluginId})`);
  return `
"use strict";
(() => {
${pluginWorkerSandboxSource()}
${pluginWorkerApiSource()}
${pluginWorkerMessageSource(pluginLabel)}
})();
${pluginWorkerBootstrapSource(source.script, source.pluginId)}
`;
}
