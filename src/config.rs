use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::net::{IpAddr, Ipv4Addr};

/// Config holds the configuration for local and remote UDP addresses
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub bind_addresses: Vec<Address>,
    pub destination_address: Option<Address>,
}

/// Address is a struct that holds an IP address and a port
/// It is used to configure the local and remote addresses for UDP
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Address {
    pub ip: IpAddr,
    pub port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            bind_addresses: vec![Address {
                ip: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                port: 5060,
            }],
            destination_address: Some(Address {
                ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                port: 5061,
            }),
        }
    }
}

/// Configuration through TOML
pub trait FromToml {
    fn from_toml(filename: String) -> Self;
}

/// Configuration through arguments
pub trait FromArguments {
    fn from_arguments(local: Vec<Address>, remote: Option<Address>) -> Self;
}

/// Default configuration
pub trait FromDefault {
    fn from_default() -> Self;
}

/// Configuration through environment variables
pub trait FromEnv {
    fn from_env() -> Self;
}

/// ## Info
/// FromToml is used when the user wants to specify
/// the addresses and ports via a config file
///
/// ## Example
/// ``` Toml
/// [[bind_addresses]]
/// ip = "127.0.0.1"
/// port = 5061
/// [destination_address]
/// ip = "127.0.0.1"
/// port = 5060
/// ```
/// ``` rust,no_run
/// use udp_polygon::config::{Config, FromToml};
/// let config = Config::from_toml("config.toml".to_string());
/// ```
impl FromToml for Config {
    fn from_toml(filename: String) -> Self {
        let contents = fs::read_to_string(filename);

        match contents {
            Ok(cf) => toml::from_str(&cf).unwrap(),
            Err(_) => Config::default(),
        }
    }
}

/// ## Info
/// FromArguments is used for when the user prefers to
/// pass the addresses and ports via their own means
///
/// ## Example
/// ``` rust
/// use udp_polygon::config::{Address, Config, FromArguments};
/// use std::net::{IpAddr, Ipv4Addr};
/// let config = Config::from_arguments(
///     vec![Address { ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port: 5061 }],
///     Some(Address { ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port: 5060 }),
/// );
/// ```
impl FromArguments for Config {
    fn from_arguments(local: Vec<Address>, remote: Option<Address>) -> Self {
        Config {
            bind_addresses: local,
            destination_address: remote,
        }
    }
}

/// FromDefault is used when no config file is specified
///
///for the time I using 5060/5061 as the default ports
impl FromDefault for Config {
    fn from_default() -> Self {
        Config::default()
    }
}

/// ## Info
/// FromEnv only supports a single address/port pair
/// for both the local and remote addresses
///
/// THE ENVIRONMENT VARIABLES ARE:
/// * BIND_ADDRS: the local address to bind to
/// * BIND_PORT: the local port to bind to
/// * DEST_ADDRS: the remote address to send to
/// * DEST_PORT: the remote port to send to
///
/// ## Example
///``` rust,no_run
/// use udp_polygon::config::{Config, FromEnv};
/// let config = Config::from_env();
/// ```
impl FromEnv for Config {
    fn from_env() -> Self {
        let bind_address = match env::var("BIND_ADDRS") {
            Ok(addr) => addr
                .parse::<IpAddr>()
                .unwrap_or_else(|_| panic!("Invalid address")),
            Err(err) => panic!("{}", err),
        };

        let bind_port = match env::var("BIND_PORT") {
            Ok(port) => port
                .parse::<u16>()
                .unwrap_or_else(|_| panic!("Invalid port")),
            Err(err) => panic!("{}", err),
        };

        let dest_address = match env::var("DEST_ADDRS") {
            Ok(addr) => Some(
                addr.parse::<IpAddr>()
                    .unwrap_or_else(|_| panic!("Invalid address")),
            ),
            Err(_err) => None,
        };

        let dest_port = match env::var("DEST_PORT") {
            Ok(port) => Some(
                port.parse::<u16>()
                    .unwrap_or_else(|_| panic!("Invalid port")),
            ),
            Err(_err) => None,
        };

        Config {
            bind_addresses: vec![Address {
                ip: bind_address,
                port: bind_port,
            }],
            destination_address: match (dest_address, dest_port) {
                (Some(ip), Some(port)) => Some(Address { ip, port }),
                _ => None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FromEnv;
    use serial_test::serial;
    use std::env;

    #[test]
    fn default_config() {
        let config = super::Config::default();
        assert_eq!(config.bind_addresses.len(), 1);
        assert_eq!(config.destination_address.is_some(), true);
    }

    #[test]
    #[serial]
    fn env_config_v4() {
        env::set_var("BIND_ADDRS", "192.168.1.0");
        env::set_var("BIND_PORT", "5060");
        env::set_var("DEST_ADDRS", "192.168.1.0");
        env::set_var("DEST_PORT", "5061");

        let config = super::Config::from_env();
        assert_eq!(config.bind_addresses.len(), 1);
        assert_eq!(config.destination_address.is_some(), true);
    }

    #[test]
    #[serial]
    #[should_panic]
    fn bind_address_env_not_there() {
        env::remove_var("BIND_ADDRS");
        env::remove_var("BIND_PORT");
        super::Config::from_env();
    }

    #[test]
    #[serial]
    fn dest_address_env_not_there() {
        env::set_var("BIND_ADDRS", "192.168.1.0");
        env::set_var("BIND_PORT", "5060");
        env::remove_var("DEST_ADDRS");
        env::remove_var("DEST_PORT");
        let config = super::Config::from_env();
        assert_eq!(config.bind_addresses.len(), 1);
        assert_eq!(config.destination_address.is_none(), true);
    }
}
