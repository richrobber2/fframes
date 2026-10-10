open Belt

type playState = Playing | Paused | WaitingForAction | CantPlay

/// This state should only contain a state that changes that affect the animation runtime or will likely change 60 t/s
@genType
type state = {
  frame: int,
  startPlayingFrame: int,
  playState: playState,
  fpsLimit: option<int>,
  playbackRate: float,
  svg: option<string>,
  volume: option<int>,
  magnet: option<int>,
  loopStart: int,
  loopEnd: int,
  rangeLoop: bool,
  zoom: float,
  viewportOffset: float,
}

@genType
type action =
  | Seek(int)
  | NewFrame(int)
  | AllowPlay
  | Play
  | Pause
  | SetVolume(int)
  | SetPlaybackRate(float)
  | SetFpsLimit(int)
  | SetMagnet
  | SetLoopStart(int)
  | SetLoopEnd(int)
  | ToggleRangeLoop
  | SetZoom(float)
  | SetViewportOffset(float)
  | BatchZoomUpdate(float, float) // zoom, viewportOffset

let currentFps: ref<option<int>> = ref(None)

@inline
let volume_key = "ffvolume"
@inline
let frame_key = "fframe"
@inline
let scene_key = "ffscene"
@inline
let zoom_key = "ffzoom"
@inline
let viewport_offset_key = "ffviewportoffset"
let get_magnet_key = (video: WasmController.videoMeta) => video.name ++ "_ffmagnet"

let min_volume = 0
let max_volume = 100
let validateVolume = Utils.Math.minMax(~min=min_volume, ~max=max_volume)

let min_zoom = 0.1
let max_zoom = 10.0
let validateZoom = Utils.Math.minMax(~min=min_zoom, ~max=max_zoom)

@module("./services/VideoFrameBufferManager")
external initVideoFrameBufferManager: int => unit = "initVideoFrameBufferManager"

module MakePlayer = (Wasm: WasmController.WasmBridge) => {
  initVideoFrameBufferManager(Wasm.videoMeta.fps)
  let frame_key = Wasm.videoMeta.name ++ "_fframe"
  let scene_key = Wasm.videoMeta.name ++ "_ffscene"
  let zoom_key = Wasm.videoMeta.name ++ "_ffzoom"
  let viewport_offset_key = Wasm.videoMeta.name ++ "_ffviewportoffset"

  module PlayerState = {
    type t = state
    open WasmController

    let sceneIndex =
      Dom.Storage.getItem(scene_key, Dom.Storage.localStorage)->Option.flatMap(Js.Int.fromString)

    let savedMagnet =
      Dom.Storage.getItem(get_magnet_key(Wasm.videoMeta), Dom.Storage.localStorage)->Option.flatMap(
        Js.Int.fromString,
      )

    let savedZoom =
      Dom.Storage.getItem(zoom_key, Dom.Storage.localStorage)
      ->Option.flatMap(str => {
        let parsed = Js.Float.fromString(str)
        if Js.Float.isNaN(parsed) {
          None
        } else {
          Some(validateZoom(parsed))
        }
      })
      ->Option.getWithDefault(1.0)

    let savedViewportOffset =
      Dom.Storage.getItem(viewport_offset_key, Dom.Storage.localStorage)
      ->Option.flatMap(str => {
        let parsed = Js.Float.fromString(str)
        if Js.Float.isNaN(parsed) || parsed < 0.0 {
          None
        } else {
          Some(parsed)
        }
      })
      ->Option.getWithDefault(0.0)

    let initialFrame = switch (
      savedMagnet,
      Wasm.videoMeta.scenesTimeline->Js.Nullable.toOption,
      sceneIndex,
    ) {
    | (Some(magnet), _, _) =>
      magnet->Utils.Math.minI(Wasm.videoMeta.durationInFrames)->Utils.Option.some
    | (_, Some(scenes), Some(index)) if index >= 0 && index < Js.Array.length(scenes) =>
      scenes->Js.Array.get(index)->Option.map(scene => scene.start)
    | _ =>
      Dom.Storage.getItem(frame_key, Dom.Storage.localStorage)
      ->Option.flatMap(Js.Int.fromString)
      ->Option.map(frame =>
        Utils.Math.minI(Utils.Math.maxI(0, frame), Wasm.videoMeta.durationInFrames)
      )
    }->Utils.Option.unwrapOr(0)

    let volume = switch Dom.Storage.getItem(volume_key, Dom.Storage.localStorage) {
    | Some(savedValue) if Wasm.videoMeta.hasAudio =>
      savedValue->Js.Int.fromString->Option.map(validateVolume)
    | None if Wasm.videoMeta.hasAudio => Some(60)
    | _ => None
    }

    let initial = switch MediaLoader.MediaLoaderObserver.get() {
    | state if state.allMediaLoaded => {
        frame: initialFrame,
        startPlayingFrame: initialFrame,
        playState: WaitingForAction,
        fpsLimit: None,
        playbackRate: 1.0,
        volume: volume,
        svg: Wasm.controller->WasmController.render_frame(0->Js.BigInt.fromInt)->Utils.Option.some,
        magnet: savedMagnet,
        loopStart: 0,
        loopEnd: Wasm.videoMeta.durationInFrames,
        rangeLoop: false,
        zoom: savedZoom,
        viewportOffset: savedViewportOffset,
      }
    | _ => {
        frame: initialFrame,
        startPlayingFrame: initialFrame,
        playState: CantPlay,
        svg: None,
        volume: volume,
        fpsLimit: None,
        playbackRate: 1.0,
        magnet: savedMagnet,
        loopStart: 0,
        loopEnd: Wasm.videoMeta.durationInFrames,
        rangeLoop: false,
        zoom: savedZoom,
        viewportOffset: savedViewportOffset,
      }
    }
  }

  include UseObservable.Pubsub(PlayerState)

  let recordFrame = frame => {
    switch Wasm.videoMeta.scenesTimeline->Js.Nullable.toOption {
    | Some(scenes) => {
        let currentScene = scenes->Js.Array.findIndex(scene => scene.end > frame)
        Dom.Storage.localStorage |> Dom.Storage.setItem(scene_key, currentScene->Js.Int.toString)
      }
    | _ => Dom.Storage.localStorage |> Dom.Storage.setItem(frame_key, frame->Js.Int.toString)
    }
  }

  let reducer = action => {
    let state = get()
    switch action {
    | Seek(frame) | NewFrame(frame) if frame >= Wasm.videoMeta.durationInFrames || frame < 0 => {
        let frame = state.magnet->Utils.Option.unwrapOr(0)
        let svg = Wasm.controller->WasmController.render_frame(frame->Js.BigInt.fromInt)

        {
          ...state,
          frame: frame,
          svg: Some(svg),
          playState: Wasm.options.loop ? Playing : Paused,
          startPlayingFrame: frame,
        }
      }
    | Seek(frame) | NewFrame(frame) => {
        let svg = Wasm.controller->WasmController.render_frame(frame->Js.BigInt.fromInt)

        {
          ...state,
          frame: frame,
          svg: Some(svg),
          startPlayingFrame: switch action {
          | Seek(frame) => frame
          | _ => state.startPlayingFrame
          },
        }
      }
    | SetLoopStart(frame) => {
        ...state,
        loopStart: Utils.Math.maxI(0, Utils.Math.minI(frame, state.loopEnd - 1)),
      }
    | SetLoopEnd(frame) => {
        ...state,
        loopEnd: Utils.Math.minI(
          Wasm.videoMeta.durationInFrames,
          Utils.Math.maxI(frame, state.loopStart + 1),
        ),
      }
    | ToggleRangeLoop => {...state, rangeLoop: !state.rangeLoop}
    | Play if state.rangeLoop => {
        let frame = if state.frame < state.loopStart || state.frame >= state.loopEnd {
          state.loopStart
        } else {
          state.frame
        }
        {
          ...state,
          frame: frame,
          startPlayingFrame: frame,
          svg: Some(Wasm.controller->WasmController.render_frame(frame->Js.BigInt.fromInt)),
          playState: Playing,
        }
      }
    | AllowPlay => {...state, playState: WaitingForAction}
    | Play if state.frame <= 0 || state.frame >= Wasm.videoMeta.durationInFrames => {
        ...state,
        frame: state.magnet->Utils.Option.unwrapOr(0),
        playState: Playing,
      }
    | Play => {...state, playState: Playing, startPlayingFrame: state.frame}
    | Pause => {...state, playState: Paused}
    | SetPlaybackRate(rate) => {...state, playbackRate: Js.Math.max(0.25, Js.Math.min(2.0, rate))}
    | SetFpsLimit(limit) => {...state, fpsLimit: limit <= 0 ? None : Some(max(1, min(120, limit)))}
    | SetVolume(volume) => {
        ...state,
        volume: Some(volume),
      }
    | SetMagnet if state.magnet === Some(state.frame) => {
        ...state,
        magnet: None,
      }
    | SetMagnet => {
        ...state,
        magnet: Some(state.frame),
      }
    | SetZoom(zoom) => {
        ...state,
        zoom: validateZoom(zoom),
      }
    | SetViewportOffset(offset) => {
        Dom.Storage.localStorage |> Dom.Storage.setItem(
          viewport_offset_key,
          offset->Js.Float.toString,
        )
        {
          ...state,
          viewportOffset: offset,
        }
      }
    | BatchZoomUpdate(zoom, offset) => {
        Dom.Storage.localStorage |> Dom.Storage.setItem(
          viewport_offset_key,
          offset->Js.Float.toString,
        )
        {
          ...state,
          zoom: validateZoom(zoom),
          viewportOffset: offset,
        }
      }
    }
  }

  let onFrame = (dispatch, ~secondsFromStart) => {
    let nextFrame =
      (secondsFromStart *. Wasm.videoMeta.fps->Float.fromInt +.
        get().startPlayingFrame->Float.fromInt)->Utils.Math.floor

    let state = get()
    let restarted = if state.rangeLoop && nextFrame >= state.loopEnd {
      AnimationRuntime.AudioRuntime.markFrame()
      dispatch(
        Seek(state.loopStart + mod(nextFrame - state.loopStart, state.loopEnd - state.loopStart)),
      )
      true
    } else if state.rangeLoop && nextFrame < state.loopStart {
      dispatch(Seek(state.loopStart))
      true
    } else {
      if nextFrame !== state.frame {
        AnimationRuntime.AudioRuntime.markFrame()
        dispatch(NewFrame(nextFrame))
      }
      false
    }

    !restarted && get().playState === Playing
  }

  let sideEffect = (action, dispatch) => {
    let startPlaying = currentFrame => {
      let state = get()
      let currentFrame = if (
        state.rangeLoop && (currentFrame < state.loopStart || currentFrame >= state.loopEnd)
      ) {
        state.loopStart
      } else {
        currentFrame
      }
      get().volume->Option.map(AnimationRuntime.AudioRuntime.setVolume)->ignore
      AnimationRuntime.AudioRuntime.startAnimation(
        ~onFrame=onFrame(dispatch),
        ~currentFrame,
        ~videoMeta=Wasm.videoMeta,
        ~playbackRate=state.playbackRate,
        ~fpsLimit=state.fpsLimit->Option.getWithDefault(0),
      )
      ()
    }

    switch action {
    | Play if get().playState !== Playing => startPlaying(get().frame)
    | Seek(newFrame) => {
        AnimationRuntime.AudioRuntime.stop()
        startPlaying(newFrame)
        recordFrame(newFrame)
      }
    | NewFrame(newFrame) if mod(newFrame, Wasm.videoMeta.fps) === 0 => recordFrame(newFrame)
    | Pause => AnimationRuntime.AudioRuntime.stop()
    | SetVolume(volume) => {
        AnimationRuntime.AudioRuntime.setVolume(volume)
        Dom.Storage.localStorage |> Dom.Storage.setItem(volume_key, volume->Js.Int.toString)
      }
    | SetMagnet if get().magnet !== Some(get().frame) =>
      Dom.Storage.localStorage |> Dom.Storage.setItem(
        get_magnet_key(Wasm.videoMeta),
        get().frame->Js.Int.toString,
      )
    | SetMagnet if get().magnet === Some(get().frame) =>
      Dom.Storage.localStorage |> Dom.Storage.removeItem(get_magnet_key(Wasm.videoMeta))
    | SetZoom(zoom) =>
      Dom.Storage.localStorage |> Dom.Storage.setItem(zoom_key, zoom->Js.Float.toString)
    | BatchZoomUpdate(zoom, _) =>
      Dom.Storage.localStorage |> Dom.Storage.setItem(zoom_key, zoom->Js.Float.toString)
    | _ => ()
    }
  }

  let rec dispatch = action => {
    sideEffect(action, dispatch)
    let wasPlaying = get().playState === Playing
    reducer(action)->set
    switch action {
    | SetPlaybackRate(_) | SetFpsLimit(_) if wasPlaying => {
        dispatch(Pause)
        dispatch(Play)
      }
    | _ => ()
    }
  }
}
