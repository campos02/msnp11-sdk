use crate::mock_http::MockHttp;
use crate::mock_ns::MockNS;
use crate::mock_sb::MockSB;
use env_logger::Env;
use log::info;

mod mock_http;
mod mock_ns;
mod mock_sb;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("trace")).init();
    info!("Starting Mock Server");
    tokio::join!(
        MockHttp::mock_passport(),
        MockNS::listen(),
        MockSB::listen()
    );
}
