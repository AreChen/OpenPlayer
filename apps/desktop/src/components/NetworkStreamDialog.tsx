import type { FormEvent } from "react";
import { Icon } from "../app/Icon";
import type { NetworkStreamHistoryEntry } from "../app/types";
import type { AppStrings } from "../i18n";

type NetworkStreamDialogProps = {
  t: AppStrings;
  networkStreamUrl: string;
  networkStreamError: string | null;
  networkStreamHistory: NetworkStreamHistoryEntry[];
  onClose: () => void;
  onSubmit: (event: FormEvent<HTMLFormElement>) => void | Promise<void>;
  onUrlChange: (value: string) => void;
  onClearHistory: () => void;
  onOpenHistoryEntry: (entry: NetworkStreamHistoryEntry) => void;
};

export function NetworkStreamDialog({
  t,
  networkStreamUrl,
  networkStreamError,
  networkStreamHistory,
  onClose,
  onSubmit,
  onUrlChange,
  onClearHistory,
  onOpenHistoryEntry,
}: NetworkStreamDialogProps) {
  return (
    <div
      className="network-stream-backdrop"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) {
          onClose();
        }
      }}
    >
      <section
        className="network-stream-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="network-stream-title"
        onContextMenu={(event) => event.stopPropagation()}
        onPointerDown={(event) => event.stopPropagation()}
      >
        <header className="network-stream-header">
          <div>
            <span className="settings-kicker">OpenPlayer</span>
            <h2 id="network-stream-title">{t.streamDialog.title}</h2>
            <p>{t.streamDialog.subtitle}</p>
          </div>
          <button className="settings-close" type="button" aria-label={t.controls.close} onClick={onClose}>
            <Icon name="close" />
          </button>
        </header>

        <form className="network-stream-form" onSubmit={onSubmit}>
          <label>
            <span>{t.streamDialog.urlLabel}</span>
            <input
              autoFocus
              type="url"
              inputMode="url"
              spellCheck={false}
              value={networkStreamUrl}
              placeholder="rtsp://192.168.1.10/live"
              onChange={(event) => onUrlChange(event.currentTarget.value)}
            />
          </label>
          {networkStreamError && <p className="network-stream-error">{t.streamDialog.error(networkStreamError)}</p>}
          <div className="network-stream-actions">
            <button className="settings-reset" type="button" onClick={onClose}>
              {t.common.cancel}
            </button>
            <button className="settings-reset network-stream-primary" type="submit" disabled={!networkStreamUrl.trim()}>
              {t.streamDialog.open}
            </button>
          </div>
        </form>

        <section className="network-stream-recent">
          <header>
            <div>
              <h3>{t.streamDialog.recent}</h3>
              <span>{t.streamDialog.supportedProtocols}</span>
            </div>
            <button className="settings-reset" type="button" onClick={onClearHistory} disabled={networkStreamHistory.length === 0}>
              {t.streamDialog.clearHistory}
            </button>
          </header>
          {networkStreamHistory.length > 0 ? (
            <div className="network-stream-list">
              {networkStreamHistory.map((entry) => (
                <button className="network-stream-item" type="button" key={`${entry.updatedAt}:${entry.url}`} onClick={() => onOpenHistoryEntry(entry)}>
                  <span>{entry.name}</span>
                  <small>
                    {entry.scheme.toUpperCase()} · {entry.url}
                  </small>
                </button>
              ))}
            </div>
          ) : (
            <div className="network-stream-empty">{t.streamDialog.emptyHistory}</div>
          )}
        </section>
      </section>
    </div>
  );
}
