open Hooks

@genType
let a = Js.Dict.empty

@genType.as("Editor") @react.component
let make = () => {
  let context = EditorContext.useEditorContext()
  let theme = Theme.useTheme(context.options.theme->Belt.Option.getWithDefault(#system))
  let (player, dispatch) = context.usePlayer()
  let (isFullScreen, fullScreenToggler) = Hooks.useToggle(false)
  let layout = useEditorLayout(~isFullScreen)
  let viewport = UseDimensions.useDimensions()
  let previewRef = React.useRef(Js.Nullable.null)
  let (listVariant, setListVariant) = React.useState(_ =>
    switch context.options.mediaListLayout {
    | #grid => MediaList.Grid
    | #list => MediaList.List
    | #fromAspectRatio =>
      if layout.preview.width /. layout.preview.height > 1.3 {
        MediaList.List
      } else {
        MediaList.Grid
      }
    }
  )

  let videoTitle = React.useMemo1(() => {
    switch context.videoMeta.name->Js.String.split("::")->Utils.Array.last {
    | Some(name) => React.string(name)
    | _ => React.string("Unknown video")
    }
  }, [context.videoMeta])

  let videoDetails = {
    let {width, height, fps, durationInFrames} = context.videoMeta
    `${width->Belt.Int.toString}×${height->Belt.Int.toString} · ${fps->Belt.Int.toString} fps · ${durationInFrames->Utils.Duration.formatFrame(
        fps,
      )}`
  }

  let viewOption = (variant, ~label, ~icon) => {
    let selected = listVariant === variant
    let button =
      <button
        type_="button"
        role="radio"
        ariaLabel=label
        onClick={_ => setListVariant(_ => variant)}
        className={Cx.cx([
          "inline-flex h-7 w-8 items-center justify-center rounded-full transition-colors duration-150 [&>svg]:size-4",
          IconButton.focusRing,
          selected
            ? "bg-surface-elevated text-default shadow-[0_1px_4px_-1px_rgb(0_0_0/20%)]"
            : "text-secondary hover:text-default",
        ])}>
        icon
      </button>

    <Tooltip content={React.string(label)}>
      {React.cloneElement(button, {"aria-checked": selected})}
    </Tooltip>
  }

  React.useEffect0(() => {
    previewRef.current
    ->Js.Nullable.toOption
    ->Belt.Option.forEach(Js.Console.log2("Happy video hacking! Your preview will be rendered at"))

    None
  })

  <Theme.Provider value=theme>
    <Tooltip.Provider>
      <div
        id="fframes-editor"
        className={Cx.cx(["relative h-screen w-screen overflow-hidden bg-surface font-sans text-default antialiased", (viewport.width < 900 || viewport.height < 650) ? "fframes-mobile" : ""])}>
        <ProjectNavigation scenes=context.videoMeta.scenesTimeline frame=player.frame
          duration=context.videoMeta.durationInFrames fps=context.videoMeta.fps
          onScene={frame => {
            if player.rangeLoop {dispatch(ToggleRangeLoop)}
            dispatch(Seek(frame))
          }} />
        <style type_="text/css">
          {React.string(
            `
            #editor-preview > svg {
            transform-origin: top left !important;
            transform: scale(${layout.preview.scale->Js.Float.toString}) !important
            }
            `,
          )}
        </style>
        <div className="fframes-preview-row flex w-full justify-center overflow-auto">
          {if !isFullScreen && layout.mediaControls->Belt.Option.isNone {
            <details className="fframes-mobile-media">
              <summary> {React.string("Media")} </summary>
              <div>
                <h1 className="text-base font-semibold"> {videoTitle} </h1>
                <p className="text-xs text-secondary"> {React.string(videoDetails)} </p>
                <MediaList variant=MediaList.List />
              </div>
            </details>
          } else {React.null}}
          {
            // MediaList
            layout.mediaControls
            ->Belt.Option.map(size =>
              <aside
                ariaLabel="Project"
                style={size->UseEditorLayout.sizeToStyle}
                className="flex h-full flex-col overflow-auto border-r border-subtle bg-surface-secondary">
                <header
                  className="sticky top-0 z-10 flex items-center justify-between gap-3 bg-surface-secondary/90 px-4 py-3 backdrop-blur-lg">
                  <div className="min-w-0">
                    <h1 className="truncate text-base font-semibold"> {videoTitle} </h1>
                    <p className="truncate text-xs text-secondary tabular">
                      {React.string(videoDetails)}
                    </p>
                  </div>
                  <div
                    role="radiogroup"
                    ariaLabel="Media layout"
                    className="flex shrink-0 gap-0.5 rounded-full bg-primary-soft-alpha p-0.5">
                    {viewOption(MediaList.List, ~label="List view", ~icon=<Icons.ListViewIcon />)}
                    {viewOption(MediaList.Grid, ~label="Grid view", ~icon=<Icons.GridViewIcon />)}
                  </div>
                </header>
                <h2 className="px-4 pt-2 pb-2 text-xs font-medium text-secondary">
                  {React.string("Media")}
                </h2>
                <div className="pb-4"> <MediaList variant=listVariant /> </div>
              </aside>
            )
            ->Utils.Option.unwrapOr(React.null)
          }
          // Preview
          <div
            id="editor-preview"
            ref={ReactDOM.Ref.domRef(previewRef)}
            style={layout.preview->UseEditorLayout.sizeToStyle}
            className="bg-black">
            {player.svg
            ->Utils.Option.unwrapOr("")
            ->VDom.parseReactElement({
              htmlparser2: {
                xmlMode: true,
              },
            })}
          </div>
        </div>
        {
          // Timeline
          layout.timeLine
          ->Belt.Option.map(sectionSize =>
            <section
              ariaLabel="Timeline"
              style={sectionSize->UseEditorLayout.sizeToStyle}
              className="w-screen border-t border-subtle bg-surface-secondary">
              <Timeline sectionSize />
            </section>
          )
          ->Utils.Option.unwrapOr(React.null)
        }
        <Dock isFullScreen fullScreenToggler timelineSize=?{layout.timeLine} />
      </div>
    </Tooltip.Provider>
  </Theme.Provider>
}
