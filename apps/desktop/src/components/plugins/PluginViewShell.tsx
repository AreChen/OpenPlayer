import type { RefObject } from "react";
import { Icon } from "../../app/Icon";
import type { ActivePluginView } from "../../app/types";
import { startMainWindowDrag } from "../../app/windowControls";
import type { AppStrings } from "../../i18n";

type PluginViewShellProps = {
  t: AppStrings;
  activePluginView: ActivePluginView;
  documentHtml: string;
  frameRef: RefObject<HTMLIFrameElement | null>;
  onClose: () => void;
};

export function PluginViewShell({ t, activePluginView, documentHtml, frameRef, onClose }: PluginViewShellProps) {
  return (
    <section
      className="plugin-view-shell"
      aria-label={activePluginView.title}
      onContextMenu={(event) => event.stopPropagation()}
      onPointerDown={(event) => event.stopPropagation()}
    >
      <header
        className="plugin-view-header"
        onPointerDown={(event) => {
          event.stopPropagation();
          if (event.button !== 0 || (event.target as HTMLElement).closest("[data-plugin-view-close]")) {
            return;
          }
          event.preventDefault();
          startMainWindowDrag();
        }}
      >
        <div>
          <span>OpenPlayer Plugin</span>
        </div>
        <button type="button" aria-label={t.controls.close} data-plugin-view-close="true" onClick={onClose}>
          <Icon name="close" />
        </button>
      </header>
      <iframe
        ref={frameRef}
        className="plugin-view-frame"
        title={activePluginView.title}
        sandbox="allow-scripts allow-forms allow-modals"
        style={{ backgroundColor: "transparent" }}
        srcDoc={documentHtml}
      />
    </section>
  );
}
