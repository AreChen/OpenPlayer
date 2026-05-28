import type { CSSProperties, RefObject } from "react";
import type { ActivePluginView } from "../../app/types";
import type { AppStrings } from "../../i18n";

type PluginViewShellProps = {
  t: AppStrings;
  activePluginView: ActivePluginView;
  documentHtml: string;
  frameRef: RefObject<HTMLIFrameElement | null>;
  onClose: () => void;
};

export function PluginViewShell({ activePluginView, documentHtml, frameRef }: PluginViewShellProps) {
  const shellStyle =
    activePluginView.frameOpacity === null
      ? undefined
      : ({ "--plugin-view-frame-opacity": String(activePluginView.frameOpacity) } as CSSProperties);

  return (
    <section
      className={`plugin-view-shell plugin-view-shell--${activePluginView.presentation}`}
      aria-label={activePluginView.title}
      style={shellStyle}
      onContextMenu={(event) => event.stopPropagation()}
      onPointerDown={(event) => event.stopPropagation()}
    >
      <iframe
        ref={frameRef}
        className="plugin-view-frame"
        title={activePluginView.title}
        allowTransparency={true}
        sandbox="allow-scripts allow-forms allow-modals"
        style={{ backgroundColor: "transparent" }}
        srcDoc={documentHtml}
      />
    </section>
  );
}
