@react.component @module("./ProjectNavigation.tsx")
external make: (~scenes: Js.Nullable.t<array<WasmController.namedRange>>, ~frame: int, ~duration: int, ~fps: int, ~onScene: int => unit) => React.element = "ProjectNavigation"
