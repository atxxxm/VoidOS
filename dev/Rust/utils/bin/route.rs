use std::net::Ipv4Addr;
use clap::Parser;

// Only the one thing we actually need: adding a default gateway. Real
// `route`/iproute2 covers a lot more (per-destination routes, netlink
// instead of ioctl, ...) -- out of scope here.
#[derive(Parser)]
#[command(version, about = "Add a default route (only \"route add default gw <ip>\" is supported)", long_about = None)]
struct Args {
    action: String,
    destination: String,
    #[arg(long = "gw")]
    gateway: String,
}

fn main() {
    let args = Args::parse();

    if args.action != "add" || args.destination != "default" {
        eprintln!("route: only \"route add default gw <ip>\" is supported");
        std::process::exit(1);
    }

    let Ok(gateway) = args.gateway.parse::<Ipv4Addr>() else {
        eprintln!("route: invalid gateway '{}'", args.gateway);
        std::process::exit(1);
    };

    if let Err(e) = add_default_route(gateway) {
        eprintln!("route: {e}");
        std::process::exit(1);
    }
}

#[cfg(unix)]
fn add_default_route(gateway: Ipv4Addr) -> std::io::Result<()> {
    use std::mem::zeroed;

    const SIOCADDRT: libc::c_ulong = 0x890B;
    const RTF_UP: libc::c_ushort = 0x0001;
    const RTF_GATEWAY: libc::c_ushort = 0x0002;

    fn sockaddr_in(ip: Ipv4Addr) -> libc::sockaddr {
        let mut addr: libc::sockaddr_in = unsafe { zeroed() };
        addr.sin_family = libc::AF_INET as libc::sa_family_t;
        addr.sin_addr.s_addr = u32::from_ne_bytes(ip.octets());
        unsafe { std::mem::transmute(addr) }
    }

    let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }

    let mut rt: libc::rtentry = unsafe { zeroed() };
    rt.rt_dst = sockaddr_in(Ipv4Addr::UNSPECIFIED);
    rt.rt_genmask = sockaddr_in(Ipv4Addr::UNSPECIFIED);
    rt.rt_gateway = sockaddr_in(gateway);
    rt.rt_flags = RTF_UP | RTF_GATEWAY;

    let ret = unsafe { libc::ioctl(fd, SIOCADDRT, &mut rt as *mut libc::rtentry as *mut libc::c_void) };
    unsafe { libc::close(fd) };

    if ret != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(unix))]
fn add_default_route(_gateway: Ipv4Addr) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "route is only supported on Unix",
    ))
}
