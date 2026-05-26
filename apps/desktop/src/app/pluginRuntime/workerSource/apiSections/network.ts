export function pluginWorkerNetworkApiSource() {
  return `network: Object.freeze({
      request(args) {
        return requestHost("network.request", args);
      },
    })`;
}
