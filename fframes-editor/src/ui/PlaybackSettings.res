@react.component @module("./PlaybackSettings.tsx")
external make: (~rate: float, ~limit: int, ~onRate: float => unit, ~onLimit: int => unit) => React.element = "PlaybackSettings"
