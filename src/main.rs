use common_http_server_rs::quick_start;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running minimal server on port 3000.");
    println!("For advanced setups, use examples under examples/ (see doc/SAMPLES.md).");
    quick_start(3000).await
}
