//! Menu animations (Horizon Extras, off by default): a screen opened in game
//! slides in over a backdrop that fades in (`horizon_client::screen_anim`).
use super::{Game, Screen};
use horizon_client::{hud::HudBatch, screen_anim};
use std::time::Instant;

impl Game<'_> {
    /// How `screen` moves in, or `None` for one that opens as it is: the chat
    /// (other mods animate it), the death screen, the game mode switcher and
    /// the screens `hud_layers` doesn't draw.
    fn menu_motion(&self, screen: Screen) -> Option<screen_anim::Motion> {
        Some(match screen {
            Screen::Inventory | Screen::Creative => screen_anim::CONTAINER,
            Screen::Paused if self.pause_menu => screen_anim::PAUSE,
            Screen::Stats
            | Screen::Advancements
            | Screen::WorldOptions
            | Screen::Friends
            | Screen::ServerLinks
            | Screen::ConfirmLink
            | Screen::DialogWarning
            | Screen::CommandBlock
            | Screen::Mod => screen_anim::MENU,
            Screen::Playing
            | Screen::Paused
            | Screen::Options
            | Screen::Chat
            | Screen::Dead
            | Screen::GameModeSwitcher
            | Screen::Win => return None,
        })
    }
    /// Move the screen just drawn from `start` to where its animation is.
    pub(super) fn animate_menu(&mut self, batch: &mut HudBatch, screen: Screen, start: usize) {
        let now = Instant::now();
        if self.menu_anim.0 != screen {
            self.menu_anim = (screen, now);
        }
        if !self.settings.extras.menu_animations {
            return;
        }
        let Some(motion) = self.menu_motion(screen) else {
            return;
        };
        let elapsed = now.duration_since(self.menu_anim.1).as_secs_f32();
        if motion.done(elapsed) {
            return;
        }
        let backdrop_end = batch.backdrop_end as usize;
        screen_anim::apply(batch, start, backdrop_end, motion.at(elapsed));
    }
}
