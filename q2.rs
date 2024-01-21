use serde::Deserialize;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{self, Duration};
use tokio::task;

#[derive(Deserialize, Debug, Clone)]
struct Ticker {
    symbol: String,
    price: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let responses = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::new();

    for _ in 0..5 {
        let responses_clone = Arc::clone(&responses);
        let handle = tokio::spawn(task_handler(responses_clone));
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.await?;
    }

    // After all threads have completed, collect the responses
    let responses = responses.lock().await.clone();

    // Print all collected responses
    println!("All Responses: {:#?}", responses);

    let mut data_file = File::create("data.txt").expect("creation failed");

    // Write contents to the file
    let mut res: f64 = 0.0;
    let length = responses.len();

    // Store all the averages in an array
    let mut averages = Vec::new();

    for item in responses.iter() {
        let num: f64 = item.price.parse().unwrap();
        res += num;

        // Calculate and store the average within the loop
        let average: f64 = res / (averages.len() + 1) as f64;
        averages.push(average);

        data_file.write("price: ".as_bytes()).expect("write failed");
        data_file.write(item.price.as_bytes()).expect("write failed");
        data_file.write("\n".as_bytes()).expect("write failed");
    }

    data_file.write("Averages: ".as_bytes()).expect("write failed");
    for avg in averages.iter() {
        data_file.write(format!("{:.2} ", avg).as_bytes()).expect("write failed");
    }

    // Calculate and print the average of averages
    let avg_of_avgs = aggregate(&averages);
    println!("Average of Averages: {:.2}", avg_of_avgs);

    println!("Cache complete. The averages of USD price of BTC are: {:?}", averages);

    Ok(())
}

fn aggregate(averages: &[f64]) -> f64 {
    if averages.is_empty() {
        0.0
    } else {
        averages.iter().sum::<f64>() / averages.len() as f64
    }
}

async fn task_handler(responses: Arc<Mutex<Vec<Ticker>>>) {
    let url = "https://api.binance.com/api/v3/ticker/price?symbol=BTCUSDT";
    let timeout_duration = Duration::from_secs(10);

    let client = reqwest::Client::new();
    let start_time = std::time::Instant::now();

    while start_time.elapsed() < timeout_duration {
        let resp_result = client.get(url).send().await;

        match resp_result {
            Ok(resp) => {
                if resp.status().is_success() {
                    let body = resp.text().await.unwrap();
                    println!("Response Body: {}", body);

                    let ticker: Ticker = serde_json::from_str(&body).unwrap();
                    println!("Parsed Response: {:#?}", ticker);

                    // Lock the mutex asynchronously, update the vector, and unlock immediately
                    let mut response_data = responses.lock().await;
                    response_data.push(ticker.clone());
                } else {
                    println!("Request failed with status: {}", resp.status());
                }
            }
            Err(err) => {
                println!("{}", err);
            }
        }

        // Introduce a delay between requests to avoid tight loops
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    // After 10 seconds, print the collected responses and the final average in this thread
    println!("Thread completed. Responses: {:#?}", responses.lock().await);
}
