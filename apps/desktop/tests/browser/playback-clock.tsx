import { Profiler, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { usePlaybackClock } from "../../src/hooks/usePlaybackClock";
import { PlaybackClockContext } from "../../src/components/player/PlaybackClockContext";
import { TransportTimeline } from "../../src/components/player/TransportTimeline";
import { translations } from "../../src/i18n";
import "../../src/styles.css";

const metrics = { shellRenders: 0, timelineCommits: 0, timelineDurationMs: 0, scheduledFrames: 0 };
const nativeFrame = window.requestAnimationFrame.bind(window);
window.requestAnimationFrame = (callback) => { metrics.scheduledFrames++; return nativeFrame(callback); };
const control: Record<string, unknown> = { metrics };
Object.assign(window, { clockTest: control });

function Fixture() {
  metrics.shellRenders++;
  const [visible, setVisible] = useState(true);
  const [frames, setFrames] = useState(false);
  const { clock, anchorDisplayClock } = usePlaybackClock({ mediaId: "test", isPlaying: true, duration: 120, playbackSpeed: 1 });
  useEffect(() => { anchorDisplayClock(10, true, 120, 1); }, []);
  Object.assign(control, { clock, setVisible, setFrames });
  return (
    <PlaybackClockContext.Provider value={{ clock, timelineVisible: visible, framesPerSecond: 30, locale: "en-US" }}>
      <div className="transport" style={{ margin: "24px", position: "static", opacity: visible ? 1 : 0 }}>
        <Profiler id="timeline" onRender={(_id, _phase, duration) => { metrics.timelineCommits++; metrics.timelineDurationMs += duration; }}>
          <TransportTimeline
            t={translations["en-US"]} mediaLoaded duration={120} displayTime={0} progress={0} progressRatio={0}
            effectiveTimeDisplayMode={frames ? "frames" : "timecode"} canShowFrames
            currentTransportLabel="00:00" durationTransportLabel={frames ? "3,600" : "02:00"}
            currentTimeToggleLabel="Current time" durationTimeToggleLabel="Duration" previousIndex={null} nextIndex={null}
            onToggleTimeDisplayMode={() => setFrames((value) => !value)} onPlayPreviousQueueItem={() => {}} onPlayNextQueueItem={() => {}}
            onSeekTo={(value) => clock.anchor(value, false, 120, 1)} onCommitSeekTo={(value) => clock.anchor(value, true, 120, 1)}
          />
        </Profiler>
      </div>
    </PlaybackClockContext.Provider>
  );
}
createRoot(document.getElementById("root")!).render(<Fixture />);
