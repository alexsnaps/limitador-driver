use std::fmt::{Display, Formatter};
use std::mem::transmute;
use std::time::Duration;
use clap::Parser;
use hdrhistogram::Histogram;
use tokio::time::Instant;
use tonic::Response;
use proto::envoy::service::ratelimit::v3::rate_limit_service_client::RateLimitServiceClient;
use proto::envoy::service::ratelimit::v3::{
    RateLimitRequest, RateLimitResponse,
};
use crate::proto::envoy::service::ratelimit::v3::rate_limit_response::Code;

mod proto;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct CliConfiguration {
    url: String,
    duration: Option<String>,
}

struct Configuration {
    url: String,
    duration: Duration,
}

impl TryFrom<CliConfiguration> for Configuration {
    type Error = String;

    fn try_from(cli: CliConfiguration) -> Result<Self, Self::Error> {
        let duration = &cli.duration.unwrap_or("10m".to_string());
        let duration = match parse_duration::parse(duration) {
            Ok(dur) => dur,
            Err(err) => return Err(format!("Couldn't parse duration from \"{}\"\n\t{}", duration, err)),
        };

        Ok(Self {
            url: cli.url,
            duration,
        })
    }
}

impl Display for Configuration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Hitting Limitador at {} for a total duration of {} minutes", self.url, self.duration.as_secs_f64() / 60.0)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = CliConfiguration::parse();
    let cfg: Result<Configuration, String> = cfg.try_into();
    match cfg {
        Ok(cfg) => {
            println!("{}", cfg);
            let mut client = RateLimitServiceClient::connect(cfg.url).await?;

            let timeout = Duration::from_secs(5).as_micros() as u64;

            let mut ok_histogram: Histogram<u64> = Histogram::new_with_max(timeout, 2).unwrap();
            let mut over_histogram: Histogram<u64> = Histogram::new_with_max(timeout, 2).unwrap();
            let mut unknown_histogram: Histogram<u64> = Histogram::new_with_max(timeout, 2).unwrap();
            let mut period_histogram: Histogram<u64> = Histogram::new_with_max(timeout, 2).unwrap();
            let mut histogram: Histogram<u64> = Histogram::new_with_max(timeout, 2).unwrap();

            let start = Instant::now();
            let mut period = Instant::now();
            while start.elapsed() < cfg.duration {
                let request = tonic::Request::new(RateLimitRequest {
                    domain: "test_namespace".to_string(),
                    descriptors: vec![],
                    hits_addend: 1,
                });

                let round_trip = Instant::now();
                let response: Response<RateLimitResponse> = client.should_rate_limit(request).await?;
                let code: Code = unsafe { transmute(response.into_inner().overall_code) };
                let elapsed = round_trip.elapsed();
                let h = match code {
                    Code::Unknown => &mut unknown_histogram,
                    Code::Ok => &mut ok_histogram,
                    Code::OverLimit => &mut over_histogram,
                };
                let elapsed = elapsed.as_micros() as u64;
                h.record(elapsed).expect("Value should be between bounds");
                period_histogram.record(elapsed).expect("Value should be between bounds");
                histogram.record(elapsed).expect("Value should be between bounds");

                if period.elapsed() > Duration::from_secs(1) {
                    print_h(&period_histogram, "Current");
                    period = Instant::now();
                    period_histogram.reset();
                }
            }
            println!("\n\n ============== Done! =================\n");
            print_h(&ok_histogram, "Ok");
            print_h(&over_histogram, "OverLimit");
            print_h(&unknown_histogram, "Unknown");
            print_h(&histogram, "Overall");

            println!();
            for v in break_once(histogram.iter_linear(histogram.value_at_quantile(0.99) / 10), |v| v.quantile() > 0.999) {
                println!(
                    "{:4.2}ms | {:80} | {:4.1}th %-ile",
                    (v.value_iterated_to() + 1) as f64 / 1000.0,
                    "*".repeat(
                        (v.count_since_last_iteration() as f64 * 80.0 / histogram.len() as f64).ceil() as usize
                    ),
                    v.percentile()
                );
            }
        }
        Err(err) => {
            eprintln!("Error:\t{}", err);
        }
    }

    Ok(())
}

fn print_h(histogram: &Histogram<u64>, prefix: &str) {
    println!(
        "{}: hits: {}, mean: {:.3}ms, p50: {:.3}ms, p90: {:.3}ms, p99: {:.3}ms, p999: {:.3}ms, max: {:.3}ms",
        prefix,
        histogram.len(),
        histogram.mean() / 1_000.0,
        histogram.value_at_quantile(0.5) as f64 / 1_000.0,
        histogram.value_at_quantile(0.9) as f64 / 1_000.0,
        histogram.value_at_quantile(0.99) as f64 / 1_000.0,
        histogram.value_at_quantile(0.999) as f64 / 1_000.0,
        histogram.max() as f64 / 1_000.0,
    );
}

fn break_once<I, F>(it: I, mut f: F) -> impl Iterator<Item=I::Item>
    where
        I: IntoIterator,
        F: FnMut(&I::Item) -> bool,
{
    let mut got_true = false;
    it.into_iter().take_while(move |i| {
        if got_true {
            // we've already yielded when f was true
            return false;
        }
        if f(i) {
            // this must be the first time f returns true
            // we should yield i, and then no more
            got_true = true;
        }
        // f returned false, so we should keep yielding
        true
    })
}
