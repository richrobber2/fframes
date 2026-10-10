import React from "react";
import { loadingSvg } from "./loading.mjs";

export type LoadingProps = {
  kind?: "bar" | "ring" | "spinner";
  /** Percentage from 0 to 100; null means unknown progress. */
  progress?: number | null;
  color?: string;
  track?: string;
  thickness?: number;
  /** Seconds per animation cycle. */
  speed?: number;
  label?: string;
  reducedMotion?: boolean;
  className?: string;
  style?: React.CSSProperties;
};

/** Accessible SVG inside a sizeable wrapper; no timers or effects required. */
export function Loading({ className, style, ...options }: LoadingProps) {
  return (
    <span
      className={className}
      style={{
        display: "inline-block",
        width: options.kind === "bar" || !options.kind ? 240 : 48,
        ...style,
      }}
      dangerouslySetInnerHTML={{ __html: loadingSvg(options) }}
    />
  );
}
export const LoadingBar = (props: Omit<LoadingProps, "kind">) => (
  <Loading {...props} kind="bar" />
);
export const ProgressRing = (props: Omit<LoadingProps, "kind">) => (
  <Loading {...props} kind="ring" />
);
export const Spinner = (props: Omit<LoadingProps, "kind" | "progress">) => (
  <Loading {...props} kind="spinner" />
);
