//! Menu animations, a Horizon extra (off by default): a screen opened over the
//! world slides into place and its dimmed backdrop fades in, as the ReAnimated
//! mod does on Java (by its author, MIT). The screen draws as it always does;
//! its vertices are moved afterwards, so no screen knows about the animation.
use crate::hud::{Blend, HudBatch};

/// How a kind of screen moves in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Motion {
    /// Seconds from open to rest.
    pub seconds: f32,
    /// GUI pixels below its place the screen starts.
    pub distance: f32,
    pub ease: Ease,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ease {
    OutCubic,
    /// A small overshoot at the end.
    OutBack,
}
impl Ease {
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Ease::OutCubic => 1.0 - (1.0 - t).powi(3),
            Ease::OutBack => {
                const C1: f32 = 1.70158;
                const C3: f32 = C1 + 1.0;
                let f = t - 1.0;
                1.0 + C3 * f * f * f + C1 * f * f
            }
        }
    }
}
/// Menus: 6 ticks, 16 pixels.
pub const MENU: Motion = Motion {
    seconds: 0.3,
    distance: 16.0,
    ease: Ease::OutCubic,
};
/// The pause menu opens a little quicker and shorter: 4 ticks, 12 pixels.
pub const PAUSE: Motion = Motion {
    seconds: 0.2,
    distance: 12.0,
    ease: Ease::OutCubic,
};
/// The inventory and containers rise further and settle with a bounce.
pub const CONTAINER: Motion = Motion {
    seconds: 0.3,
    distance: 30.0,
    ease: Ease::OutBack,
};

/// Where a screen is `elapsed` seconds after it opened.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    /// GUI pixels the screen is drawn below its place.
    pub offset: f32,
    /// The backdrop's opacity, 0 to 1.
    pub backdrop: f32,
}
impl Motion {
    pub fn at(self, elapsed: f32) -> Frame {
        let t = (elapsed / self.seconds.max(0.01)).clamp(0.0, 1.0);
        Frame {
            offset: (1.0 - self.ease.apply(t)) * self.distance,
            backdrop: Ease::OutCubic.apply(t),
        }
    }
    /// Whether the screen has come to rest.
    pub fn done(self, elapsed: f32) -> bool {
        elapsed >= self.seconds
    }
}

/// Apply `frame` to the screen drawn from vertex `start`: the vertices before
/// `backdrop_end` (its dimmed background) fade, the rest move down by
/// `frame.offset`, the pixel scissors of its 3D previews with them.
pub fn apply(batch: &mut HudBatch, start: usize, backdrop_end: usize, frame: Frame) {
    let backdrop_end = backdrop_end.clamp(start, batch.vertices.len());
    if frame.backdrop < 1.0 {
        for v in &mut batch.vertices[start..backdrop_end] {
            let alpha = ((v.color >> 24) as f32 * frame.backdrop).round() as u32;
            v.color = (v.color & 0x00ff_ffff) | alpha.min(255) << 24;
        }
    }
    if frame.offset == 0.0 {
        return;
    }
    for v in &mut batch.vertices[backdrop_end..] {
        v.pos[1] += frame.offset;
    }
    // Previews (the inventory's player) draw in their own scissor, which
    // the renderer keeps inside the window.
    let shift = (frame.offset * batch.scale).round() as i64;
    for (range, blend) in &mut batch.blends {
        if range.start as usize >= backdrop_end
            && let Blend::Depth([_, y, _, _]) = blend
        {
            *y = (i64::from(*y) + shift).max(0) as u32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hud::rgba;

    #[test]
    fn eases_start_at_zero_and_end_at_one() {
        for e in [Ease::OutCubic, Ease::OutBack] {
            assert_eq!(e.apply(0.0), 0.0);
            assert!((e.apply(1.0) - 1.0).abs() < 1e-6);
        }
        // OutBack overshoots before it settles; OutCubic never does.
        assert!(Ease::OutBack.apply(0.8) > 1.0);
        assert!(Ease::OutCubic.apply(0.8) < 1.0);
    }

    #[test]
    fn a_screen_slides_up_from_below_and_rests() {
        let start = MENU.at(0.0);
        assert_eq!(start.offset, 16.0);
        assert_eq!(start.backdrop, 0.0);
        let mid = MENU.at(0.15);
        assert!(mid.offset > 0.0 && mid.offset < 16.0);
        assert!(mid.backdrop > 0.0 && mid.backdrop < 1.0);
        assert_eq!(
            MENU.at(0.3),
            Frame {
                offset: 0.0,
                backdrop: 1.0
            }
        );
        assert_eq!(MENU.at(5.0), MENU.at(0.3));
        assert!(!MENU.done(0.29) && MENU.done(0.3));
        // A container passes its place and comes back.
        assert!(CONTAINER.at(0.25).offset < 0.0);
    }

    #[test]
    fn the_backdrop_fades_and_the_panel_moves() {
        let mut batch = HudBatch::new([640, 480]);
        let uv = [[0.0; 2]; 4];
        let square = |x: f32| [[x, x], [x + 10.0, x], [x + 10.0, x + 10.0], [x, x + 10.0]];
        batch.quad(square(0.0), uv, rgba(0, 0, 0, 200));
        let start = 0;
        let backdrop_end = batch.vertices.len();
        batch.quad(square(5.0), uv, u32::MAX);
        apply(
            &mut batch,
            start,
            backdrop_end,
            Frame {
                offset: 8.0,
                backdrop: 0.5,
            },
        );
        assert!(
            batch.vertices[..backdrop_end]
                .iter()
                .all(|v| v.color >> 24 == 100)
        );
        assert!(
            batch.vertices[..backdrop_end]
                .iter()
                .all(|v| v.pos[1] <= 10.0)
        );
        assert!(
            batch.vertices[backdrop_end..]
                .iter()
                .all(|v| v.pos[1] >= 13.0)
        );
        assert!(
            batch.vertices[backdrop_end..]
                .iter()
                .all(|v| v.color == u32::MAX)
        );
    }

    #[test]
    fn a_preview_scissor_moves_with_the_panel() {
        let mut batch = HudBatch::new([640, 480]);
        batch.blends.push((0..6, Blend::Depth([10, 100, 50, 60])));
        let frame = |offset| Frame {
            offset,
            backdrop: 1.0,
        };
        apply(&mut batch, 0, 0, frame(4.0));
        let shift = (4.0 * batch.scale) as u32;
        assert_eq!(batch.blends[0].1, Blend::Depth([10, 100 + shift, 50, 60]));
        // Overshooting upward never wraps the scissor below zero.
        apply(&mut batch, 0, 0, frame(-1000.0));
        assert_eq!(batch.blends[0].1, Blend::Depth([10, 0, 50, 60]));
    }
}
