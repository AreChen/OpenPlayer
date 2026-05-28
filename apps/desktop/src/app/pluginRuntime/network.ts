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

export function normalizePluginNetworkBodyFile(value: unknown) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return null;
  }
  const record = value as Record<string, unknown>;
  const path = runtimeStringArg(record, "path");
  if (!path) {
    return null;
  }
  const contentType = runtimeStringArg(record, "contentType");
  return contentType ? { path, contentType } : { path };
}

export async function runPluginNetworkRequest(args: Record<string, unknown>, pluginId: string) {
  const url = pluginNetworkUrl(args.url);
  if (!url) {
    throw new Error("network.request requires an http or https url");
  }
  const method = (runtimeStringArg(args, "method") ?? "GET").toUpperCase();
  if (!supportedPluginNetworkMethods.has(method)) {
    throw new Error(`network.request method is unsupported: ${method}`);
  }
  const responseType = runtimeStringArg(args, "responseType") ?? "text";
  if (responseType !== "text" && responseType !== "base64") {
    throw new Error(`network.request responseType is unsupported: ${responseType}`);
  }
  const timeoutMs = Math.min(MAX_PLUGIN_NETWORK_TIMEOUT_MS, Math.max(1000, runtimeNumberArg(args, "timeoutMs") ?? 15_000));
  const headers = normalizePluginNetworkHeaders(args.headers);
  let body: BodyInit | undefined;
  const bodyFile = normalizePluginNetworkBodyFile(args.bodyFile);
  if (typeof args.body === "string" && bodyFile) {
    throw new Error("network.request cannot use both body and bodyFile");
  }
  if (method !== "GET" && method !== "HEAD" && typeof args.body === "string") {
    if (args.body.length > 256 * 1024) {
      throw new Error("network.request body is too large");
    }
    body = args.body;
  } else if ((method === "GET" || method === "HEAD") && (typeof args.body === "string" || bodyFile)) {
    throw new Error("network.request body requires a non-GET method");
  }

  const response = await invoke("plugin_network_request", {
    pluginId,
    args: {
      url,
      method,
      headers,
      body,
      bodyFile,
      timeoutMs,
      responseType,
    },
  });
  if (!response || typeof response !== "object") {
    throw new Error("network.request returned an invalid response");
  }
  return response;
}
