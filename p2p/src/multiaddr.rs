use libp2p::multiaddr::Protocol;
use libp2p::Multiaddr;
use std::net::IpAddr;
pub fn to_ip_addr(multiaddr: &Multiaddr) -> Option<IpAddr> {
    match multiaddr.iter().collect::<Vec<_>>().first() {
        Some(Protocol::Ip4(ip)) => Some(IpAddr::V4(*ip)),
        Some(Protocol::Ip6(ip)) => Some(IpAddr::V6(*ip)),
        _ => None,
    }
}
pub fn from_ip_addr(ip_addr: &IpAddr) -> Multiaddr {
    let mut multiaddr = Multiaddr::empty();
    let protocol = match ip_addr {
        IpAddr::V4(ip) => Protocol::Ip4(*ip),
        IpAddr::V6(ip) => Protocol::Ip6(*ip),
    };
    multiaddr.push(protocol);
    multiaddr.push(Protocol::Tcp(9333));
    multiaddr
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_to_ip_addr() {
        assert_eq!(to_ip_addr(&"".parse::<Multiaddr>().unwrap()), None);
        assert_eq!(to_ip_addr(&"/tcp/9333".parse::<Multiaddr>().unwrap()), None);
        assert_eq!(
            to_ip_addr(&"/ip4/0.0.0.0".parse::<Multiaddr>().unwrap()),
            Some("0.0.0.0".parse().unwrap())
        );
        assert_eq!(
            to_ip_addr(&"/ip4/0.0.0.0/tcp/9333".parse::<Multiaddr>().unwrap()),
            Some("0.0.0.0".parse().unwrap())
        );
        assert_eq!(
            to_ip_addr(&"/ip6/::".parse::<Multiaddr>().unwrap()),
            Some("::".parse().unwrap())
        );
        assert_eq!(
            to_ip_addr(&"/ip6/::/tcp/9333".parse::<Multiaddr>().unwrap()),
            Some("::".parse().unwrap())
        );
    }
    #[test]
    fn test_from_ip_addr() {
        assert_eq!(
            from_ip_addr(&"0.0.0.0".parse().unwrap()),
            "/ip4/0.0.0.0/tcp/9333".parse::<Multiaddr>().unwrap()
        );
        assert_eq!(
            from_ip_addr(&"::".parse().unwrap()),
            "/ip6/::/tcp/9333".parse::<Multiaddr>().unwrap()
        );
    }
}
