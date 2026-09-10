//! Game-owned provider composition; listing never grants admission.

use super::*;
use bevy_gamekit::discovery::{
    Compatibility, DiscoveryObservation, DiscoveryProviderId, DiscoverySource, MdnsAdvertiser,
    MdnsBrowser, MdnsSessionAdvertisement, TailnetBrowser, TailnetResponder, TailscaleCli,
    TailscaleStatusTask,
};

#[derive(Default)]
pub(super) struct HostProviders {
    mdns: Option<MdnsAdvertiser>,
    tailnet: Option<TailnetResponder>,
    status: Option<TailscaleStatusTask>,
    target: Option<DiscoveredDirectTarget>,
    notices: BTreeMap<&'static str, String>,
}

impl HostProviders {
    pub fn start(
        settings: &HostSettings,
        metadata: SessionMetadata,
        target: DiscoveredDirectTarget,
    ) -> Self {
        let mut providers = Self {
            target: Some(target.clone()),
            ..Default::default()
        };
        if settings.lan {
            let addresses = local_network_addresses().unwrap_or_default();
            let requested = target.endpoint.host().parse::<std::net::IpAddr>().ok();
            let address = requested
                .filter(|ip| addresses.contains(ip) && !tailnet_ip(*ip))
                .or_else(|| addresses.into_iter().find(|ip| !tailnet_ip(*ip)));
            let result = address
                .ok_or_else(|| "No non-tailnet LAN address is available.".to_owned())
                .and_then(|ip| retarget(&target, ip))
                .and_then(|target| {
                    MdnsSessionAdvertisement::new(metadata, target).map_err(|e| e.to_string())
                })
                .and_then(|advertisement| {
                    MdnsAdvertiser::start(advertisement).map_err(|e| e.to_string())
                });
            match result {
                Ok(mdns) => providers.mdns = Some(mdns),
                Err(error) => {
                    providers.notices.insert("LAN", error);
                }
            }
        }
        if settings.tailnet {
            providers.status = Some(TailscaleCli::default().request_status());
        }
        providers
    }
    pub fn refresh(&mut self, metadata: &SessionMetadata) {
        if let Some(mdns) = self.mdns.as_mut() {
            if let Err(error) = mdns.refresh_metadata(metadata.clone()) {
                self.notices.insert("LAN", error.to_string());
            }
        }
        if let Some(tailnet) = self.tailnet.as_mut() {
            if let Err(error) = tailnet.refresh_metadata(metadata.clone()) {
                self.notices.insert("Tailnet", error.to_string());
            }
        }
    }
    fn poll(&mut self, metadata: &SessionMetadata) {
        if let Some(mdns) = self.mdns.as_ref() {
            if let Err(error) = mdns.poll_health() {
                self.notices.insert("LAN", error.to_string());
            }
        }
        if let Some(result) = self.status.as_ref().and_then(TailscaleStatusTask::poll) {
            self.status = None;
            let result = result.map_err(|e| e.to_string()).and_then(|status| {
                let address = status
                    .local_addresses
                    .first()
                    .copied()
                    .ok_or("No tailnet address is available.")?;
                let target =
                    retarget(self.target.as_ref().ok_or("No session identity.")?, address)?;
                TailnetResponder::bind(address, metadata.clone(), target).map_err(|e| e.to_string())
            });
            match result {
                Ok(responder) => self.tailnet = Some(responder),
                Err(error) => {
                    self.notices.insert("Tailnet", error);
                }
            }
        }
        if let Some(tailnet) = self.tailnet.as_ref() {
            if let Err(error) = tailnet.poll(64) {
                self.notices.insert("Tailnet", error.to_string());
            }
        }
    }
}

#[derive(Resource)]
pub(super) struct Browser {
    mdns: Option<MdnsBrowser>,
    tailnet: Option<TailnetBrowser>,
    status: Option<TailscaleStatusTask>,
    tailnet_enabled: bool,
    next_refresh: Instant,
}

pub(super) fn browse(world: &mut World, tailnet_enabled: bool) {
    let mdns = match MdnsBrowser::start() {
        Ok(mdns) => Some(mdns),
        Err(error) => {
            unavailable(world, DiscoveryProviderId::MDNS, error.to_string());
            None
        }
    };
    world.insert_resource(Browser {
        mdns,
        tailnet: None,
        status: tailnet_enabled.then(|| TailscaleCli::default().request_status()),
        tailnet_enabled,
        next_refresh: Instant::now() + Duration::from_secs(10),
    });
}

#[cfg(test)]
pub(super) fn browse_fake(world: &mut World) {
    world.insert_resource(Browser {
        mdns: None,
        tailnet: None,
        status: None,
        tailnet_enabled: false,
        next_refresh: Instant::now() + Duration::from_secs(10),
    });
}

pub(super) fn poll(world: &mut World) {
    if let Some(mut hosted) = world.remove_resource::<Hosted>() {
        hosted.providers.poll(&hosted.metadata);
        world.resource_mut::<LabyrinthView>().provider_notices = hosted
            .providers
            .notices
            .iter()
            .map(|(name, text)| format!("{name}: {text}"))
            .collect();
        world.insert_resource(hosted);
    }
    let Some(mut browser) = world.remove_resource::<Browser>() else {
        return;
    };
    let at = world.resource::<Time<Real>>().elapsed();
    if let Some(mdns) = browser.mdns.as_mut() {
        match mdns.poll(at, unix_now()) {
            Ok(found) => {
                for observation in found {
                    world.write_message(observation);
                }
            }
            Err(error) => {
                unavailable(world, DiscoveryProviderId::MDNS, error.to_string());
                browser.mdns = None;
            }
        }
    }
    if browser.tailnet_enabled && browser.status.is_none() && Instant::now() >= browser.next_refresh
    {
        browser.status = Some(TailscaleCli::default().request_status());
    }
    if let Some(result) = browser.status.as_ref().and_then(TailscaleStatusTask::poll) {
        browser.status = None;
        browser.next_refresh = Instant::now() + Duration::from_secs(10);
        let result = result.map_err(|e| e.to_string()).and_then(|status| {
            if browser.tailnet.is_none() {
                let address = status
                    .local_addresses
                    .first()
                    .copied()
                    .ok_or("No local tailnet address.")?;
                browser.tailnet = Some(TailnetBrowser::bind(address).map_err(|e| e.to_string())?);
            }
            if let Some(tailnet) = browser.tailnet.as_mut() {
                tailnet
                    .refresh(&status.peers, GAME_ID)
                    .map_err(|e| e.to_string())?;
            }
            Ok(())
        });
        if let Err(error) = result {
            unavailable(world, DiscoveryProviderId::TAILSCALE, error);
            browser.tailnet = None;
        }
    }
    if let Some(tailnet) = browser.tailnet.as_ref() {
        match tailnet.poll(at, unix_now()) {
            Ok(found) => {
                for observation in found {
                    world.write_message(observation);
                }
            }
            Err(error) => unavailable(world, DiscoveryProviderId::TAILSCALE, error.to_string()),
        }
    }
    world.insert_resource(browser);
}

pub(super) fn project(world: &mut World) {
    if !world.contains_resource::<Browser>() {
        return;
    }
    let at = world.resource::<Time<Real>>().elapsed();
    let registry = world.resource::<DiscoveryRegistry>();
    let listings = registry
        .sessions(at)
        .into_iter()
        .map(|session| {
            let sources = session
                .sources
                .iter()
                .map(|source| match source {
                    DiscoverySource::Lan => "LAN",
                    DiscoverySource::Tailnet => "TAILNET",
                    DiscoverySource::Service => "SERVICE",
                })
                .collect::<Vec<_>>()
                .join(" + ");
            ListingView {
                id: session.session_id,
                compatible: session.compatibility == Compatibility::Compatible,
                label: format!(
                    "{} [{sources}] {}/{} | {} | {}s",
                    session.metadata.display_name(),
                    session.metadata.claimed_players(),
                    session.metadata.player_capacity(),
                    if session.metadata.password_required() {
                        "LOCKED"
                    } else {
                        "PRIVATE INVITE"
                    },
                    session.freshness.as_secs()
                ),
            }
        })
        .collect();
    let notices = registry.provider_notices().values().cloned().collect();
    let mut view = world.resource_mut::<LabyrinthView>();
    view.listings = listings;
    view.provider_notices = notices;
}

fn unavailable(world: &mut World, provider: DiscoveryProviderId, reason: String) {
    world.write_message(DiscoveryObservation::Unavailable { provider, reason });
}
fn retarget(
    target: &DiscoveredDirectTarget,
    ip: std::net::IpAddr,
) -> Result<DiscoveredDirectTarget, String> {
    Ok(DiscoveredDirectTarget {
        endpoint: DirectEndpoint::new(ip.to_string(), target.endpoint.port())
            .map_err(|e| e.to_string())?,
        ..target.clone()
    })
}
fn tailnet_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(ip) => {
            let [a, b, _, _] = ip.octets();
            a == 100 && (64..128).contains(&b)
        }
        std::net::IpAddr::V6(ip) => ip
            .segments()
            .first()
            .is_some_and(|segment| *segment == 0xfd7a),
    }
}
