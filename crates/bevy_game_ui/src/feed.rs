//! Follow-latest scrolling without stealing the reader's position.
use bevy::prelude::*;

/// Attach to a native scroll container. The game supplies a monotonic content
/// revision; this component never stores or interprets event data.
#[derive(Component, Debug)]
pub struct UiFeedScroll {
    /// Latest content revision (reset by replacing this component on a new feed).
    pub revision: u64,
    seen: u64,
    following: bool,
    previous_max: f32,
    previous_offset: f32,
    requested: bool,
}
impl Default for UiFeedScroll {
    fn default() -> Self {
        Self {
            revision: 0,
            seen: 0,
            following: true,
            previous_max: 0.0,
            previous_offset: 0.0,
            requested: false,
        }
    }
}
impl UiFeedScroll {
    /// Whether new content currently follows the newest entry.
    #[must_use]
    pub fn follows_latest(&self) -> bool {
        self.following
    }
    /// Content revisions received while reading older entries.
    #[must_use]
    pub fn unread(&self) -> u64 {
        self.revision.saturating_sub(self.seen)
    }
    /// Explicitly return to the newest content on the next layout pass.
    pub fn jump_to_latest(&mut self) {
        self.requested = true;
    }
    fn resolve(&mut self, offset: f32, maximum: f32) -> f32 {
        // Compare against the previous extent so arriving content does not look
        // like the user scrolled upward. Native wheel/keyboard input wins.
        if (offset - self.previous_offset).abs() > 1.0 {
            self.following = offset >= self.previous_max - 1.0;
        }
        if self.requested {
            self.following = true;
            self.requested = false;
        }
        let next = if self.following {
            self.seen = self.revision;
            maximum
        } else {
            offset.clamp(0.0, maximum)
        };
        self.previous_max = maximum;
        self.previous_offset = next;
        next
    }
}

/// Native feed behavior; add beside GameUiPlugin in each adopting composition root.
pub struct GameUiFeedPlugin;
impl Plugin for GameUiFeedPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, follow.after(bevy::ui::UiSystems::PostLayout));
    }
}
fn follow(mut feeds: Query<(&ComputedNode, &mut ScrollPosition, &mut UiFeedScroll)>) {
    for (node, mut scroll, mut feed) in &mut feeds {
        if node.size().y <= 0.0 {
            continue;
        }
        let max = ((node.content_size().y - node.size().y) * node.inverse_scale_factor).max(0.0);
        let next = feed.resolve(scroll.y, max);
        if scroll.y != next {
            scroll.y = next;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arrivals_preserve_reading_position_until_explicit_latest() {
        let mut feed = UiFeedScroll {
            revision: 1,
            ..default()
        };
        assert_eq!(feed.resolve(0.0, 100.0), 100.0);
        assert_eq!(feed.resolve(40.0, 100.0), 40.0);
        feed.revision = 3;
        assert_eq!(feed.resolve(40.0, 200.0), 40.0);
        assert_eq!(feed.unread(), 2);
        feed.jump_to_latest();
        assert_eq!(feed.resolve(40.0, 200.0), 200.0);
        assert_eq!(feed.unread(), 0);
        feed.revision = 4;
        assert_eq!(feed.resolve(200.0, 250.0), 250.0);
    }
}
