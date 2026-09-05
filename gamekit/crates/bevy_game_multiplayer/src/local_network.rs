//! Cross-platform enumeration of addresses suitable for an advertised local route.

use std::{fmt, io, net::IpAddr, num::NonZeroU32};

/// Returns active, non-loopback, non-point-to-point addresses in deterministic preference order.
///
/// Private IPv4 addresses are preferred for the common LAN-hosting case. Public IPv4,
/// unique-local IPv6, and other usable IPv6 addresses remain available to callers that
/// need an explicit route. Link-local addresses are excluded because a connection code
/// cannot preserve the interface scope required to use them safely.
pub fn local_network_addresses() -> Result<Vec<IpAddr>, LocalNetworkAddressError> {
    let mut addresses = if_addrs::get_if_addrs()
        .map_err(LocalNetworkAddressError::Enumerate)?
        .into_iter()
        .filter(|interface| {
            interface.is_oper_up()
                && !interface.is_loopback()
                && !interface.is_p2p()
                && !interface.is_link_local()
        })
        .map(|interface| interface.ip())
        .filter(|address| !address.is_unspecified() && !address.is_multicast())
        .collect::<Vec<_>>();
    addresses.sort_by_key(|address| (address_rank(*address), address_bytes(*address)));
    addresses.dedup();
    Ok(addresses)
}

/// Returns the operating-system interface index owning a concrete local address.
///
/// Native DNS-SD implementations use this index to keep a LAN advertisement on
/// the same interface as its advertised game route.
pub fn local_network_interface_index(
    address: IpAddr,
) -> Result<Option<NonZeroU32>, LocalNetworkAddressError> {
    Ok(if_addrs::get_if_addrs()
        .map_err(LocalNetworkAddressError::Enumerate)?
        .into_iter()
        .find(|interface| interface.ip() == address)
        .and_then(|interface| interface.index)
        .and_then(NonZeroU32::new))
}

fn address_rank(address: IpAddr) -> u8 {
    match address {
        IpAddr::V4(address) if address.is_private() => 0,
        IpAddr::V4(_) => 1,
        IpAddr::V6(address) if (address.segments()[0] & 0xfe00) == 0xfc00 => 2,
        IpAddr::V6(_) => 3,
    }
}

fn address_bytes(address: IpAddr) -> [u8; 16] {
    match address {
        IpAddr::V4(address) => address.to_ipv6_mapped().octets(),
        IpAddr::V6(address) => address.octets(),
    }
}

/// Failure to inspect local network interfaces.
#[derive(Debug)]
pub enum LocalNetworkAddressError {
    /// The operating system did not return its interface table.
    Enumerate(io::Error),
}

impl fmt::Display for LocalNetworkAddressError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Enumerate(error) => {
                write!(formatter, "could not inspect local addresses: {error}")
            }
        }
    }
}

impl std::error::Error for LocalNetworkAddressError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preference_places_private_ipv4_before_other_routes() {
        let mut addresses = [
            "2001:db8::2".parse().expect("IPv6 fixture"),
            "203.0.113.2".parse().expect("IPv4 fixture"),
            "192.168.1.20".parse().expect("private fixture"),
            "fd00::2".parse().expect("ULA fixture"),
        ];
        addresses.sort_by_key(|address| (address_rank(*address), address_bytes(*address)));
        assert_eq!(
            addresses.first().copied().expect("first address"),
            "192.168.1.20".parse::<IpAddr>().expect("private fixture")
        );
        assert_eq!(
            addresses.get(1).copied().expect("second address"),
            "203.0.113.2".parse::<IpAddr>().expect("public fixture")
        );
        assert_eq!(
            addresses.get(2).copied().expect("third address"),
            "fd00::2".parse::<IpAddr>().expect("ULA fixture")
        );
    }
}
