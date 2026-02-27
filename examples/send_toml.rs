extern crate udp_polygon;
use serde::{Deserialize, Serialize};
use std::{thread, time};
use udp_polygon::{config::Config, config::FromToml, Polygon};

#[derive(Debug, Deserialize, Serialize)]
pub struct Message {
    pub id: u32,
    pub msg: String,
}

fn main() {
    let config = Config::from_toml("config_send.toml".to_string());

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
