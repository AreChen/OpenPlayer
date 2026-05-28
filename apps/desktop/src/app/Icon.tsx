import type { IconName } from "./types";

export function Icon({ name }: { name: IconName }) {
  const paths: Record<IconName, string> = {
    camera: "M4 8h3l1.5-2h7L17 8h3v10H4V8ZM12 16a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7Z",
    close: "M6 6l12 12M18 6 6 18",
    cpu: "M9 3v3M15 3v3M9 18v3M15 18v3M3 9h3M3 15h3M18 9h3M18 15h3M8 6h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2ZM10 10h4v4h-4z",
    folder: "M3 7.5h6l2 2h10v8.5a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7.5Z",
    folderAdd: "M3 7.5h6l2 2h10v8.5a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7.5ZM12 13v5M9.5 15.5h5",
    fullscreen: "M8 4H4v4M16 4h4v4M20 16v4h-4M8 20H4v-4",
    info: "M12 17v-6M12 7h.01M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18Z",
    list: "M8 6h12M8 12h12M8 18h12M4 6h.01M4 12h.01M4 18h.01",
    maximize: "M7 7h10v10H7z",
    minimize: "M6 12h12",
    next: "M7 6l7 6-7 6V6ZM16 6v12",
    palette: "M12 3a9 9 0 0 0 0 18h1.2a1.8 1.8 0 0 0 1.3-3.05 1.8 1.8 0 0 1 1.27-3.07H18a3 3 0 0 0 3-3A9 9 0 0 0 12 3ZM7.5 11.5h.01M9 7.5h.01M14 7.5h.01M16.5 11h.01",
    pause: "M8 6h3v12H8zM13 6h3v12h-3z",
    pin: "M14 4l6 6-3 1-4 4v4l-2 2-2-6-6-2 2-2h4l4-4 1-3Z",
    play: "M8 5v14l11-7z",
    plugin: "M9 3v4M15 3v4M8 7h8a2 2 0 0 1 2 2v3a6 6 0 0 1-12 0V9a2 2 0 0 1 2-2ZM12 18v3",
    preview: "M4 5h16v11H4zM8 20h8M10 16l-1.5 4M14 16l1.5 4M7 13l3-3 2 2 2.5-3 3.5 4",
    previous: "M17 6l-7 6 7 6V6ZM8 6v12",
    record: "M12 7a5 5 0 1 1 0 10 5 5 0 0 1 0-10Z",
    restart: "M5 12a7 7 0 1 0 2-4.9M5 5v5h5",
    settings: "M12 8.5a3.5 3.5 0 1 1 0 7 3.5 3.5 0 0 1 0-7ZM19 12a7.2 7.2 0 0 0-.08-1l2-1.55-2-3.45-2.36.95a7.4 7.4 0 0 0-1.72-1L14.5 3h-4l-.34 2.95a7.4 7.4 0 0 0-1.72 1L6.08 6l-2 3.45L6.08 11A7.2 7.2 0 0 0 6 12c0 .34.03.67.08 1l-2 1.55 2 3.45 2.36-.95c.53.42 1.1.75 1.72 1l.34 2.95h4l.34-2.95c.62-.25 1.19-.58 1.72-1l2.36.95 2-3.45-2-1.55c.05-.33.08-.66.08-1Z",
    stop: "M7 7h10v10H7z",
    stream: "M5 18a13 13 0 0 1 14 0M8 14a7.5 7.5 0 0 1 8 0M11 10a2 2 0 0 1 2 0M12 10v8",
    tracks: "M4 7h7M15 7h5M11 5v4M4 12h12M20 12h0M16 10v4M4 17h4M12 17h8M8 15v4",
    tv: "M4 7a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V7ZM8 21h8M12 17v4",
    volume: "M4 10v4h4l5 4V6l-5 4H4Z M16 9a4 4 0 0 1 0 6",
    volumeMuted: "M4 10v4h4l5 4V6l-5 4H4Z M17 9l4 6M21 9l-4 6",
  };

  return (
    <svg aria-hidden="true" className="icon" fill="none" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.8" viewBox="0 0 24 24">
      <path d={paths[name]} />
    </svg>
  );
}
