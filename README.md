# edgar-lib

A Rust library and CLI for working with the SEC EDGAR API. Look up companies, pull filings, get standardized financials, and watch for new filings in real time.

## Install

Add it to your project:

```
cargo add edgar-lib
```

Or build the CLI:

```
cargo build --release
```

The binary is called `edgar`.

## Library usage

Everything goes through `EdgarClient`. The SEC requires a user-agent with your email.

```rust
use edgar_lib::EdgarClient;

#[tokio::main]
async fn main() -> edgar_lib::Result<()> {
    let client = EdgarClient::new("you@example.com")?;

    let company = client.company("AAPL").await?;
    println!("{} (CIK {})", company.name, company.cik);

    let filings = client.filings("AAPL").await?;
    for f in filings.iter().take(5) {
        println!("{} {}", f.filing_date, f.filing_type);
    }

    Ok(())
}
```

### Standardized financials

The library maps raw XBRL tags to common metric names so you can compare across companies without dealing with the tag mess yourself.

```rust
let client = EdgarClient::builder("you@example.com")
    .tag_store_path("discovered_tags.json")
    .build()?;

let financials = client.financials("MSFT").await?;
for period in &financials.annual {
    println!("{}", period.period);
    for (metric, mv) in &period.metrics {
        println!("  {:?}: {:.0}", metric, mv.value);
    }
}
```

### Watch for new filings

```rust
use edgar_lib::watcher::filter::WatchFilter;
use edgar_lib::watcher::WatcherConfig;

let filter = WatchFilter::new()
    .with_form_types(["10-K".to_string(), "10-Q".to_string()]);

let config = WatcherConfig {
    poll_interval: std::time::Duration::from_secs(60),
    filter,
    ..Default::default()
};

let mut handle = client.start_watcher(config);
```

## CLI

```
edgar company AAPL
edgar filings AAPL --form-type 10-K
edgar financials MSFT
edgar search "artificial intelligence" --form-type 8-K
edgar watch --form-types 10-K,10-Q
edgar frames us-gaap Revenue USD 2024
edgar bulk companies
```

Output formats: table (default), json, csv.

## How it works

- Rate limited to 10 requests per second per SEC guidelines
- Responses are cached in memory (LRU, 24h TTL) so repeated lookups are fast
- XBRL tag standardization uses ordered fallback chains to find the right tag for each metric
- The watcher polls the EDGAR RSS feed and broadcasts new filings over a channel

## License

MIT
