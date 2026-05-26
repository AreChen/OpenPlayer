export function pluginWorkerSandboxSource() {
  return `
  const readyHandlers = [];
  const eventHandlers = [];
  const beforeOpenMediaHandlers = [];
  const commandHandlers = new Map();
  const pending = new Map();
  let nextRequestId = 1;
  const requestHost = (command, args = {}) => {
    if (typeof command !== "string" || !command.trim()) {
      return Promise.reject(new Error("OpenPlayer plugin command is required"));
    }
    const requestId = nextRequestId++;
    globalThis.postMessage({ type: "openplayer:request", requestId, command, args });
    return new Promise((resolve, reject) => {
      pending.set(requestId, { resolve, reject });
    });
  };
  const disabledApi = () => {
    throw new Error("This browser API is disabled in the OpenPlayer plugin worker sandbox");
  };
  globalThis.fetch = undefined;
  globalThis.XMLHttpRequest = undefined;
  globalThis.WebSocket = undefined;
  globalThis.EventSource = undefined;
  globalThis.Worker = undefined;
  globalThis.SharedWorker = undefined;
  globalThis.importScripts = disabledApi;
`;
}
