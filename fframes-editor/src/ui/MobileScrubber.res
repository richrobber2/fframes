@react.component @module("./MobileScrubber.tsx")
external make: (~frame: int, ~duration: int, ~fps: int, ~onSeek: int => unit) => React.element = "MobileScrubber"
