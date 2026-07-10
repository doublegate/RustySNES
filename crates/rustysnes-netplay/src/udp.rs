//! The native UDP [`Transport`] — a real, tested `std::net::UdpSocket` connected to exactly one
//! remote peer (2-player point-to-point, matching [`crate::session::MAX_PLAYERS`]).
//!
//! Connection establishment (learning the peer's `SocketAddr`) is the caller's concern — this
//! type takes an already-known address, not a matchmaking/signaling layer of its own (out of
//! this ticket's scope; see the module doc on `crate` for what's ported vs. not).

use std::net::{SocketAddr, UdpSocket};

use crate::message::NetMessage;
use crate::transport::Transport;

/// A point-to-point UDP transport. Non-blocking: [`Transport::poll`] drains every datagram
/// currently available and returns immediately rather than blocking the caller's frame loop.
pub struct UdpTransport {
    socket: UdpSocket,
    peer: SocketAddr,
    /// Scratch receive buffer — a [`NetMessage`] is small (the largest variant, `Sync`, is
    /// magic + version + a 32-byte hash = 39 bytes plus the tag byte); comfortably under any
    /// realistic MTU, so a single `recv_from` always holds a whole datagram.
    recv_buf: [u8; 512],
}

impl UdpTransport {
    /// Bind a local UDP socket at `local_addr` and connect it to `peer` (in the `UdpSocket`
    /// sense: `send`/`recv` — not `send_to`/`recv_from` — implicitly target/accept only this
    /// address, the OS-level "connected UDP socket" pattern). Non-blocking, so [`Transport::poll`]
    /// never stalls a frame loop waiting on a packet that hasn't arrived yet.
    ///
    /// # Errors
    /// Returns the underlying `std::io::Error` if the socket can't be bound, connected, or set
    /// non-blocking.
    pub fn connect(local_addr: SocketAddr, peer: SocketAddr) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(local_addr)?;
        socket.connect(peer)?;
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            peer,
            recv_buf: [0; 512],
        })
    }

    /// The peer address this transport is connected to.
    #[must_use]
    pub const fn peer(&self) -> SocketAddr {
        self.peer
    }
}

impl Transport for UdpTransport {
    fn send(&mut self, msg: &NetMessage) {
        // Best-effort by design (UDP; the session's own ack/resend logic is the reliability
        // layer) — a send error here (e.g. a transient ENOBUFS) is not a protocol violation,
        // just a dropped packet like any other.
        let _ = self.socket.send(&msg.encode());
    }

    fn poll(&mut self) -> Vec<NetMessage> {
        let mut received = Vec::new();
        loop {
            match self.socket.recv(&mut self.recv_buf) {
                Ok(n) => {
                    if let Ok(msg) = NetMessage::decode(&self.recv_buf[..n]) {
                        received.push(msg);
                    }
                    // A malformed/foreign datagram is silently dropped (untrusted network input,
                    // `master-core` module 60) rather than treated as a protocol error — it may
                    // simply not be from this peer at all.
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break, // Any other transient OS error: stop this poll, try again next call.
            }
        }
        received
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_round_trip() {
        let a_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let b_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        // Bind both ends first to learn their OS-assigned ports, then connect each to the
        // other's real address.
        let a_socket = UdpSocket::bind(a_addr).unwrap();
        let b_socket = UdpSocket::bind(b_addr).unwrap();
        let a_real = a_socket.local_addr().unwrap();
        let b_real = b_socket.local_addr().unwrap();
        drop(a_socket);
        drop(b_socket);

        let mut a = UdpTransport::connect(a_real, b_real).unwrap();
        let mut b = UdpTransport::connect(b_real, a_real).unwrap();

        a.send(&NetMessage::Input {
            player: 0,
            frame: 7,
            input: 0x1234,
        });

        // A real OS-level round trip over loopback; poll a few times to absorb any scheduling
        // delay rather than assuming the datagram is instantly visible.
        let mut received = Vec::new();
        for _ in 0..50 {
            received.extend(b.poll());
            if !received.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(
            received,
            vec![NetMessage::Input {
                player: 0,
                frame: 7,
                input: 0x1234,
            }]
        );
    }

    #[test]
    fn poll_with_nothing_sent_returns_empty() {
        let a_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let b_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let a_socket = UdpSocket::bind(a_addr).unwrap();
        let b_socket = UdpSocket::bind(b_addr).unwrap();
        let a_real = a_socket.local_addr().unwrap();
        let b_real = b_socket.local_addr().unwrap();
        drop(a_socket);
        drop(b_socket);
        let mut a = UdpTransport::connect(a_real, b_real).unwrap();
        assert!(a.poll().is_empty());
    }
}
