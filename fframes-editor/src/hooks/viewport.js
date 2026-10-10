// The visual viewport shrinks when Android browser chrome or the keyboard opens.
export function measureViewport(viewport, inner, insets) {
  const width = Math.max(
    1,
    Math.floor(Math.min(inner.width, viewport?.width ?? inner.width))
  );
  const height = Math.max(
    1,
    Math.floor(Math.min(inner.height, viewport?.height ?? inner.height))
  );
  return {
    width: Math.max(1, width - insets.left - insets.right),
    height: Math.max(1, height - insets.top - insets.bottom - 12),
    viewportWidth: width,
    viewportHeight: height,
  };
}
export function getDimensions() {
  const probe = document.createElement("div");
  probe.style.cssText =
    "position:fixed;visibility:hidden;padding:env(safe-area-inset-top,0px) env(safe-area-inset-right,0px) env(safe-area-inset-bottom,0px) env(safe-area-inset-left,0px)";
  document.body.append(probe);
  const style = getComputedStyle(probe);
  const inset = side => Math.ceil(parseFloat(style[side]) || 0);
  const size = measureViewport(
    window.visualViewport,
    { width: window.innerWidth, height: window.innerHeight },
    {
      top: inset("paddingTop"),
      right: inset("paddingRight"),
      bottom: inset("paddingBottom"),
      left: inset("paddingLeft"),
    }
  );
  probe.remove();
  document.documentElement.style.setProperty(
    "--editor-viewport-width",
    `${size.viewportWidth}px`
  );
  document.documentElement.style.setProperty(
    "--editor-viewport-height",
    `${size.viewportHeight}px`
  );
  return { width: size.width, height: Math.max(1, size.height - (size.width < 360 ? 148 : 80)) };
}
export function observeViewport(callback) {
  let request;
  const update = () => {
    cancelAnimationFrame(request);
    request = requestAnimationFrame(callback);
  };
  window.addEventListener("resize", update);
  window.addEventListener("orientationchange", update);
  window.visualViewport?.addEventListener("resize", update);
  return () => {
    cancelAnimationFrame(request);
    window.removeEventListener("resize", update);
    window.removeEventListener("orientationchange", update);
    window.visualViewport?.removeEventListener("resize", update);
  };
}
