import type { IconName } from "./types";
import paths from "./icon-paths.json";

export function Icon({ name }: { name: IconName }) {
  return (
    <svg aria-hidden="true" className="icon" fill="none" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.8" viewBox="0 0 24 24">
      <path d={paths[name]} />
    </svg>
  );
}
