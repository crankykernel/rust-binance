use binance::futures::websocket::{agg_trade_stream, kline_stream};
use futures_util::StreamExt;

#[tokio::main]
pub async fn main() {
    println!("Futures WebSocket Example.");
    let kline_stream = kline_stream("btcusdt", "1m");
    let aggtrade_stream = agg_trade_stream("solusdt");
    let (mut ws, _response) =
        binance::futures::websocket::connect_combined(&[kline_stream, aggtrade_stream])
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
