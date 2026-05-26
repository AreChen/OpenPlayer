import { Icon } from "../../app/Icon";
import { formatHistoryProgress } from "../../app/playback";
import type { MediaItem, PlaybackHistoryEntry, PluginActionDefinition, PluginActionInstance } from "../../app/types";
import type { AppStrings } from "../../i18n";
import { PluginActionButton } from "../plugins/PluginActionButton";

type PlaylistDrawerProps = {
  t: AppStrings;
  locale: string;
  queueItems: MediaItem[];
  currentIndex: number | null;
  playbackHistory: PlaybackHistoryEntry[];
  currentMediaPath: string | null;
  isPickerOpen: boolean;
  pluginPlaylistActions: PluginActionInstance[];
  isPluginActionDisabled: (action: PluginActionDefinition) => boolean;
  onExecutePluginAction: (instance: PluginActionInstance) => void;
  onAppendNativeMediaFiles: () => void;
  onAppendNativeMediaFolder: () => void;
  onChooseQueueItem: (index: number) => void;
  onOpenHistoryEntry: (entry: PlaybackHistoryEntry) => void;
  onClearPlaybackHistory: () => void;
};

export function PlaylistDrawer({
  t,
  locale,
  queueItems,
  currentIndex,
  playbackHistory,
  currentMediaPath,
  isPickerOpen,
  pluginPlaylistActions,
  isPluginActionDisabled,
  onExecutePluginAction,
  onAppendNativeMediaFiles,
  onAppendNativeMediaFolder,
  onChooseQueueItem,
  onOpenHistoryEntry,
  onClearPlaybackHistory,
}: PlaylistDrawerProps) {
  return (
    <aside className="playlist-drawer playlist-drawer--open" aria-label={t.media.playlist}>
      <header className="playlist-drawer-header">
        <h3>{t.media.playlist}</h3>
        <div className="playlist-actions">
          <button type="button" onClick={onAppendNativeMediaFiles} disabled={isPickerOpen}>
            <Icon name="folderAdd" />
            <span>{t.media.addFiles}</span>
          </button>
          <button type="button" onClick={onAppendNativeMediaFolder} disabled={isPickerOpen}>
            <Icon name="folder" />
            <span>{t.media.addFolder}</span>
          </button>
          {pluginPlaylistActions.map((instance) => (
            <PluginActionButton
              key={`${instance.plugin.id}:${instance.action.id}`}
              instance={instance}
              locale={locale}
              t={t}
              disabled={isPluginActionDisabled(instance.action)}
              onExecute={onExecutePluginAction}
            />
          ))}
        </div>
      </header>

      {queueItems.length > 0 && (
        <ol>
          {queueItems.map((item, index) => (
            <li key={item.id}>
              <button
                className={`playlist-item ${index === currentIndex ? "playlist-item--active" : ""}`}
                type="button"
                aria-current={index === currentIndex ? "true" : undefined}
                onClick={() => onChooseQueueItem(index)}
              >
                <span>{item.name}</span>
              </button>
            </li>
          ))}
        </ol>
      )}

      {playbackHistory.length > 0 && (
        <section className="history-section" aria-label={t.media.recent}>
          <header>
            <h3>{t.media.recent}</h3>
            <button className="history-clear" type="button" onClick={onClearPlaybackHistory}>
              {t.common.clear}
            </button>
          </header>
          <div className="history-list">
            {playbackHistory.map((entry) => (
              <button
                key={entry.path}
                className={`history-item ${currentMediaPath === entry.path ? "history-item--active" : ""}`}
                type="button"
                title={entry.path}
                onClick={() => onOpenHistoryEntry(entry)}
              >
                <span>{entry.name}</span>
                <small>{formatHistoryProgress(entry, t)}</small>
              </button>
            ))}
          </div>
        </section>
      )}

      {queueItems.length === 0 && playbackHistory.length === 0 && <div className="playlist-empty">{t.media.emptyPlaylist}</div>}
    </aside>
  );
}
