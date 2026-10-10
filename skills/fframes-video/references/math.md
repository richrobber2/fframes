# Teaching math with fframes

Read this when the video explains mathematics, science or any idea with numbers in it. The working
example is [`examples/math-ideas`](https://github.com/dmtrKovalenko/fframes/tree/main/examples/math-ideas):
Euler's formula, matrices and eigenvectors, and Fourier series in three scenes. It renders as a
36-second video and runs live at <https://fframes.studio/math/>, where the viewer moves the
parameters.

## The rule: compute, do not draw

Every number and every shape on screen comes from a function that computes the mathematics from
its definition. Never place a point where it "should" be, never type a result into a label.

- Put the mathematics in its own module (`src/math.rs`) of plain functions: `euler(theta)`,
  `Matrix2::det`, `Matrix2::eigen`, `fourier_partial_sum(x, n)`. No fframes types in it.
- The scenes call those functions and turn their results into coordinates and text. Drawing code
  holds no formulas of its own beyond mapping to the screen.
- Test the module with the facts the lesson teaches, so a wrong picture fails a test, not a viewer:

```rust
#[test]
fn eigenvectors_only_stretch() {
    let m = Matrix2 { i: (2.0, 1.0), j: (1.0, 2.0) };
    for (lambda, v) in m.eigen() {
        let mv = m.apply(v);
        assert!((mv.0 - lambda * v.0).abs() < 1e-5 && (mv.1 - lambda * v.1).abs() < 1e-5);
    }
}

#[test]
fn gibbs_overshoot_does_not_go_away() {
    for terms in [20, 100, 400] {
        assert!((fourier_peak(terms) - 1.179).abs() < 0.01); // (2/π)·Si(π)
    }
}
```

Good facts to test: identities (|e^{iθ}| = 1, e^{iπ} = −1), invariants (det = area of the unit
square's image, eigenvalues multiply to det and add to the trace), limits and their surprises
(partial sums converge away from jumps, the 9% overshoot at the jump never shrinks), and known
values at special points.

## One idea per scene

- A scene states one idea in its title as a sentence: "Rotation is multiplication", "Matrices
  move space", "Any wave is circles". Four short lines of plain words under it, no symbols the
  reader has not met.
- One parameter moves: the angle, where î lands, the number of circles. In a file render it moves
  on its own (autopilot: a slow function of `frame.seconds()`); live, the viewer drives it.
- Show the live values next to the picture as text, recomputed every frame (`θ = 2.98 rad = 171°`,
  `det M = −0.71: space flips`). When the parameter reaches a special value, say so on screen
  (`θ = π:  e^(iπ) = −1`).
- Color carries meaning and stays fixed across the lesson: one color for the real part / î / the
  first quantity, another for the imaginary part / ĵ / the second, a third for what stays
  invariant (eigenvectors).
- Draw the reference state faintly (the original grid, the target square wave, the axes) under
  the changing one, so the change reads as a change.

## Text that renders correctly

- Check the fonts in `media/` for every symbol you use. A sans-serif text font often lacks Greek
  letters (θ, λ) and letters like ĵ; put formulas in a monospace font that has them (JetBrains Mono
  covers Greek, sub- and superscript digits, ∑, √, ≈, −, ×). `inspect` reports missing glyphs.
- Use real symbols: the minus sign −, ×, ·, π. Format numbers with a fixed number of decimals and
  never print "-0.00".
- Browsers collapse runs of spaces in SVG text. When formula lines are aligned with spaces (matrix
  rows), replace the spaces with no-break spaces (`'\u{a0}'`).
- Keep a fixed number of text lines per scene so the live frame keeps its shape (see below).

## Making it interactive

The same video runs live in a web page through `fframes-live` (see `AGENTS.md` in the repository,
"Interactive animation"). Read the viewer through `ctx.input()`:

```rust
// The pointer drives the parameter when it is over the picture; otherwise autopilot.
let theta = match ctx.input().pointer.filter(|&(x, _)| x < PANEL_X) {
    Some((px, py)) => (CY - py).atan2(px - CX).rem_euclid(TAU),
    None => (0.62 * frame.seconds()).rem_euclid(TAU),
};

// Buttons: clicks on data-action elements count; adjust the autopilot value by them.
let n = (1 + frame.seconds() as i64).min(12)
    + i64::from(input.count("more")) - i64::from(input.count("fewer"));
```

```rust
fframes::svgr!(
    <g data-action="more" role="button" tabindex="0" aria-label="More circles">
        <rect x="1600" y="880" width="96" height="64" rx="32" fill="#0d0d0d" />
        <text x="1648" y="925" text-anchor="middle" fill="#fff" pointer-events="none">"+"</text>
    </g>
)
```

- Keep the tree's shape fixed: draw the maximum number of circles, eigenvector lines and labels
  every frame and hide the unused ones with `opacity="0"`. A changing child count makes the live
  runtime replace that whole group.
- Things that should keep moving while the viewer explores (turning circles) can follow
  `input.time` when it is above zero (live) and `frame.seconds()` otherwise; mount with
  `keepAlive: true`.
- Give the words a `data-role` (`title`, `body`, `fact`) so a page can show them as HTML on small
  screens, where 1080p text scaled to a phone is too small to read. `landing/math/index.html` crops
  the stage to the picture and copies those texts into a card below it.

## Check it without watching

```bash
cargo test -p my-lesson                      # the mathematics is right
$R inspect --every 0.25s                     # no missing glyphs, nothing cut off
$R strip -n 9                                # every scene, the parameter moving
$R frame Rotation@50%,Matrices@50%           # read the numbers on screen and check them by hand
```

Pick frames where the answer is known (the angle at π, a matrix with a flip, many circles) and
confirm the on-screen values against the test facts. Read the numbers from the frame image or
from `svg TIME`; they are text.
