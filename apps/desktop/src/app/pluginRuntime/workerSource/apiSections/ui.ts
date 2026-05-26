export function pluginWorkerUiApiSource() {
  return `ui: Object.freeze({
      toast(message, options = {}) {
        return requestHost("ui.toast", { ...options, message });
      },
      openSettings(section) {
        return requestHost("ui.openSettings", { section });
      },
      openPanel(panel) {
        return requestHost("ui.openPanel", { panel });
      },
      openView(viewId) {
        return requestHost("ui.openPluginView", { viewId });
      },
      closeView() {
        return requestHost("ui.closePluginView");
      },
    })`;
}
