open WebAudio
open Belt

/* @returns whether continue execution or not */
type onFrame = (~secondsFromStart: float) => bool

@module("./playbackTiming.js") external nextTick: (float, float, int) => float = "nextTick"
type audioSchedule = {delay: float, offset: float, duration: float}
@module("./playbackTiming.js") external audioTiming: (int, WasmController.namedRange, int, float) => audioSchedule = "audioTiming"
@module("./playbackTiming.js") external setSourceRate: (WebAudio.AudioNode.t, float) => unit = "setSourceRate"
module AudioRuntime = {
  let rafId: ref<option<Webapi.rafId>> = ref(None)
  let playingSources: ref<array<(WasmController.namedRange, AudioNode.t)>> = ref([])
  let startTime = ref(0.)
  let lastFrameTime = ref(None)
  let runtimeFps = ref(None)
  let smoothing = 0.9
  let tickTime = ref(-1.)
  let generation = ref(0)
  let ctx = AudioContext.create()
  let gain = ctx->AudioContext.createGain

  let setVolume = value => {
    gain["gain"]["value"] = value->Float.fromInt /. 100.
  }

  let stop = () => {
    generation := generation.contents + 1
    rafId.contents->Belt.Option.map(Webapi.cancelAnimationFrame)->ignore
    playingSources.contents->Array.forEach(nameAndSource => {
      let (_, source) = nameAndSource
      source->AudioNode.stop
    })
    playingSources := []
    rafId := None
  }

  let markFrame = () => {
    let now = ctx->AudioContext.getCurrentTime
    switch lastFrameTime.contents {
    | Some(previous) if now > previous => {
        let fps = 1. /. (now -. previous)
        runtimeFps := Some(switch runtimeFps.contents {
          | Some(last) => last *. smoothing +. fps *. (1. -. smoothing)
          | None => fps
        })
      }
    | _ => ()
    }
    lastFrameTime := Some(now)
  }

  let rec frame = (~onFrame: onFrame, ~playbackRate, ~fpsLimit, _timestamp) => {
    let currentTime = ctx->AudioContext.getCurrentTime
    let tick = nextTick(currentTime, tickTime.contents, fpsLimit)
    if tick < 0. {
      rafId := Some(Webapi.requestCancellableAnimationFrame(frame(~onFrame, ~playbackRate, ~fpsLimit)))
    } else {
      tickTime := tick
      let secondsFromStart = (currentTime -. startTime.contents) *. playbackRate
      let generationBefore = generation.contents
      let continue = onFrame(~secondsFromStart)
      if generationBefore !== generation.contents { () }
      else if continue {
        rafId := Some(Webapi.requestCancellableAnimationFrame(frame(~onFrame, ~playbackRate, ~fpsLimit)))
      } else { stop() }
    }
  }

  let connectAudioFiles = (ctx, videoMeta: WasmController.videoMeta) => {
    let {mediaList} = MediaLoader.MediaLoaderObserver.get()

    videoMeta.audioMap
    ->Js.Nullable.toOption
    ->Utils.Option.unwrapOr([])
    ->Array.keepMap(track => {
      mediaList
      ->Map.String.getExn(track.name)
      ->(
        (media: MediaLoader.loadableMedia) =>
          switch media {
          | Media(media) =>
            switch media {
            | Audio(info) => Some((track, info))
            | _ => None
            }
          | _ => None
          }
      )
    })
    ->Array.map(((track, info)) => {
      let source = ctx->AudioContext.createBufferSource
      source->AudioNode.setBuffer(info.audioData)

      (track, source)
    })
  }

  let startAnimation = (~onFrame, ~currentFrame, ~videoMeta, ~playbackRate, ~fpsLimit) => {
    stop() // in case if other animation playing

    // Browsers suspend contexts created before a user gesture. Silent videos
    // also use this clock, so resume it even when there are no audio sources.
    ctx
    ->AudioContext.resume
    ->AudioContext.catchResume(error => {
      Js.Console.error(error)
    })
    ->ignore
    lastFrameTime := None
    runtimeFps := None
    tickTime := -1.

    playingSources := ctx->connectAudioFiles(videoMeta)->Array.keep(((track, _)) => track.end > currentFrame)

    gain->AudioNode.connect(ctx.destination)->ignore
    startTime := ctx.currentTime

    playingSources.contents->Array.forEach(((track, source)) => {
      let timing = audioTiming(currentFrame, track, videoMeta.fps, playbackRate)
      setSourceRate(source, playbackRate)
      source->AudioNode.connect(gain)
      if timing.duration > 0. {
        source->AudioNode.startWithOffset(
          ~startTime=startTime.contents +. timing.delay,
          ~offset=timing.offset,
          ~duration=timing.duration,
        )
      }
    })

    rafId := Some(Webapi.requestCancellableAnimationFrame(frame(~onFrame, ~playbackRate, ~fpsLimit)))
  }
}
