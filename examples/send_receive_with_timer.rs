extern crate udp_polygon;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::{thread, time};
use udp_polygon::timers::Timers;
use udp_polygon::{config::Address, config::Config, config::FromArguments, Polygon};

#[derive(Debug, Deserialize, Serialize)]
pub struct Message {
    pub id: u32,
    pub msg: String,
}

#[tokio::main]
async fn main() {
    let config = Config::from_arguments(
        vec![Address {
            ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port: 5060,
        }],
        Some(Address {
            ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port: 5060,
        }),
    );

    let mut polygon = Polygon::configure(config).expect("failed to configure polygon");

    let rx = polygon.receive();
    let pause = Arc::clone(&polygon.pause_timer_send);

    tokio::spawn(async move {
        let mut counter = 0;
        loop {
            let msg = rx.recv().unwrap();
            println!("Received: {:?}", msg);
            counter += 1;
            if counter == 2 {
                *pause.lock().unwrap() = true;
            }
        }
    });

    println!("Sending message...");
    polygon.send_with_timer(
        "Hello World".as_bytes().to_vec(),
        Timers {
            delays: vec![500, 600, 1000, 1500],
        },
    );

    loop {
        thread::sleep(time::Duration::from_millis(1000));
    }
}
