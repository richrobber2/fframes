type dimensions = {width: int, height: int}
@module("./viewport.js") external getDimensions: unit => dimensions = "getDimensions"
@module("./viewport.js") external observeViewport: (unit => unit) => (unit => unit) = "observeViewport"
let useDimensions = () => {
  let (dimensions, setDimensions) = React.useState(getDimensions)
  React.useLayoutEffect0(() => {
    let cleanup = observeViewport(() => setDimensions(_ => getDimensions()))
    Some(cleanup)
  })
  dimensions
}
