import { buildDialogViewProps } from "./viewProps/dialogs";
import { buildPanelViewProps } from "./viewProps/panels";
import { buildShellViewProps } from "./viewProps/shell";
import { buildTransportViewProps } from "./viewProps/transport";
import type { PlayerAppViewProps, PlayerAppViewPropsContext } from "./viewProps/types";

export function buildPlayerAppViewProps(context: PlayerAppViewPropsContext): PlayerAppViewProps {
  return {
    ...buildShellViewProps(context),
    ...buildTransportViewProps(context),
    ...buildPanelViewProps(context),
    ...buildDialogViewProps(context),
  };
}
