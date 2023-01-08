use libp2p::{multiaddr::Protocol, Multiaddr};
use std::net::IpAddr;
pub fn filter_ip(multiaddr: &Multiaddr) -> Option<Multiaddr> {
    let components = multiaddr.iter().collect::<Vec<_>>();
    let mut multiaddr: Multiaddr = "".parse().unwrap();
    match components.get(0) {
        Some(Protocol::Ip4(ip)) => multiaddr.push(Protocol::Ip4(*ip)),
        Some(Protocol::Ip6(ip)) => multiaddr.push(Protocol::Ip6(*ip)),
        _ => return None,
    };
    Some(multiaddr)
}
pub fn filter_ip_port(multiaddr: &Multiaddr) -> Option<Multiaddr> {
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
    matches!(components.get(1), Some(Protocol::Tcp(_)))
}
pub fn addr(multiaddr: &Multiaddr) -> Option<IpAddr> {
    match multiaddr.iter().collect::<Vec<_>>().first() {
        Some(Protocol::Ip4(ip)) => Some(IpAddr::V4(*ip)),
        Some(Protocol::Ip6(ip)) => Some(IpAddr::V6(*ip)),
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_filter_ip() {
        assert_eq!(filter_ip(&"".parse::<Multiaddr>().unwrap()), None);
        assert_eq!(filter_ip(&"/tcp/9333".parse::<Multiaddr>().unwrap()), None);
        assert_eq!(
            filter_ip(&"/ip4/0.0.0.0/tcp/9333".parse::<Multiaddr>().unwrap()).unwrap(),
            "/ip4/0.0.0.0".parse::<Multiaddr>().unwrap()
        );
    }
    #[test]
    fn test_filter_ip_port() {
        assert_eq!(filter_ip_port(&"".parse::<Multiaddr>().unwrap()), None);
        assert_eq!(filter_ip_port(&"/tcp/9333".parse::<Multiaddr>().unwrap()), None);
        assert_eq!(
            filter_ip_port(&"/ip4/0.0.0.0".parse::<Multiaddr>().unwrap()).unwrap(),
            "/ip4/0.0.0.0".parse::<Multiaddr>().unwrap()
        );
        assert_eq!(
            filter_ip_port(&"/ip4/0.0.0.0/tcp/9333".parse::<Multiaddr>().unwrap()).unwrap(),
            "/ip4/0.0.0.0".parse::<Multiaddr>().unwrap()
        );
        assert_eq!(
            filter_ip_port(&"/ip4/0.0.0.0/tcp/9334".parse::<Multiaddr>().unwrap()).unwrap(),
            "/ip4/0.0.0.0/tcp/9334".parse::<Multiaddr>().unwrap()
        );
    }
    #[test]
    fn test_has_port() {
        assert_eq!(has_port(&"".parse::<Multiaddr>().unwrap()), false);
        assert_eq!(has_port(&"/ip4/0.0.0.0".parse::<Multiaddr>().unwrap()), false);
        assert_eq!(has_port(&"/tcp/9333".parse::<Multiaddr>().unwrap()), false);
        assert_eq!(has_port(&"/ip4/0.0.0.0/tcp/9333".parse::<Multiaddr>().unwrap()), true);
    }
}
