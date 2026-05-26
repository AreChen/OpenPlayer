import { invoke } from "@tauri-apps/api/core";
import { MAX_PLUGIN_NETWORK_TIMEOUT_MS, supportedPluginNetworkMethods } from "./constants";
import { runtimeNumberArg, runtimeStringArg } from "./args";

export function normalizePluginNetworkHeaders(value: unknown) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return {};
  }
  const headers: Record<string, string> = {};
  for (const [rawKey, rawValue] of Object.entries(value as Record<string, unknown>).slice(0, 32)) {
    const key = rawKey.trim();
    if (!/^[A-Za-z0-9!#$%&'*+.^_`|~-]{1,64}$/.test(key) || typeof rawValue !== "string") {
      continue;
    }
    const headerValue = rawValue.trim();
    if (!headerValue || headerValue.length > 1024 || /[\r\n]/.test(headerValue)) {
      continue;
    }
    headers[key] = headerValue;
  }
  return headers;
}

export function pluginNetworkUrl(value: unknown) {
  if (typeof value !== "string" || value.length > 2048 || /\s/.test(value)) {
    return null;
  }
  try {
    const url = new URL(value);
    return url.protocol === "http:" || url.protocol === "https:" ? url.toString() : null;
  } catch {
    return null;
  }
}

export async function runPluginNetworkRequest(args: Record<string, unknown>) {
  const url = pluginNetworkUrl(args.url);
  if (!url) {
    throw new Error("network.request requires an http or https url");
  }
  const method = (runtimeStringArg(args, "method") ?? "GET").toUpperCase();
  if (!supportedPluginNetworkMethods.has(method)) {
    throw new Error(`network.request method is unsupported: ${method}`);
  }
  const timeoutMs = Math.min(MAX_PLUGIN_NETWORK_TIMEOUT_MS, Math.max(1000, runtimeNumberArg(args, "timeoutMs") ?? 15_000));
  const headers = normalizePluginNetworkHeaders(args.headers);
  let body: BodyInit | undefined;
  if (method !== "GET" && method !== "HEAD" && typeof args.body === "string") {
    if (args.body.length > 256 * 1024) {
      throw new Error("network.request body is too large");
    }
    body = args.body;
  }

  const response = await invoke("plugin_network_request", {
    args: {
      url,
      method,
      headers,
      body,
      timeoutMs,
    },
  });
  if (!response || typeof response !== "object") {
    throw new Error("network.request returned an invalid response");
  }
  return response;
}
