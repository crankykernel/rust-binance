use binance::common::websocket::{stream_name_aggtrade, stream_name_kline};

#[tokio::main]
pub async fn main() {
    println!("Futures WebSocket Example.");
    let kline_stream = stream_name_kline("btcusdt", "1m");
    let aggtrade_stream = stream_name_aggtrade("solusdt");
    let mut ws = binance::futures::websocket::connect_combined(&[kline_stream, aggtrade_stream])
        .await
        .unwrap();
    loop {
        match ws.next().await {
            None => {
                println!("WebSocket stream is done.");
                break;
            }
            Some(Err(err)) => {
                println!("WebSocket error: {:?}", err);
                break;
            }
            Some(Ok(event)) => {
                dbg!(event);
            }
        }
    }
}
