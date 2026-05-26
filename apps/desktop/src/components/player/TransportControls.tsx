import { TransportControlStrip } from "./TransportControlStrip";
import type { TransportControlsProps } from "./TransportControls.types";
import { TransportTimeline } from "./TransportTimeline";

export function TransportControls(props: TransportControlsProps) {
  return (
    <div className="transport" aria-label={props.t.contextMenu.play}>
      <TransportTimeline
        t={props.t}
        mediaLoaded={props.mediaLoaded}
        duration={props.duration}
        displayTime={props.displayTime}
        progress={props.progress}
        progressRatio={props.progressRatio}
        effectiveTimeDisplayMode={props.effectiveTimeDisplayMode}
        canShowFrames={props.canShowFrames}
        currentTransportLabel={props.currentTransportLabel}
        durationTransportLabel={props.durationTransportLabel}
        currentTimeToggleLabel={props.currentTimeToggleLabel}
        durationTimeToggleLabel={props.durationTimeToggleLabel}
        previousIndex={props.previousIndex}
        nextIndex={props.nextIndex}
        onToggleTimeDisplayMode={props.onToggleTimeDisplayMode}
        onPlayPreviousQueueItem={props.onPlayPreviousQueueItem}
        onSeekTo={props.onSeekTo}
        onCommitSeekTo={props.onCommitSeekTo}
        onPlayNextQueueItem={props.onPlayNextQueueItem}
      />
      <TransportControlStrip
        t={props.t}
        locale={props.locale}
        mediaLoaded={props.mediaLoaded}
        isPickerOpen={props.isPickerOpen}
        isPlaying={props.isPlaying}
        mediaPanelMode={props.mediaPanelMode}
        isMuted={props.isMuted}
        volumeMuteLabel={props.volumeMuteLabel}
        volumeLevel={props.volumeLevel}
        playbackSpeed={props.playbackSpeed}
        isPlaylistOpen={props.isPlaylistOpen}
        hardwareDecodingMode={props.hardwareDecodingMode}
        hardwareDecodingLabel={props.hardwareDecodingLabel}
        hardwareDecodingToggleLabel={props.hardwareDecodingToggleLabel}
        pluginControlLeftActions={props.pluginControlLeftActions}
        pluginControlCenterActions={props.pluginControlCenterActions}
        pluginControlRightActions={props.pluginControlRightActions}
        isPluginActionDisabled={props.isPluginActionDisabled}
        onExecutePluginAction={props.onExecutePluginAction}
        onOpenNativeMediaFiles={props.onOpenNativeMediaFiles}
        onStopPlayback={props.onStopPlayback}
        onTogglePlayback={props.onTogglePlayback}
        onToggleLoopPanel={props.onToggleLoopPanel}
        onToggleMute={props.onToggleMute}
        onSetVolume={props.onSetVolume}
        onToggleSpeedPanel={props.onToggleSpeedPanel}
        onToggleTrackPanel={props.onToggleTrackPanel}
        onTogglePlaylist={props.onTogglePlaylist}
        onToggleHardwareDecoding={props.onToggleHardwareDecoding}
      />
    </div>
  );
}
