//! Narrow CONNECT gateway for the macOS sandbox, which cannot match DNS names.
//! The runtime can reach only this listener. Destination and TLS SNI must both
//! match an allowlisted provider host; arbitrary egress never leaves the app.
use std::{
    io::{Read, Write},
    net::{IpAddr, Shutdown, TcpListener, TcpStream, ToSocketAddrs},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
const HOSTS: [&str; 3] = ["auth.openai.com", "chatgpt.com", "api.openai.com"];
const CONNECTION_LIMIT: usize = 8;
pub(crate) struct Gateway {
    pub port: u16,
    stop: Arc<AtomicBool>,
    streams: Arc<Mutex<Vec<(uuid::Uuid, TcpStream)>>>,
}
impl Drop for Gateway {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Ok(streams) = self.streams.lock() {
            for (_, stream) in &*streams {
                let _ = stream.shutdown(Shutdown::Both);
            }
        }
    }
}
impl Gateway {
    pub fn start() -> Result<Self, &'static str> {
        let listener =
            TcpListener::bind(("127.0.0.1", 0)).map_err(|_| "PLAN_RUNTIME_UNAVAILABLE")?;
        listener
            .set_nonblocking(true)
            .map_err(|_| "PLAN_RUNTIME_UNAVAILABLE")?;
        let port = listener
            .local_addr()
            .map_err(|_| "PLAN_RUNTIME_UNAVAILABLE")?
            .port();
        let stop = Arc::new(AtomicBool::new(false));
        let streams = Arc::new(Mutex::new(Vec::<(uuid::Uuid, TcpStream)>::new()));
        let active = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (thread_stop, thread_streams) = (stop.clone(), streams.clone());
        std::thread::spawn(move || {
            while !thread_stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((client, peer)) if peer.ip().is_loopback() => {
                        if active.fetch_add(1, Ordering::AcqRel) >= CONNECTION_LIMIT {
                            active.fetch_sub(1, Ordering::AcqRel);
                            continue;
                        }
                        let (active, stop, streams) =
                            (active.clone(), thread_stop.clone(), thread_streams.clone());
                        std::thread::spawn(move || {
                            let _ = forward(client, &stop, &streams);
                            active.fetch_sub(1, Ordering::AcqRel);
                        });
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(25));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            port,
            stop,
            streams,
        })
    }
}
fn connect_host(header: &[u8]) -> Option<&str> {
    let header = std::str::from_utf8(header).ok()?;
    let first = header.lines().next()?;
    let parts = first.split(' ').collect::<Vec<_>>();
    if parts.len() != 3 || parts[0] != "CONNECT" || !matches!(parts[2], "HTTP/1.1" | "HTTP/1.0") {
        return None;
    }
    let host = parts[1].strip_suffix(":443")?;
    HOSTS.contains(&host).then_some(host)
}
fn read_header(client: &mut TcpStream) -> Result<Vec<u8>, ()> {
    let mut header = Vec::new();
    let end = Instant::now() + Duration::from_secs(5);
    while header.len() < 8192 && Instant::now() < end {
        let mut byte = [0];
        client.read_exact(&mut byte).map_err(|_| ())?;
        header.push(byte[0]);
        if header.ends_with(b"\r\n\r\n") {
            return Ok(header);
        }
    }
    Err(())
}
fn read_client_hello(client: &mut TcpStream) -> Result<(Vec<u8>, String), ()> {
    let mut wire = Vec::new();
    let mut handshake = Vec::new();
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        if Instant::now() > end || wire.len() > 65536 {
            return Err(());
        }
        let mut header = [0_u8; 5];
        client.read_exact(&mut header).map_err(|_| ())?;
        if header[0] != 22 || header[1] != 3 {
            return Err(());
        }
        let size = usize::from(u16::from_be_bytes([header[3], header[4]]));
        if size == 0 || wire.len() + size + 5 > 65536 {
            return Err(());
        }
        let mut record = vec![0; size];
        client.read_exact(&mut record).map_err(|_| ())?;
        wire.extend_from_slice(&header);
        wire.extend_from_slice(&record);
        handshake.extend_from_slice(&record);
        if handshake.len() >= 4 {
            let length = (usize::from(handshake[1]) << 16)
                | (usize::from(handshake[2]) << 8)
                | usize::from(handshake[3]);
            if handshake[0] != 1 || length > 65532 {
                return Err(());
            }
            if handshake.len() >= length + 4 {
                return Ok((
                    wire,
                    server_name(&handshake[..length + 4]).ok_or(())?.to_owned(),
                ));
            }
        }
    }
}
fn take<'a>(bytes: &'a [u8], offset: &mut usize, size: usize) -> Option<&'a [u8]> {
    let end = offset.checked_add(size)?;
    let slice = bytes.get(*offset..end)?;
    *offset = end;
    Some(slice)
}
fn word(bytes: &[u8], offset: &mut usize) -> Option<usize> {
    let data = take(bytes, offset, 2)?;
    Some(usize::from(u16::from_be_bytes([data[0], data[1]])))
}
fn server_name(bytes: &[u8]) -> Option<&str> {
    let mut offset = 4;
    take(bytes, &mut offset, 34)?;
    let session = usize::from(*take(bytes, &mut offset, 1)?.first()?);
    take(bytes, &mut offset, session)?;
    let ciphers = word(bytes, &mut offset)?;
    take(bytes, &mut offset, ciphers)?;
    let compression = usize::from(*take(bytes, &mut offset, 1)?.first()?);
    take(bytes, &mut offset, compression)?;
    let total = word(bytes, &mut offset)?;
    let end = offset.checked_add(total)?;
    if end != bytes.len() {
        return None;
    }
    let mut name = None;
    while offset < end {
        let kind = word(bytes, &mut offset)?;
        let length = word(bytes, &mut offset)?;
        let extension = take(bytes, &mut offset, length)?;
        if kind == 0 {
            if name.is_some() {
                return None;
            }
            let mut index = 0;
            let length = word(extension, &mut index)?;
            if length + 2 != extension.len() {
                return None;
            }
            if take(extension, &mut index, 1)? != [0] {
                return None;
            }
            let length = word(extension, &mut index)?;
            let host = std::str::from_utf8(take(extension, &mut index, length)?).ok()?;
            if index != extension.len() || !HOSTS.contains(&host) {
                return None;
            }
            name = Some(host);
        }
    }
    name
}
fn public_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            !ip.is_private()
                && !ip.is_loopback()
                && !ip.is_link_local()
                && !ip.is_broadcast()
                && !ip.is_documentation()
                && !ip.is_unspecified()
                && !ip.is_multicast()
                && ip.octets()[0] != 0
                && ip.octets()[0] < 224
                && !(ip.octets()[0] == 100 && (64..=127).contains(&ip.octets()[1]))
        }
        IpAddr::V6(ip) => {
            ip.to_ipv4_mapped().is_none()
                && !ip.is_loopback()
                && !ip.is_unspecified()
                && !ip.is_unique_local()
                && !ip.is_unicast_link_local()
                && !ip.is_multicast()
        }
    }
}
fn forward(
    mut client: TcpStream,
    stop: &Arc<AtomicBool>,
    streams: &Arc<Mutex<Vec<(uuid::Uuid, TcpStream)>>>,
) -> Result<(), ()> {
    // On macOS, accept() can inherit the listener's nonblocking mode. This
    // worker uses blocking reads with deadlines, including the TLS flight that
    // arrives only after CONNECT succeeds. EAGAIN here used to close the tunnel.
    client.set_nonblocking(false).map_err(|_| ())?;
    client
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|_| ())?;
    client
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|_| ())?;
    let header = read_header(&mut client)?;
    let host = connect_host(&header).ok_or(())?.to_owned();
    client
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .map_err(|_| ())?;
    let (hello, sni) = read_client_hello(&mut client)?;
    if sni != host || stop.load(Ordering::Acquire) {
        return Err(());
    }
    // Only constant hostnames reach resolution. DNS failures never widen egress.
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    let resolve_host = host.clone();
    std::thread::spawn(move || {
        let _ = send.send(
            (resolve_host.as_str(), 443)
                .to_socket_addrs()
                .map(|addresses| {
                    addresses
                        .filter(|a| public_address(a.ip()))
                        .take(4)
                        .collect::<Vec<_>>()
                }),
        );
    });
    let addresses = receive
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| ())?
        .map_err(|_| ())?;
    let mut server = addresses
        .into_iter()
        .find_map(|addr| TcpStream::connect_timeout(&addr, Duration::from_secs(2)).ok())
        .ok_or(())?;
    server
        .set_read_timeout(Some(Duration::from_secs(120)))
        .map_err(|_| ())?;
    server
        .set_write_timeout(Some(Duration::from_secs(120)))
        .map_err(|_| ())?;
    client
        .set_read_timeout(Some(Duration::from_secs(120)))
        .map_err(|_| ())?;
    client
        .set_write_timeout(Some(Duration::from_secs(120)))
        .map_err(|_| ())?;
    let id = uuid::Uuid::now_v7();
    {
        let mut all = streams.lock().map_err(|_| ())?;
        if all.len() > 64 {
            return Err(());
        }
        all.push((id, client.try_clone().map_err(|_| ())?));
        all.push((id, server.try_clone().map_err(|_| ())?));
    }
    let _lease = StreamLease { pool: streams, id };
    server.write_all(&hello).map_err(|_| ())?;
    let mut upstream_client = client.try_clone().map_err(|_| ())?;
    let mut upstream_server = server.try_clone().map_err(|_| ())?;
    let sender = std::thread::spawn(move || {
        let _ = std::io::copy(&mut upstream_client, &mut upstream_server);
        let _ = upstream_server.shutdown(Shutdown::Write);
    });
    let _ = std::io::copy(&mut server, &mut client);
    let _ = client.shutdown(Shutdown::Both);
    let _ = server.shutdown(Shutdown::Both);
    let _ = sender.join();
    Ok(())
}
struct StreamLease<'a> {
    pool: &'a Mutex<Vec<(uuid::Uuid, TcpStream)>>,
    id: uuid::Uuid,
}
impl Drop for StreamLease<'_> {
    fn drop(&mut self) {
        if let Ok(mut streams) = self.pool.lock() {
            streams.retain(|(id, _)| *id != self.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inherited_nonblocking_socket_waits_for_tls_and_rejects_invalid_data() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let (accepted, _) = listener.accept().unwrap();
        // Reproduce the macOS inheritance on every supported test platform.
        accepted.set_nonblocking(true).unwrap();
        let (finished, result) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            finished
                .send(forward(
                    accepted,
                    &Arc::new(AtomicBool::new(false)),
                    &Arc::new(Mutex::new(Vec::new())),
                ))
                .unwrap();
        });
        client
            .write_all(b"CONNECT auth.openai.com:443 HTTP/1.1\r\nHost: auth.openai.com:443\r\n\r\n")
            .unwrap();
        let established = b"HTTP/1.1 200 Connection Established\r\n\r\n";
        let mut received = vec![0; established.len()];
        client.read_exact(&mut received).unwrap();
        assert_eq!(received, established);
        assert!(matches!(
            result.recv_timeout(Duration::from_millis(50)),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        ));
        // No upstream DNS or connection occurs for this invalid TLS record.
        client.write_all(&[0; 5]).unwrap();
        assert!(
            result
                .recv_timeout(Duration::from_secs(1))
                .unwrap()
                .is_err()
        );
        worker.join().unwrap();
    }
    #[tokio::test]
    #[ignore = "explicit public authentication endpoint TLS check; no credentials or login"]
    async fn authentication_tls_through_gateway_qualification() {
        let gateway = Gateway::start().unwrap();
        let response = reqwest::Client::builder()
            .https_only(true)
            .proxy(reqwest::Proxy::https(format!("http://127.0.0.1:{}", gateway.port)).unwrap())
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(20))
            .build()
            .unwrap()
            .get("https://auth.openai.com/oauth/token")
            .send()
            .await
            .expect("certificate-validated TLS through the allowlisted gateway");
        assert!(response.status().is_success() || response.status().is_client_error());
    }
    #[test]
    fn connect_only_allows_provider_tls() {
        assert_eq!(
            connect_host(b"CONNECT chatgpt.com:443 HTTP/1.1\r\n\r\n"),
            Some("chatgpt.com")
        );
        for header in [
            "CONNECT evil.example:443 HTTP/1.1",
            "CONNECT auth.openai.com:80 HTTP/1.1",
            "CONNECT chatgpt.com.evil:443 HTTP/1.1",
            "GET https://chatgpt.com HTTP/1.1",
            "CONNECT user@chatgpt.com:443 HTTP/1.1",
            "CONNECT 127.0.0.1:443 HTTP/1.1",
        ] {
            assert!(connect_host(header.as_bytes()).is_none());
        }
        for ip in [
            "127.0.0.1",
            "10.1.2.3",
            "192.168.1.1",
            "169.254.1.1",
            "100.64.0.1",
            "::1",
            "::ffff:8.8.8.8",
        ] {
            assert!(!public_address(ip.parse().unwrap()));
        }
    }
    #[test]
    fn valid_client_hello_preserves_the_exact_allowlisted_host() {
        fn hello(host: &str) -> Vec<u8> {
            let mut name = vec![0];
            name.extend_from_slice(&u16::try_from(host.len()).unwrap().to_be_bytes());
            name.extend_from_slice(host.as_bytes());
            let mut extension = u16::try_from(name.len()).unwrap().to_be_bytes().to_vec();
            extension.extend(name);
            let mut extensions = vec![0, 0];
            extensions.extend_from_slice(&u16::try_from(extension.len()).unwrap().to_be_bytes());
            extensions.extend(extension);
            let mut body = vec![3, 3];
            body.extend([0; 32]);
            body.extend([0, 0, 2, 0x13, 1, 1, 0]);
            body.extend_from_slice(&u16::try_from(extensions.len()).unwrap().to_be_bytes());
            body.extend(extensions);
            let length = u16::try_from(body.len()).unwrap().to_be_bytes();
            let mut handshake = vec![1, 0, length[0], length[1]];
            handshake.extend(body);
            handshake
        }
        for host in HOSTS {
            assert_eq!(server_name(&hello(host)), Some(host));
        }
        assert!(server_name(&hello("chatgpt.com.evil.example")).is_none());
        assert!(server_name(&hello("127.0.0.1")).is_none());
        let mut malformed = hello("chatgpt.com");
        malformed.push(0);
        assert!(server_name(&malformed).is_none());
    }
    #[test]
    fn malformed_client_hellos_never_pass() {
        for bytes in [vec![], vec![0; 65536], vec![1, 0, 0, 1, 0]] {
            assert!(server_name(&bytes).is_none());
        }
    }
}
