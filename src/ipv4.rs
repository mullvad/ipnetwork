use std::fmt;
use std::net::{Ipv4Addr};

use common::{IpNetworkError, cidr_parts, parse_prefix};

const IPV4_BITS: u8 = 32;

#[derive(Debug,Clone,Copy,Hash,PartialEq,Eq)]
pub struct Ipv4Network {
    addr: Ipv4Addr,
    prefix: u8,
}

impl Ipv4Network {
    /// Constructs a new `Ipv4Network` from any `Ipv4Addr` and a prefix denoting the network size.
    /// If the prefix is larger than 32 this will return an `IpNetworkError::InvalidPrefix`.
    pub fn new(addr: Ipv4Addr, prefix: u8) -> Result<Ipv4Network, IpNetworkError> {
        if prefix > IPV4_BITS {
            Err(IpNetworkError::InvalidPrefix)
        } else {
            Ok(Ipv4Network {
                addr: addr,
                prefix: prefix,
            })
        }
    }

    /// Creates an `Ipv4Network` from parsing a string in CIDR notation.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::Ipv4Addr;
    /// use ipnetwork::Ipv4Network;
    ///
    /// let new = Ipv4Network::new(Ipv4Addr::new(10, 1, 9, 32), 16).unwrap();
    /// let from_cidr = Ipv4Network::from_cidr("10.1.9.32/16").unwrap();
    /// assert_eq!(new.ip(), from_cidr.ip());
    /// assert_eq!(new.prefix(), from_cidr.prefix());
    /// ```
    pub fn from_cidr(cidr: &str) -> Result<Ipv4Network, IpNetworkError> {
        let (addr_str, prefix_str) = try!(cidr_parts(cidr));
        let addr = try!(Self::parse_addr(addr_str));
        let prefix = try!(parse_prefix(prefix_str, IPV4_BITS));
        Self::new(addr, prefix)
    }

    /// Returns an iterator over `Ipv4Network`. Each call to `next` will return the next
    /// `Ipv4Addr` in the given network. `None` will be returned when there are no more
    /// addresses.
    pub fn iter(&self) -> Ipv4NetworkIterator {
        let (_, start) = self.network();
        let end = start as u64 + self.size();
        Ipv4NetworkIterator {
            next: start as u64,
            end: end,
        }
    }

    pub fn ip(&self) -> Ipv4Addr {
        self.addr
    }

    pub fn prefix(&self) -> u8 {
        self.prefix
    }

    /// Returns the mask for this `Ipv4Network`.
    /// That means the `prefix` most significant bits will be 1 and the rest 0
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::Ipv4Addr;
    /// use ipnetwork::Ipv4Network;
    ///
    /// let net = Ipv4Network::from_cidr("127.0.0.0/16").unwrap();
    /// let (mask_ip, mask_u32) = net.mask();
    /// assert_eq!(mask_ip, Ipv4Addr::new(255, 255, 0, 0));
    /// assert_eq!(mask_u32, 0xffff0000);
    /// ```
    pub fn mask(&self) -> (Ipv4Addr, u32) {
        let prefix = self.prefix;
        let mask = !(0xffffffff as u64 >> prefix) as u32;
        (Ipv4Addr::from(mask), mask)
    }

    /// Returns the address of the network denoted by this `Ipv4Network`.
    /// This means the lowest possible IPv4 address inside of the network.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::Ipv4Addr;
    /// use ipnetwork::Ipv4Network;
    ///
    /// let net = Ipv4Network::from_cidr("10.1.9.32/16").unwrap();
    /// let (net_ip, net_u32) = net.network();
    /// assert_eq!(net_ip, Ipv4Addr::new(10, 1, 0, 0));
    /// assert_eq!(net_u32, (10 << 24) + (1 << 16));
    /// ```
    pub fn network(&self) -> (Ipv4Addr, u32) {
        let (_, mask) = self.mask();
        let ip = u32::from(self.addr) & mask;
        (Ipv4Addr::from(ip), ip)
    }

    /// Returns the broadcasting address of this `Ipv4Network`.
    /// This means the highest possible IPv4 address inside of the network.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::Ipv4Addr;
    /// use ipnetwork::Ipv4Network;
    ///
    /// let net = Ipv4Network::from_cidr("10.9.0.32/16").unwrap();
    /// let (bcast_ip, bcast_u32) = net.broadcast();
    /// assert_eq!(bcast_ip, Ipv4Addr::new(10, 9, 255, 255));
    /// assert_eq!(bcast_u32, (10 << 24) + (9 << 16) + 0xffff);
    /// ```
    pub fn broadcast(&self) -> (Ipv4Addr, u32) {
        let (_, mask) = self.mask();
        let broadcast = u32::from(self.addr) | !mask;
        (Ipv4Addr::from(broadcast), broadcast)
    }

    /// Checks if a given `Ipv4Addr` is in this `Ipv4Network`
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::Ipv4Addr;
    /// use ipnetwork::Ipv4Network;
    ///
    /// let net = Ipv4Network::from_cidr("127.0.0.0/24").unwrap();
    /// assert!(net.contains(Ipv4Addr::new(127, 0, 0, 70)));
    /// assert!(!net.contains(Ipv4Addr::new(127, 0, 1, 70)));
    /// ```
    pub fn contains(&self, ip: Ipv4Addr) -> bool {
        let (_, net) = self.network();
        let (_, mask) = self.mask();
        (u32::from(ip) & mask) == net
    }

    /// Returns number of possible host addresses in this `Ipv4Network`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::Ipv4Addr;
    /// use ipnetwork::Ipv4Network;
    ///
    /// let net = Ipv4Network::from_cidr("10.1.0.0/16").unwrap();
    /// assert_eq!(net.size(), 65536);
    ///
    /// let tinynet = Ipv4Network::from_cidr("0.0.0.0/32").unwrap();
    /// assert_eq!(tinynet.size(), 1);
    /// ```
    pub fn size(&self) -> u64 {
        let host_bits = (IPV4_BITS - self.prefix) as u32;
        (2 as u64).pow(host_bits)
    }

    /// Returns the `n`:th address within this network.
    /// The adresses are indexed from 0 and `n` must be smaller than the size of the network.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::Ipv4Addr;
    /// use ipnetwork::Ipv4Network;
    ///
    /// let net = Ipv4Network::from_cidr("192.168.0.0/24").unwrap();
    /// assert_eq!(net.nth(0).unwrap(), Ipv4Addr::new(192, 168, 0, 0));
    /// assert_eq!(net.nth(15).unwrap(), Ipv4Addr::new(192, 168, 0, 15));
    /// assert!(net.nth(256).is_none());
    ///
    /// let net2 = Ipv4Network::from_cidr("10.0.0.0/16").unwrap();
    /// assert_eq!(net2.nth(256).unwrap(), Ipv4Addr::new(10, 0, 1, 0));
    /// ```
    pub fn nth(&self, n: u32) -> Option<Ipv4Addr> {
        if (n as u64) < self.size() {
            let (_, net) = self.network();
            Some(Ipv4Addr::from(net + n))
        } else {
            None
        }
    }

    fn parse_addr(addr: &str) -> Result<Ipv4Addr, IpNetworkError> {
        let addr_parts = addr.split('.').map(|b| b.parse::<u8>());
        let mut bytes = [0; 4];
        for (i, byte) in addr_parts.enumerate() {
            if i >= 4 {
                return Err(IpNetworkError::InvalidAddr(format!("More than 4 bytes: {}", addr)));
            }
            bytes[i] = try!(byte.map_err(|_| {
                IpNetworkError::InvalidAddr(format!("All bytes not 0-255: {}", addr))
            }));
        }
        Ok(Ipv4Addr::new(bytes[0], bytes[1], bytes[2], bytes[3]))
    }
}

impl fmt::Display for Ipv4Network {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}/{}", self.ip(), self.prefix())
    }
}

pub struct Ipv4NetworkIterator {
    next: u64,
    end: u64,
}

impl Iterator for Ipv4NetworkIterator {
    type Item = Ipv4Addr;

    fn next(&mut self) -> Option<Ipv4Addr> {
        if self.next < self.end {
            let next = Ipv4Addr::from(self.next as u32);
            self.next += 1;
            Some(next)
        } else {
            None
        }
    }
}


#[cfg(test)]
mod test {
    use std::mem;
    use std::collections::HashMap;
    use std::net::Ipv4Addr;
    use super::*;

    #[test]
    fn create_v4() {
        let cidr = Ipv4Network::new(Ipv4Addr::new(77, 88, 21, 11), 24).unwrap();
        assert_eq!(cidr.prefix(), 24);
    }

    #[test]
    fn create_v4_invalid_prefix() {
        let net = Ipv4Network::new(Ipv4Addr::new(0, 0, 0, 0), 33);
        assert!(net.is_err());
    }

    #[test]
    fn parse_v4_0bit() {
        let cidr = Ipv4Network::from_cidr("0/0").unwrap();
        assert_eq!(cidr.ip(), Ipv4Addr::new(0, 0, 0, 0));
        assert_eq!(cidr.prefix(), 0);
    }

    #[test]
    fn parse_v4_24bit() {
        let cidr = Ipv4Network::from_cidr("127.1.0.0/24").unwrap();
        assert_eq!(cidr.ip(), Ipv4Addr::new(127, 1, 0, 0));
        assert_eq!(cidr.prefix(), 24);
    }

    #[test]
    fn parse_v4_32bit() {
        let cidr = Ipv4Network::from_cidr("127.0.0.0/32").unwrap();
        assert_eq!(cidr.ip(), Ipv4Addr::new(127, 0, 0, 0));
        assert_eq!(cidr.prefix(), 32);
    }

    #[test]
    fn parse_v4_fail_addr() {
        let cidr = Ipv4Network::from_cidr("10.a.b/8");
        assert!(cidr.is_err());
    }

    #[test]
    fn parse_v4_fail_addr2() {
        let cidr = Ipv4Network::from_cidr("10.1.1.1.0/8");
        assert!(cidr.is_err());
    }

    #[test]
    fn parse_v4_fail_addr3() {
        let cidr = Ipv4Network::from_cidr("256/8");
        assert!(cidr.is_err());
    }

    #[test]
    fn parse_v4_non_zero_host_bits() {
        let cidr = Ipv4Network::from_cidr("10.1.1.1/24").unwrap();
        assert_eq!(cidr.ip(), Ipv4Addr::new(10, 1, 1, 1));
        assert_eq!(cidr.prefix(), 24);
    }

    #[test]
    fn parse_v4_fail_prefix() {
        let cidr = Ipv4Network::from_cidr("0/39");
        assert!(cidr.is_err());
    }

    #[test]
    fn size_v4_24bit() {
        let net = Ipv4Network::from_cidr("0/24").unwrap();
        assert_eq!(net.size(), 256);
    }

    #[test]
    fn size_v4_1bit() {
        let net = Ipv4Network::from_cidr("0/31").unwrap();
        assert_eq!(net.size(), 2);
    }

    #[test]
    fn size_v4_max() {
        let net = Ipv4Network::from_cidr("0/0").unwrap();
        assert_eq!(net.size(), 4_294_967_296);
    }

    #[test]
    fn size_v4_min() {
        let net = Ipv4Network::from_cidr("0/32").unwrap();
        assert_eq!(net.size(), 1);
    }

    #[test]
    fn nth_v4() {
        let net = Ipv4Network::new(Ipv4Addr::new(127, 0, 0, 0), 24).unwrap();
        assert_eq!(net.nth(0).unwrap(), Ipv4Addr::new(127, 0, 0, 0));
        assert_eq!(net.nth(1).unwrap(), Ipv4Addr::new(127, 0, 0, 1));
        assert_eq!(net.nth(255).unwrap(), Ipv4Addr::new(127, 0, 0, 255));
        assert!(net.nth(256).is_none());
    }

    #[test]
    fn nth_v4_fail() {
        let net = Ipv4Network::new(Ipv4Addr::new(10, 0, 0, 0), 32).unwrap();
        assert!(net.nth(1).is_none());
    }

    #[test]
    fn hash_eq_compatibility_v4() {
        let mut map = HashMap::new();
        let net = Ipv4Network::new(Ipv4Addr::new(127, 0, 0, 1), 16).unwrap();
        map.insert(net, 137);
        let out = map.get(&net).unwrap();
        assert_eq!(137, *out);
    }

    #[test]
    fn copy_compatibility_v4() {
        let net = Ipv4Network::new(Ipv4Addr::new(127, 0, 0, 1), 16).unwrap();
        mem::drop(net);
        assert_eq!(16, net.prefix());
    }

    #[test]
    fn mask_v4() {
        let cidr = Ipv4Network::new(Ipv4Addr::new(74, 125, 227, 0), 29).unwrap();
        let (ip, int) = cidr.mask();
        assert_eq!(ip, Ipv4Addr::new(255, 255, 255, 248));
        assert_eq!(int, 4294967288);
    }

    #[test]
    fn network_v4() {
        let cidr = Ipv4Network::new(Ipv4Addr::new(10, 10, 1, 97), 23).unwrap();
        let (ip, int) = cidr.network();
        assert_eq!(ip, Ipv4Addr::new(10, 10, 0, 0));
        assert_eq!(int, 168427520);
    }

    #[test]
    fn broadcast_v4() {
        let cidr = Ipv4Network::new(Ipv4Addr::new(10, 10, 1, 97), 23).unwrap();
        let (ip, int) = cidr.broadcast();
        assert_eq!(ip, Ipv4Addr::new(10, 10, 1, 255));
        assert_eq!(int, 168428031);
    }

    #[test]
    fn contains_v4() {
        let cidr = Ipv4Network::new(Ipv4Addr::new(74, 125, 227, 0), 25).unwrap();
        let ip = Ipv4Addr::new(74, 125, 227, 4);
        assert!(cidr.contains(ip));
    }

    #[test]
    fn not_contains_v4() {
        let cidr = Ipv4Network::new(Ipv4Addr::new(10, 0, 0, 50), 24).unwrap();
        let ip = Ipv4Addr::new(10, 1, 0, 1);
        assert!(!cidr.contains(ip));
    }

    #[test]
    fn iterator_v4() {
        let cidr = Ipv4Network::from_cidr("192.168.122.0/30").unwrap();
        let mut iter = cidr.iter();
        assert_eq!(Ipv4Addr::new(192, 168, 122, 0), iter.next().unwrap());
        assert_eq!(Ipv4Addr::new(192, 168, 122, 1), iter.next().unwrap());
        assert_eq!(Ipv4Addr::new(192, 168, 122, 2), iter.next().unwrap());
        assert_eq!(Ipv4Addr::new(192, 168, 122, 3), iter.next().unwrap());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn iterator_v4_tiny() {
        let cidr = Ipv4Network::from_cidr("10/32").unwrap();
        let mut iter = cidr.iter();
        assert_eq!(Ipv4Addr::new(10, 0, 0, 0), iter.next().unwrap());
        assert_eq!(None, iter.next());
    }

    // Tests the entire IPv4 space to see if the iterator will stop at the correct place
    // and not overflow or wrap around. Ignored since it takes a long time to run.
    #[test]
    #[ignore]
    fn iterator_v4_huge() {
        let cidr = Ipv4Network::from_cidr("0/0").unwrap();
        let mut iter = cidr.iter();
        for i in 0..(u32::max_value() as u64 + 1) {
            assert_eq!(i as u32, u32::from(iter.next().unwrap()));
        }
        assert_eq!(None, iter.next());
    }
}