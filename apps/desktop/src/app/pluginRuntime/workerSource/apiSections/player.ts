export function pluginWorkerPlayerApiSource() {
  return `player: Object.freeze({
      play() {
        return requestHost("player.play");
      },
      pause() {
        return requestHost("player.pause");
      },
      togglePlayback() {
        return requestHost("player.togglePlayback");
      },
      stop() {
        return requestHost("player.stop");
      },
      seek(args) {
        return requestHost("player.seek", args);
      },
      frameStep() {
        return requestHost("player.frameStep");
      },
      frameBackStep() {
        return requestHost("player.frameBackStep");
      },
      setVolume(volume, options = {}) {
        return requestHost("player.setVolume", { ...options, volume });
      },
      setSpeed(speed) {
        return requestHost("player.setSpeed", { speed });
      },
      setLoopMode(mode) {
        return requestHost("player.setLoopMode", { mode });
      },
      setVideoFill(enabled) {
        return requestHost("player.setVideoFill", { enabled });
      },
      setSubtitleDelay(delay) {
        return requestHost("player.setSubtitleDelay", { delay });
      },
      selectTrack(kind, trackId) {
        return requestHost("player.selectTrack", { kind, trackId });
      },
      wall: Object.freeze({
        open(tiles) {
          return requestHost("player.wall.open", { tiles });
        },
        layout(tiles) {
          return requestHost("player.wall.layout", { tiles });
        },
        snapshot() {
          return requestHost("player.wall.snapshot");
        },
        setVisible(visible) {
          return requestHost("player.wall.setVisible", { visible });
        },
        close() {
          return requestHost("player.wall.close");
        },
      }),
    })`;
}
