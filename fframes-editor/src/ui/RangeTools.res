@react.component @module("./RangeTools.tsx")
external make: (
  ~fps: int,
  ~start: int,
  ~end: int,
  ~duration: int,
  ~current: int,
  ~enabled: bool,
  ~onStart: int => unit,
  ~onEnd: int => unit,
  ~onToggle: unit => unit,
  ~onPause: unit => unit,
) => React.element = "RangeTools"
