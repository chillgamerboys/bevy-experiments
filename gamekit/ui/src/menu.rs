//! Shared local navigation and menu layouts. No simulation or network authority.

use crate::{modal, UiPanel, UiSkin};
use bevy::prelude::*;

/// Local page history with game-owned routes. Reopening a page returns to it
/// instead of accumulating duplicates. No page transition pauses a game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiMenuStack<R> {
    pages: Vec<R>,
}

impl<R> Default for UiMenuStack<R> {
    fn default() -> Self {
        Self { pages: Vec::new() }
    }
}

impl<R: PartialEq> UiMenuStack<R> {
    /// Open a page; retain at most sixteen navigation levels.
    pub fn open(&mut self, page: R) {
        if let Some(index) = self.pages.iter().position(|item| item == &page) {
            self.pages.truncate(index + 1);
        } else if self.pages.len() < 16 {
            self.pages.push(page);
        }
    }
    /// Return to the previous page, or close the final page.
    pub fn back(&mut self) {
        self.pages.pop();
    }
    /// Dismiss this local menu without altering any game/session state.
    pub fn close(&mut self) {
        self.pages.clear();
    }
    /// Visible page, if any.
    #[must_use]
    pub fn current(&self) -> Option<&R> {
        self.pages.last()
    }
    /// Whether this local menu captures gameplay input.
    #[must_use]
    pub fn is_open(&self) -> bool {
        !self.pages.is_empty()
    }
}

/// Full-screen local modal backdrop, with shared focus trapping/restoration.
/// Attach game-owned actions and use a [`menu_panel`] for its content.
pub fn menu_overlay(name: impl Into<String>) -> impl Bundle {
    (modal(name), UiSkin::Modal)
}

/// Consistent scrollable menu surface. Palette/fonts remain semantic skin tokens;
/// consumers may replace the Node or skin without changing navigation.
/// The default height limit assumes a definite-height modal parent. In an
/// auto-height page with an outer scroller, override `max_height` to `Val::Auto`
/// and `flex_shrink` to zero so wrapped content determines its own height.
pub fn menu_panel(name: impl Into<String>) -> impl Bundle {
    (
        Name::new(name.into()),
        UiPanel,
        UiSkin::Panel,
        menu_panel_node(),
        BackgroundColor::default(),
        BorderColor::default(),
    )
}

/// Baseline layout shared by main menus, settings and confirmation dialogs.
#[must_use]
pub fn menu_panel_node() -> Node {
    Node {
        // A definite preferred width lets wrapped text participate in intrinsic
        // height measurement; the percentage cap still fits narrow windows.
        width: Val::Px(620.0),
        max_width: Val::Percent(90.0),
        max_height: Val::Percent(90.0),
        min_height: Val::Px(0.0),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(14.0),
        padding: UiRect::all(Val::Px(24.0)),
        overflow: Overflow::scroll_y(),
        ..default()
    }
}

/// Vertical action-list layout, usable independently of a modal.
#[must_use]
pub fn menu_actions_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(10.0),
        flex_shrink: 0.0,
        ..default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn navigation_is_local_bounded_and_returns_to_existing_pages() {
        let mut menu = UiMenuStack::default();
        menu.open("game");
        menu.open("settings");
        menu.open("confirm");
        menu.open("settings");
        assert_eq!(menu.pages, ["game", "settings"]);
        menu.back();
        assert_eq!(menu.current(), Some(&"game"));
        menu.back();
        menu.back();
        assert!(!menu.is_open());
        for _ in 0..100 {
            menu.open("same");
        }
        assert_eq!(menu.pages.len(), 1);
        menu.close();
        assert!(!menu.is_open());
        let mut bounded = UiMenuStack::default();
        for page in 0..100 {
            bounded.open(page);
        }
        assert_eq!(bounded.pages.len(), 16);
    }
    #[test]
    fn templates_are_native_and_do_not_contain_game_state() {
        let mut world = World::new();
        let root = world.spawn(menu_overlay("menu")).id();
        let panel = world.spawn(menu_panel("page")).id();
        assert!(world.get::<crate::UiModalScope>(root).is_some());
        assert!(world.get::<Node>(panel).is_some());
    }
}
