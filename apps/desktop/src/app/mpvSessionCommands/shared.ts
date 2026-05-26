export type RefValue<T> = {
  current: T;
};

export type SetValue<T> = (value: T) => void;
export type ReportError = (error: unknown) => void;
export type AnchorDisplayClock = (
  position: number,
  playing: boolean,
  upperDuration?: number,
  speed?: number,
) => void;
