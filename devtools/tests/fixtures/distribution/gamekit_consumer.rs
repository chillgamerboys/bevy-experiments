//! Synthetic external consumer; not a game or an alternative rules implementation.

#[cfg(all(test, feature = "pure"))]
#[test]
fn pure_algorithms_and_persistence() -> Result<(), Box<dyn std::error::Error>> {
    use bevy_gamekit::{hex::Hex, turns::TurnOrder};
    assert_eq!(Hex::ZERO.distance(Hex { q: 1, r: 0 }), 1);
    let mut turns = TurnOrder::new(vec![1_u8, 2])?;
    let transition = turns.advance()?;
    assert_eq!(transition.current, 2);
    let stored = serde_json::to_string(&turns)?;
    assert_eq!(serde_json::from_str::<TurnOrder<u8>>(&stored)?, turns);
    Ok(())
}

#[cfg(all(test, feature = "ui"))]
#[test]
fn headless_ui_without_a_game_or_network() {
    use bevy_gamekit::{testing, ui};
    let mut app = testing::TestAppBuilder::new().with_ui(1280, 720).build();
    let child = app.world_mut().spawn(ui::button("ProbeAction")).id();
    app.world_mut()
        .spawn(ui::screen_root("ProbeMenu"))
        .add_child(child);
    testing::run_frames(&mut app, 3);
    let snapshot = testing::ui_tree_snapshot(app.world_mut()).to_string();
    assert!(snapshot.contains("ProbeMenu/ProbeAction [action]"));
}

#[cfg(all(test, feature = "network"))]
#[test]
fn native_adapters_compile_and_memory_link_is_independent() -> Result<(), Box<dyn std::error::Error>>
{
    use bevy_gamekit::{discovery, multiplayer, session};
    // Type-check forwarded APIs without opening sockets or invoking Tailscale.
    let _ = std::mem::size_of::<multiplayer::PreparedDirectHost>();
    let _ = std::mem::size_of::<discovery::DiscoveryPlugin>();
    let _ = std::mem::size_of::<discovery::MdnsBrowser>();
    let _ = std::mem::size_of::<discovery::TailscaleCli>();
    let _ = std::mem::size_of::<session::SessionId>();
    let (host, guest) = multiplayer::InMemorySessionLink::pair(2, 64);
    host.send(vec![1, 2, 3])?;
    assert_eq!(guest.try_receive()?, Some(vec![1, 2, 3]));
    Ok(())
}
