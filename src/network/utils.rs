use std::net::{Ipv4Addr, Ipv6Addr};

use libp2p::Multiaddr;

pub fn get_external_addrs(addr: &[Multiaddr]) -> Vec<Multiaddr> {
    addr.iter()
        .filter_map(|addr| {
            if is_public_ip(addr) {
                Some((*addr).clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
}

pub fn is_public_ip(addr: &Multiaddr) -> bool {
    let addr_str = addr.to_string();
    if addr_str.starts_with("/ip4/") {
        let addr_str = addr_str.replace("/ip4/", "");
        let addr_str = addr_str.split('/').collect::<Vec<_>>()[0];
        let ip: Ipv4Addr = addr_str.parse().unwrap();
        if u32::from_be_bytes(ip.octets()) == 0xc0000009
            || u32::from_be_bytes(ip.octets()) == 0xc000000a
        {
            return true;
        }
        let is_shared = ip.octets()[0] == 100 && (ip.octets()[1] & 0b1100_0000 == 0b0100_0000);
        !ip.is_private()
            && !ip.is_loopback()
            && !ip.is_link_local()
            && !ip.is_broadcast()
            && !ip.is_documentation()
            && !is_shared
    } else if addr_str.starts_with("/ip6/") {
        //TODO? Handle ipv6 cases
        let addr_str = addr_str.replace("/ip6/", "");
        let addr_str = addr_str.split('/').collect::<Vec<_>>()[0];
        let ip: Ipv6Addr = addr_str.parse().unwrap();
        !ip.is_loopback() && !ip.is_unspecified()
    } else {
        false
    }
}
