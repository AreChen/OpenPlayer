export function pluginWorkerBootstrapSource(pluginScript: string, pluginId: string) {
  return `try {
${pluginScript}
} catch (error) {
  globalThis.postMessage({ type: "openplayer:error", message: String(error && error.message ? error.message : error) });
}
try {
  Promise.resolve()
    .then(() => globalThis.__openplayerPluginReady())
    .then(() => {
      globalThis.postMessage({ type: "openplayer:ready" });
    })
    .catch((error) => {
      globalThis.postMessage({ type: "openplayer:error", message: String(error && error.message ? error.message : error) });
    })
    .finally(() => {
      delete globalThis.__openplayerPluginReady;
    });
} catch (error) {
  globalThis.postMessage({ type: "openplayer:error", message: String(error && error.message ? error.message : error) });
  delete globalThis.__openplayerPluginReady;
}
//# sourceURL=openplayer-plugin-${pluginId.replace(/[^a-z0-9.-]/gi, "_")}.js
`;
}
