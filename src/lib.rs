//! # udp-polygon
//!
//! `udp-polygon` is a library that allows to send and receive UDP packets.
//!
//! It can be configured in many ways, using toml, args, or env vars.
//!
//! It also supports retransmission of packets, using timers.
//!
//! ## Requirements
//! * the consumer requires  [tokio](https://docs.rs/tokio/)
//! * a producer does not require anything extra
//! * a producer with the timer flag enabled requires [tokio](https://docs.rs/tokio/)

use std::net::{SocketAddr, UdpSocket};
/// This is the configuration module. It allows to configure the lib, using toml, args, or env vars.
pub mod config;
use config::Config;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

#[cfg(feature = "timers")]
pub mod timers;

#[cfg(feature = "timers")]
use crate::timers::Timers;

#[cfg(feature = "timers")]
use tokio::time::Duration;

/// Polygon is a UDP socket that can send and receive data.
/// It can be configured by using the `configure` method.
/// ``` rust,no_run
/// # use udp_polygon::Polygon;
/// # use udp_polygon::config::{Config, FromDefault};
/// # let config = Config::from_default();
/// let mut polygon = Polygon::configure(config).expect("failed to configure");
/// ```
#[derive(Debug)]
pub struct Polygon {
    pub socket: UdpSocket,
    pub destination: Option<SocketAddr>,
    pub pause_timer_send: Arc<Mutex<bool>>,
}

impl Polygon {
    pub fn get_channel() -> (Sender<Vec<u8>>, Receiver<Vec<u8>>) {
        let (tx, rx) = mpsc::channel();
        (tx, rx)
    }
    pub fn configure(config: Config) -> std::io::Result<Self> {
        let addrs = config
            .bind_addresses
            .into_iter()
            .map(|addr| SocketAddr::new(addr.ip, addr.port))
            .collect::<Vec<_>>();

        let socket = UdpSocket::bind(&addrs[..])?;

        Ok(Self {
            socket,
            destination: config
                .destination_address
                .map(|addr| SocketAddr::new(addr.ip, addr.port)),
            pause_timer_send: Arc::new(Mutex::new(false)),
        })
    }
    #[must_use]
    pub fn receive(&mut self) -> Receiver<Vec<u8>> {
        let std_socket = self.socket.try_clone().expect("failed to clone socket");
        std_socket
            .set_nonblocking(true)
            .expect("failed to set non-blocking mode");
        let socket =
            tokio::net::UdpSocket::from_std(std_socket).expect("failed to create async socket");
        let (tx, rx) = Self::get_channel();

        tokio::spawn(async move {
            let mut buffer = [0u8; 65535];
            loop {
                match socket.recv(&mut buffer).await {
                    Ok(amt) => {
                        let data = buffer[..amt].to_vec();
                        if tx.send(data).is_err() {
                            break; // receiver was dropped, stop the task
                        }
                    }
                    Err(e) => {
                        eprintln!("udp_polygon receive error: {e}");
                    }
                }
            }
        });

        rx
    }

    #[cfg(feature = "timers")]
    pub fn resume_timer_send(&mut self) {
        *self.pause_timer_send.lock().unwrap() = false;
    }
    #[cfg(feature = "timers")]
    pub fn pause_timer_send(&mut self) {
        *self.pause_timer_send.lock().unwrap() = true;
    }
    #[cfg(feature = "timers")]
    #[deprecated(since = "0.2.3", note = "use `pause_timer_send` instead")]
    pub fn cancel_timer_receive(&mut self) {
        self.pause_timer_send();
    }
    #[cfg(feature = "timers")]
    pub fn send_with_timer(&mut self, data: Vec<u8>, timers: Timers) {
        let socket = self.socket.try_clone().expect("failed to clone socket");
        let destination = self.destination.expect("no destination address configured");
        let pause = Arc::clone(&self.pause_timer_send);
        tokio::spawn(async move {
            let mut current_timer = timers.delays.into_iter();
            let mut counter = 0;
            loop {
                if *pause.lock().unwrap() && counter > 0 {
                    break;
                }
                let next_timer = match current_timer.next() {
                    Some(timer) => timer,
                    None => {
                        break;
                    }
                };

                if let Err(e) = socket.send_to(&data, destination) {
                    eprintln!("udp_polygon send_with_timer error: {e}");
                    break;
                }
                tokio::time::sleep(Duration::from_millis(next_timer)).await;
                counter += 1;
            }
        });
    }
    pub fn send(&mut self, data: Vec<u8>) -> std::io::Result<usize> {
        let destination = self.destination.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "no destination address configured",
            )
        })?;
        self.socket.send_to(&data, destination)
    }
    pub fn change_destination(&mut self, new_destination: SocketAddr) {
        self.destination = Some(new_destination);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Address, Config, FromArguments};
    use std::net::{IpAddr, Ipv4Addr};

    fn loopback(port: u16) -> Address {
        Address {
            ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port,
        }
    }

    #[test]
    fn configure_binds_successfully() {
        let config = Config::from_arguments(vec![loopback(0)], None);
        let polygon = Polygon::configure(config);
        assert!(polygon.is_ok());
    }

    #[test]
    fn configure_fails_on_bad_address() {
        // Port 1 requires root; binding 0.0.0.0:1 should fail in unprivileged contexts.
        // We test a duplicate bind instead, which is reliably rejected.
        let config = Config::from_arguments(vec![loopback(0)], None);
        let p1 = Polygon::configure(config).unwrap();
        let bound_port = p1.socket.local_addr().unwrap().port();
        let config2 = Config::from_arguments(vec![loopback(bound_port)], None);
        // Second bind to the same port should fail.
        assert!(Polygon::configure(config2).is_err());
    }

    #[test]
    fn send_without_destination_returns_error() {
        let config = Config::from_arguments(vec![loopback(0)], None);
        let mut polygon = Polygon::configure(config).unwrap();
        let result = polygon.send(b"hello".to_vec());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotConnected);
    }

    #[test]
    fn change_destination_updates_address() {
        let config = Config::from_arguments(vec![loopback(0)], None);
        let mut polygon = Polygon::configure(config).unwrap();
        assert!(polygon.destination.is_none());
        let new_dest = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9999);
        polygon.change_destination(new_dest);
        assert_eq!(polygon.destination, Some(new_dest));
    }

    #[tokio::test]
    async fn send_and_receive_loopback() {
        // Bind receiver on a random port.
        let recv_config = Config::from_arguments(vec![loopback(0)], None);
        let mut receiver = Polygon::configure(recv_config).unwrap();
        let recv_port = receiver.socket.local_addr().unwrap().port();

        // Bind sender pointed at receiver.
        let send_config =
            Config::from_arguments(vec![loopback(0)], Some(loopback(recv_port)));
        let mut sender = Polygon::configure(send_config).unwrap();

        let rx = receiver.receive();

        let msg = b"hello udp_polygon".to_vec();
        sender.send(msg.clone()).unwrap();

        // Give the async task a moment to receive and forward.
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let received = rx.try_recv().expect("no message received");
        assert_eq!(received, msg);
    }
}

