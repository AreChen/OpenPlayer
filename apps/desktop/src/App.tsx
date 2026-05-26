import { surface } from "./app/constants";
import { PlayerOverlayApp } from "./components/player/PlayerOverlayApp";

function App() {
  if (surface === "video") {
    return <main className="video-host-surface" aria-label="OpenPlayer video surface" />;
  }

  return <PlayerOverlayApp />;
}

export default App;
