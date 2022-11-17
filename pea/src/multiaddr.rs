use libp2p::{multiaddr::Protocol, Multiaddr};
pub fn filter_ip(multiaddr: Multiaddr) -> Option<Multiaddr> {
    let components = multiaddr.iter().collect::<Vec<_>>();
    let mut multiaddr: Multiaddr = "".parse().unwrap();
    match components.get(0) {
        Some(Protocol::Ip4(ip)) => multiaddr.push(Protocol::Ip4(*ip)),
        Some(Protocol::Ip6(ip)) => multiaddr.push(Protocol::Ip6(*ip)),
        _ => return None,
    };
    Some(multiaddr)
}
pub fn filter_ip_port(multiaddr: Multiaddr) -> Option<Multiaddr> {
    let components = multiaddr.iter().collect::<Vec<_>>();
    let mut multiaddr: Multiaddr = "".parse().unwrap();
    match components.get(0) {
        Some(Protocol::Ip4(ip)) => multiaddr.push(Protocol::Ip4(*ip)),
        Some(Protocol::Ip6(ip)) => multiaddr.push(Protocol::Ip6(*ip)),
        _ => return None,
    };
    match components.get(1) {
        Some(Protocol::Tcp(port)) => {
            if port == &9333_u16 {
                return Some(multiaddr);
            }
            multiaddr.push(Protocol::Tcp(*port))
        }
        _ => return Some(multiaddr),
    };
    Some(multiaddr)
}
pub fn has_port(multiaddr: &Multiaddr) -> bool {
    let components = multiaddr.iter().collect::<Vec<_>>();
    match components.get(1) {
        Some(Protocol::Tcp(_)) => true,
        _ => false,
    }
}
