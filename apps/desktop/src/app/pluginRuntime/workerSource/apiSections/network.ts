export function pluginWorkerNetworkApiSource() {
  return `network: Object.freeze({
      request(args) {
        return requestHost("network.request", args);
      },
      requestJson(args) {
        const input = args && typeof args === "object" ? args : {};
        const headers = Object.assign({}, input.headers || {});
        if (!headers.Accept && !headers.accept) {
          headers.Accept = "application/json";
        }
        const request = Object.assign({}, input, { headers, responseType: "text" });
        if (Object.prototype.hasOwnProperty.call(input, "body") && input.body !== undefined) {
          request.body = JSON.stringify(input.body);
          if (!headers["Content-Type"] && !headers["content-type"] && !input.bodyFile) {
            headers["Content-Type"] = "application/json";
          }
        }
        return requestHost("network.request", request).then((response) => {
          const json = response.text ? JSON.parse(response.text) : null;
          return Object.assign({}, response, { json });
        });
      },
    })`;
}
