export type LoadingOptions = {
  kind?: "bar" | "ring" | "spinner";
  progress?: number | null;
  color?: string;
  track?: string;
  thickness?: number;
  speed?: number;
  label?: string;
  reducedMotion?: boolean;
};
export function loadingSvg(options?: LoadingOptions): string;
