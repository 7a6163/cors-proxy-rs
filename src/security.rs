use std::net::{IpAddr, ToSocketAddrs};

use crate::error::ProxyError;

pub fn validate_target_ip(host: &str) -> Result<(), ProxyError> {
    let addr = format!("{host}:80");
    let resolved = addr
        .to_socket_addrs()
        .map_err(|_| ProxyError::InvalidTargetUrl(format!("Cannot resolve host: {host}")))?;

    for socket_addr in resolved {
        if is_private_ip(socket_addr.ip()) {
            return Err(ProxyError::PrivateIpBlocked);
        }
    }

    Ok(())
}

fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.octets()[0] == 169 && v4.octets()[1] == 254 // link-local
        }
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified() || is_unique_local_v6(v6),
    }
}

fn is_unique_local_v6(v6: std::net::Ipv6Addr) -> bool {
    // fc00::/7
    let first_byte = v6.octets()[0];
    (first_byte & 0xfe) == 0xfc
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_loopback_is_private() {
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::LOCALHOST)));
        assert!(is_private_ip(IpAddr::V6(Ipv6Addr::LOCALHOST)));
    }

    #[test]
    fn test_private_ranges() {
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))));
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1))));
    }

    #[test]
    fn test_public_ip_is_not_private() {
        assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
    }

    #[test]
    fn test_unique_local_v6() {
        let addr: Ipv6Addr = "fc00::1".parse().unwrap();
        assert!(is_private_ip(IpAddr::V6(addr)));

        let addr: Ipv6Addr = "fd00::1".parse().unwrap();
        assert!(is_private_ip(IpAddr::V6(addr)));
    }

    #[test]
    fn test_public_v6_is_not_private() {
        let addr: Ipv6Addr = "2001:4860:4860::8888".parse().unwrap();
        assert!(!is_private_ip(IpAddr::V6(addr)));
    }
}
