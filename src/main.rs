use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use tiny_http::server::Server;
use tiny_http::Result;

#[tokio::main]
async fn main() -> Result<()>{
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");


    let server = Server::new("127.0.0.1:8080");
    server.run().await
}
