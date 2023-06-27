use libp2p::multiaddr::Multiaddr;
use libp2p::multiaddr::Protocol;
use std::net::IpAddr;
pub const MAINNET_PORT: u16 = 2020;
pub const TESTNET_PORT: u16 = 3030;
pub trait ToMultiaddr {
    fn multiaddr(&self, testnet: bool) -> Multiaddr;
}
pub trait ToIpAddr {
    fn ip_addr(&self) -> Option<IpAddr>;
}
impl ToMultiaddr for IpAddr {
    fn multiaddr(&self, testnet: bool) -> Multiaddr {
        let port = if testnet { TESTNET_PORT } else { MAINNET_PORT };
        let mut multiaddr = Multiaddr::empty();
        match self {
            IpAddr::V4(ip) => {
                multiaddr.push(Protocol::Ip4(*ip));
            }
            IpAddr::V6(ip) => {
                multiaddr.push(Protocol::Ip6(*ip));
            }
        }
        multiaddr.push(Protocol::Tcp(port));
        multiaddr
    }
}
impl ToIpAddr for Multiaddr {
    fn ip_addr(&self) -> Option<IpAddr> {
        match self.iter().collect::<Vec<_>>().first() {
            Some(Protocol::Ip4(ip)) => Some(IpAddr::V4(*ip)),
            Some(Protocol::Ip6(ip)) => Some(IpAddr::V6(*ip)),
            _ => None,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_to_ip_addr() {
        assert_eq!("".parse::<Multiaddr>().unwrap().ip_addr(), None);
        assert_eq!(
            format!("/tcp/{MAINNET_PORT}")
                .parse::<Multiaddr>()
                .unwrap()
                .ip_addr(),
            None
        );
        assert_eq!(
            "/ip4/0.0.0.0".parse::<Multiaddr>().unwrap().ip_addr(),
            Some("0.0.0.0".parse().unwrap())
        );
        assert_eq!(
            format!("/ip4/0.0.0.0/tcp/{MAINNET_PORT}")
                .parse::<Multiaddr>()
                .unwrap()
                .ip_addr(),
            Some("0.0.0.0".parse().unwrap())
        );
        assert_eq!(
            "/ip6/::".parse::<Multiaddr>().unwrap().ip_addr(),
            Some("::".parse().unwrap())
        );
        assert_eq!(
            format!("/ip6/::/tcp/{MAINNET_PORT}")
                .parse::<Multiaddr>()
                .unwrap()
                .ip_addr(),
            Some("::".parse().unwrap())
        );
    }
    #[test]
    fn test_from_ip_addr() {
        assert_eq!(
            "0.0.0.0".parse::<IpAddr>().unwrap().multiaddr(false),
            format!("/ip4/0.0.0.0/tcp/{MAINNET_PORT}")
                .parse::<Multiaddr>()
                .unwrap()
        );
        assert_eq!(
            "::".parse::<IpAddr>().unwrap().multiaddr(false),
            format!("/ip6/::/tcp/{MAINNET_PORT}")
                .parse::<Multiaddr>()
                .unwrap()
        );
    }
}
