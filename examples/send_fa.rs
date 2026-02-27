extern crate udp_polygon;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr};
use std::{thread, time};
use udp_polygon::{config::Address, config::Config, config::FromArguments, Polygon};

#[derive(Debug, Deserialize, Serialize)]
pub struct Message {
    pub id: u32,
    pub msg: String,
}

fn main() {
    let config = Config::from_arguments(
        vec![Address {
            ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port: 5061,
        }],
        Some(Address {
            ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port: 5060,
        }),
    );

    let mut polygon = Polygon::configure(config).expect("failed to configure polygon");

    loop {
        println!("sending message...");
        polygon.send(
            serde_json::to_string(&Message {
                id: 1,
                msg: String::from("Hello"),
            })
            .unwrap()
            .into(),
        ).expect("send failed");
        thread::sleep(time::Duration::from_secs(2));
    }
}
