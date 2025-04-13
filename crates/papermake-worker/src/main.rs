use std::time::Duration;

#[tokio::main]
async fn main() {
    println!("papermake-worker starting up");
    println!("Using papermake version: {}", papermake::version());
    
    // Just keep the worker running to verify it works
    loop {
        println!("Worker running... (waiting for implementation)");
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}