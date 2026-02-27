extern crate udp_polygon;
use udp_polygon::{config::Config, config::FromToml, Polygon};

#[tokio::main]
async fn main() {
    let config = Config::from_toml("config_receive.toml".to_string());
    let mut polygon = Polygon::configure(config).expect("failed to configure polygon");

    let rx = polygon.receive();

    loop {
        let maybe = rx.try_recv();
        if let Ok(data) = maybe {
            println!("receiving... {data:?}");
        }
    }
}
