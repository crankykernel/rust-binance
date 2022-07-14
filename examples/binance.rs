use anyhow::Result;
use binance::common::client::Authentication;
use clap::Parser;

#[derive(clap::Parser, Debug)]
enum Opts {
    Spot,
    #[clap(subcommand)]
    Futures(Futures),
}

#[derive(clap::Parser, Debug)]
enum Futures {
    Buy(FuturesBuyOptions),
    OpenOrders(OpenOrdersOpts),
    Cancel(CancelOrderOpts),
}

#[derive(clap::Parser, Debug)]
struct OpenOrdersOpts {
    #[clap(long)]
    symbol: Option<String>,
}

#[derive(clap::Parser, Debug)]
struct CancelOrderOpts {
    #[clap(long)]
    symbol: String,
    #[clap(long)]
    order_id: Option<u64>,
    #[clap(long)]
    client_order_id: Option<String>,
}

#[derive(clap::Parser, Debug)]
struct FuturesBuyOptions {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opts: Opts = Opts::parse();
    match opts {
        Opts::Futures(Futures::Cancel(opts)) => futures_cancel_order(opts).await,
        Opts::Futures(Futures::OpenOrders(opts)) => futures_open_orders(opts).await,
        _ => unimplemented!(),
    }
}

fn get_binance_authentication() -> Result<Authentication> {
    Ok(Authentication {
        api_key: std::env::var("API_KEY")?,
        api_secret: std::env::var("API_SECRET")?,
    })
}

async fn futures_open_orders(opts: OpenOrdersOpts) -> Result<()> {
    let auth = get_binance_authentication()?;
    let client = binance::futures::client::Client::new(Some(auth));
    let orders = client.get_open_orders(opts.symbol.clone()).await?;
    for order in orders {
        dbg!(order);
    }
    Ok(())
}

async fn futures_cancel_order(opts: CancelOrderOpts) -> Result<()> {
    let auth = get_binance_authentication()?;
    let cancel_order_request = binance::types::CancelOrder {
        symbol: opts.symbol,
        order_id: opts.order_id,
        client_order_id: opts.client_order_id,
    };
    let client = binance::futures::client::Client::new(Some(auth));
    match client.cancel_order(&cancel_order_request).await {
        Ok(response) => println!("success: {:?}", response),
        Err(error) => println!("error: {:?}", error),
    }
    Ok(())
}

async fn futures_buy_order(options: FuturesBuyOptions) -> Result<()> {
    let auth = get_binance_authentication()?;

    Ok(())
}
