//! The mathematics behind the three scenes, computed from definitions. The scenes draw these
//! numbers and nothing else, and the tests below state each idea as a fact that can be checked.

use std::f32::consts::PI;

/// `e^{iθ}` as `(cos θ, sin θ)`: the point at angle `θ` on the unit circle.
pub fn euler(theta: f32) -> (f32, f32) {
    (theta.cos(), theta.sin())
}

/// Multiplies two complex numbers `(re, im)`.
pub fn complex_mul(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

/// A 2×2 matrix by its columns: where `î = (1, 0)` and `ĵ = (0, 1)` land.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix2 {
    pub i: (f32, f32),
    pub j: (f32, f32),
}

impl Matrix2 {
    /// `M · v`: the point `v.0 · î + v.1 · ĵ`.
    pub fn apply(&self, v: (f32, f32)) -> (f32, f32) {
        (
            v.0 * self.i.0 + v.1 * self.j.0,
            v.0 * self.i.1 + v.1 * self.j.1,
        )
    }

    /// The signed area of the parallelogram spanned by `î` and `ĵ`, which is how much `M` scales
    /// every area. Negative when `M` flips orientation.
    pub fn det(&self) -> f32 {
        self.i.0 * self.j.1 - self.j.0 * self.i.1
    }

    pub fn trace(&self) -> f32 {
        self.i.0 + self.j.1
    }

    /// Real eigenvalues with unit eigenvectors, largest first. Empty when the eigenvalues are
    /// complex (the map turns every direction).
    pub fn eigen(&self) -> Vec<(f32, (f32, f32))> {
        let (t, d) = (self.trace(), self.det());
        let discriminant = t * t / 4.0 - d;
        if discriminant < 0.0 {
            return Vec::new();
        }
        let root = discriminant.sqrt();
        let (a, b, c, dd) = (self.i.0, self.j.0, self.i.1, self.j.1);
        [t / 2.0 + root, t / 2.0 - root]
            .into_iter()
            .map(|lambda| {
                // (M − λI) v = 0: take the row with the larger entries for numerical stability.
                let v = if b.abs() + (lambda - a).abs() >= c.abs() + (lambda - dd).abs() {
                    (b, lambda - a)
                } else {
                    (lambda - dd, c)
                };
                let length = v.0.hypot(v.1);
                let v = if length < 1e-6 {
                    // M = λI: every direction is an eigenvector.
                    (1.0, 0.0)
                } else {
                    (v.0 / length, v.1 / length)
                };
                (lambda, v)
            })
            .collect()
    }
}

/// The square wave `sign(sin x)`.
pub fn square_wave(x: f32) -> f32 {
    let s = x.sin();
    if s > 0.0 {
        1.0
    } else if s < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// Radius of the `term`-th circle (0-based) of the square wave's Fourier series: the term is
/// `(4/π) · sin(k x) / k` with odd `k = 2·term + 1`.
pub fn fourier_term(term: usize) -> (f32, f32) {
    let k = (2 * term + 1) as f32;
    (k, 4.0 / (PI * k))
}

/// The sum of the first `terms` circles at `x`.
pub fn fourier_partial_sum(x: f32, terms: usize) -> f32 {
    (0..terms)
        .map(|term| {
            let (k, amplitude) = fourier_term(term);
            amplitude * (k * x).sin()
        })
        .sum()
}

/// The highest value of the partial sum just after the jump at 0. It does not approach 1 as more
/// circles are added but about 1.179: the Gibbs overshoot of about 9% of the jump.
pub fn fourier_peak(terms: usize) -> f32 {
    // The first maximum sits at x = π / (2N); search around it.
    let terms = terms.max(1);
    let first_max = PI / (2.0 * terms as f32);
    (1..=200)
        .map(|step| fourier_partial_sum(first_max * 2.0 * step as f32 / 200.0, terms))
        .fold(f32::MIN, f32::max)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32, tolerance: f32) -> bool {
        (a - b).abs() <= tolerance
    }

    #[test]
    fn euler_points_lie_on_the_unit_circle() {
        for step in 0..100 {
            let (x, y) = euler(step as f32 * 0.13);
            assert!(close(x.hypot(y), 1.0, 1e-5));
        }
    }

    #[test]
    fn e_to_the_i_pi_is_minus_one() {
        let (x, y) = euler(PI);
        assert!(close(x, -1.0, 1e-6) && close(y, 0.0, 1e-6));
    }

    #[test]
    fn multiplying_by_e_to_the_i_theta_rotates_by_theta() {
        let z = (3.0, 4.0);
        let rotated = complex_mul(z, euler(PI / 2.0));
        assert!(close(rotated.0, -4.0, 1e-5) && close(rotated.1, 3.0, 1e-5));
        // Angles add when you multiply.
        let product = complex_mul(euler(0.4), euler(1.1));
        let sum = euler(1.5);
        assert!(close(product.0, sum.0, 1e-6) && close(product.1, sum.1, 1e-6));
    }

    #[test]
    fn determinant_is_the_area_of_the_unit_square_image() {
        let m = Matrix2 {
            i: (2.0, 0.5),
            j: (1.0, 1.5),
        };
        // Shoelace formula over the image of the unit square.
        let corners = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].map(|v| m.apply(v));
        let shoelace: f32 = (0..4)
            .map(|k| {
                let (a, b) = (corners[k], corners[(k + 1) % 4]);
                a.0 * b.1 - b.0 * a.1
            })
            .sum::<f32>()
            / 2.0;
        assert!(close(m.det(), shoelace, 1e-5));
        // Rotations keep area; reflections flip its sign.
        let (c, s) = euler(0.7);
        assert!(close(
            Matrix2 {
                i: (c, s),
                j: (-s, c)
            }
            .det(),
            1.0,
            1e-6
        ));
        assert!(close(
            Matrix2 {
                i: (1.0, 0.0),
                j: (0.0, -1.0)
            }
            .det(),
            -1.0,
            1e-6
        ));
    }

    #[test]
    fn eigenvectors_only_stretch() {
        let m = Matrix2 {
            i: (2.0, 1.0),
            j: (1.0, 2.0),
        };
        let pairs = m.eigen();
        assert_eq!(pairs.len(), 2);
        assert!(close(pairs[0].0, 3.0, 1e-5) && close(pairs[1].0, 1.0, 1e-5));
        for (lambda, v) in pairs {
            let mv = m.apply(v);
            assert!(close(mv.0, lambda * v.0, 1e-5) && close(mv.1, lambda * v.1, 1e-5));
        }
        // Eigenvalues multiply to the determinant and add to the trace.
        let m = Matrix2 {
            i: (1.5, 0.3),
            j: (0.4, 0.8),
        };
        let pairs = m.eigen();
        assert!(close(pairs[0].0 * pairs[1].0, m.det(), 1e-5));
        assert!(close(pairs[0].0 + pairs[1].0, m.trace(), 1e-5));
    }

    #[test]
    fn rotations_have_no_real_eigenvectors() {
        let (c, s) = euler(1.0);
        assert!(
            Matrix2 {
                i: (c, s),
                j: (-s, c)
            }
            .eigen()
            .is_empty()
        );
    }

    #[test]
    fn fourier_sums_approach_the_square_wave_away_from_the_jumps() {
        for &x in &[0.6, 1.5, 2.4, 4.0, 5.5] {
            let error = |terms| (fourier_partial_sum(x, terms) - square_wave(x)).abs();
            assert!(error(200) < error(3));
            assert!(error(400) < 0.01);
        }
    }

    #[test]
    fn gibbs_overshoot_does_not_go_away() {
        // (2/π) · Si(π) ≈ 1.1790: about 9% of the jump from −1 to 1.
        for terms in [20, 100, 400] {
            assert!(close(fourier_peak(terms), 1.179, 0.01));
        }
    }
}
