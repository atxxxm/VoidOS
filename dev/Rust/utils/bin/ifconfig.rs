use std::net::Ipv4Addr;
use clap::Parser;

#[derive(Parser)]
#[command(version, about = "Configure a network interface", long_about = None)]
struct Args {
    interface: String,

    /// Address in CIDR form, e.g. 10.0.2.15/24
    address: Option<String>,

    /// Bring the interface up (implied when an address is given without
    /// --down)
    #[arg(long)]
    up: bool,

    /// Bring the interface down
    #[arg(long)]
    down: bool,
}

fn main() {
    let args = Args::parse();
    if let Err(e) = run(&args) {
        eprintln!("ifconfig: {e}");
        std::process::exit(1);
    }
}

fn run(args: &Args) -> std::io::Result<()> {
    if args.address.is_none() && !args.up && !args.down {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "specify an address (CIDR) and/or --up/--down",
        ));
    }

    if let Some(address) = &args.address {
        let (ip, prefix) = parse_cidr(address).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("invalid address '{address}' (expected e.g. 10.0.2.15/24)"),
            )
        })?;
        net::set_address(&args.interface, ip)?;
        net::set_netmask(&args.interface, prefix_to_netmask(prefix))?;
    }

    if args.down {
        net::set_flag(&args.interface, false)?;
    } else if args.up || args.address.is_some() {
        // Assigning an address without an explicit --down still brings
        // the interface up, matching classic `ifconfig` behavior.
        net::set_flag(&args.interface, true)?;
    }

    Ok(())
}

fn parse_cidr(spec: &str) -> Option<(Ipv4Addr, u8)> {
    let (ip, prefix) = spec.split_once('/')?;
    let prefix: u8 = prefix.parse().ok()?;
    if prefix > 32 {
        return None;
    }
    Some((ip.parse().ok()?, prefix))
}

fn prefix_to_netmask(prefix: u8) -> Ipv4Addr {
    let bits: u32 = if prefix == 0 { 0 } else { u32::MAX << (32 - prefix as u32) };
    Ipv4Addr::from(bits)
}

// Classic ioctl-based interface configuration (SIOCSIFADDR/SIOCSIFNETMASK/
// SIOCSIFFLAGS) -- the same mechanism the original `ifconfig` used before
// iproute2/netlink became the modern way. `ifreq` isn't exposed by the
// `libc` crate for this target (it has it for Android/l4re but not plain
// glibc yet), so it's defined here by hand, matching <net/if.h> exactly;
// the ioctl numbers below are stable, decades-old Linux ABI constants.
#[cfg(unix)]
mod net {
    use std::{ffi::CString, mem::zeroed, net::Ipv4Addr};

    const IFNAMSIZ: usize = 16;
    const SIOCSIFADDR: libc::c_ulong = 0x8916;
    const SIOCSIFNETMASK: libc::c_ulong = 0x891C;
    const SIOCGIFFLAGS: libc::c_ulong = 0x8913;
    const SIOCSIFFLAGS: libc::c_ulong = 0x8914;

    #[repr(C)]
    #[derive(Clone, Copy)]
    union IfrIfru {
        addr: libc::sockaddr,
        flags: libc::c_short,
    }

    #[repr(C)]
    struct IfReq {
        name: [libc::c_char; IFNAMSIZ],
        ifru: IfrIfru,
    }

    fn open_dgram_socket() -> std::io::Result<libc::c_int> {
        let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
        if fd < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(fd)
        }
    }

    fn ifreq_with_name(name: &str) -> std::io::Result<IfReq> {
        let c_name = CString::new(name).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid interface name")
        })?;
        let bytes = c_name.as_bytes_with_nul();
        if bytes.len() > IFNAMSIZ {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "interface name too long",
            ));
        }
        let mut req: IfReq = unsafe { zeroed() };
        for (i, &b) in bytes.iter().enumerate() {
            req.name[i] = b as libc::c_char;
        }
        Ok(req)
    }

    fn sockaddr_in(ip: Ipv4Addr) -> libc::sockaddr {
        let mut addr: libc::sockaddr_in = unsafe { zeroed() };
        addr.sin_family = libc::AF_INET as libc::sa_family_t;
        // Ipv4Addr::octets() is already in network byte order; from_ne_bytes
        // reassembles them into s_addr without any endian-swapping, which
        // is exactly what we want here (s_addr is treated as an opaque
        // 4-byte blob, never re-interpreted as a host-order integer).
        addr.sin_addr.s_addr = u32::from_ne_bytes(ip.octets());
        unsafe { std::mem::transmute(addr) }
    }

    pub fn set_address(iface: &str, ip: Ipv4Addr) -> std::io::Result<()> {
        let fd = open_dgram_socket()?;
        let mut req = ifreq_with_name(iface)?;
        req.ifru.addr = sockaddr_in(ip);
        let ret = unsafe { libc::ioctl(fd, SIOCSIFADDR, &mut req as *mut IfReq as *mut libc::c_void) };
        unsafe { libc::close(fd) };
        if ret != 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }

    pub fn set_netmask(iface: &str, mask: Ipv4Addr) -> std::io::Result<()> {
        let fd = open_dgram_socket()?;
        let mut req = ifreq_with_name(iface)?;
        req.ifru.addr = sockaddr_in(mask);
        let ret = unsafe { libc::ioctl(fd, SIOCSIFNETMASK, &mut req as *mut IfReq as *mut libc::c_void) };
        unsafe { libc::close(fd) };
        if ret != 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }

    pub fn set_flag(iface: &str, up: bool) -> std::io::Result<()> {
        let fd = open_dgram_socket()?;
        let mut req = ifreq_with_name(iface)?;

        // Read the current flags first so this only toggles IFF_UP/
        // IFF_RUNNING rather than clobbering whatever else is set.
        let ret = unsafe { libc::ioctl(fd, SIOCGIFFLAGS, &mut req as *mut IfReq as *mut libc::c_void) };
        if ret != 0 {
            unsafe { libc::close(fd) };
            return Err(std::io::Error::last_os_error());
        }

        let current = unsafe { req.ifru.flags };
        req.ifru.flags = if up {
            current | (libc::IFF_UP as libc::c_short) | (libc::IFF_RUNNING as libc::c_short)
        } else {
            current & !(libc::IFF_UP as libc::c_short)
        };

        let ret = unsafe { libc::ioctl(fd, SIOCSIFFLAGS, &mut req as *mut IfReq as *mut libc::c_void) };
        unsafe { libc::close(fd) };
        if ret != 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
}

#[cfg(not(unix))]
mod net {
    use std::net::Ipv4Addr;

    fn unsupported() -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::Unsupported, "ifconfig is only supported on Unix")
    }

    pub fn set_address(_iface: &str, _ip: Ipv4Addr) -> std::io::Result<()> {
        Err(unsupported())
    }
    pub fn set_netmask(_iface: &str, _mask: Ipv4Addr) -> std::io::Result<()> {
        Err(unsupported())
    }
    pub fn set_flag(_iface: &str, _up: bool) -> std::io::Result<()> {
        Err(unsupported())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cidr_address() {
        let (ip, prefix) = parse_cidr("10.0.2.15/24").unwrap();
        assert_eq!(ip, Ipv4Addr::new(10, 0, 2, 15));
        assert_eq!(prefix, 24);
    }

    #[test]
    fn rejects_a_prefix_over_32() {
        assert!(parse_cidr("10.0.2.15/33").is_none());
    }

    #[test]
    fn converts_prefix_to_netmask() {
        assert_eq!(prefix_to_netmask(24), Ipv4Addr::new(255, 255, 255, 0));
        assert_eq!(prefix_to_netmask(16), Ipv4Addr::new(255, 255, 0, 0));
        assert_eq!(prefix_to_netmask(0), Ipv4Addr::new(0, 0, 0, 0));
    }
}
