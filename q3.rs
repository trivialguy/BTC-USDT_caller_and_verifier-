use serde::Deserialize;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{self, Duration};
use tokio::task;
use hmac::{Hmac,Mac};
use sha2::Sha256;

#[derive(Deserialize, Debug, Clone)]
struct Ticker {
    symbol: String,
    price: String,
}

// Struct to hold data for each thread
#[derive(Debug)]
struct ThreadData {
    signed_message: String,
    // Use a single symmetric key for illustration purposes
    secret_key: Vec<u8>,
    average: f64,
}

impl ThreadData {
    // Function to verify the signature using the secret key
    fn verify_signature(&self) -> bool {
        // Create an HMAC instance with SHA-256
        let mut hmac = Hmac::<Sha256>::new_from_slice(&self.secret_key).expect("HMAC creation failed");

        // Convert the average to bytes and update the HMAC
        hmac.update(&self.average.to_be_bytes());

        // Verify the signature
        let signature_bytes = hex::decode(&self.signed_message).expect("Invalid hex");
        hmac.verify_slice(&signature_bytes).is_ok()
    }
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
    // println!("All Responses: {:#?}", responses);

    let mut data_file = File::create("data.txt").expect("creation failed");

    // Write contents to the file
    let mut res: f64 = 0.0;
    let length = responses.len();

    // Store all the thread data in a vector
    let mut thread_data_vec = Vec::new();

    for item in responses.iter() {
        let num: f64 = item.price.parse().unwrap();
        res += num;

        // Calculate and store the average within the loop
        let average: f64 = res / (thread_data_vec.len() + 1) as f64;

        // Use a simple symmetric key for illustration purposes
        let secret_key = b"my_secret_key".to_vec();

        // Create a new ThreadData instance and push it to the vector
        let thread_data = ThreadData {
            signed_message: sign_message(&secret_key, &average),
            secret_key,
            average,
        };
        thread_data_vec.push(thread_data);

        data_file.write("price: ".as_bytes()).expect("write failed");
        data_file.write(item.price.as_bytes()).expect("write failed");
        data_file.write("\n".as_bytes()).expect("write failed");
    }

    data_file.write("Thread Data: {:#?}\n".as_bytes()).expect("write failed");

    // Print the thread data and verify signatures
    for data in thread_data_vec.iter() {
        data_file
            .write(format!("{:#?}\n", data).as_bytes())
            .expect("write failed");

        // Verify the signature for each thread data
        if data.verify_signature() {
            println!("Signature verified for thread data: {:#?}", data);
        } else {
            println!("Signature verification failed for thread data: {:#?}", data);
        }
    }

    // Calculate and print the average of averages
    let avg_of_avgs = aggregate(&thread_data_vec);
    println!("Average of Averages: {:.2}", avg_of_avgs);


    Ok(())
}

// Function to sign a message using HMAC with SHA-256
fn sign_message(secret_key: &[u8], message: &f64) -> String {
    // Create an HMAC instance with SHA-256
    let mut hmac = Hmac::<Sha256>::new_from_slice(secret_key).expect("HMAC creation failed");

    // Convert the message to bytes and update the HMAC
    hmac.update(&message.to_be_bytes());

    // Get the HMAC result as bytes and convert to hexadecimal
    let result_bytes = hmac.finalize().into_bytes();
    hex::encode(result_bytes)
}

fn aggregate(thread_data_vec: &[ThreadData]) -> f64 {
    if thread_data_vec.is_empty() {
        return 0.0;
    }

    // Verify signatures before calculating the average
    let all_signatures_valid = thread_data_vec
        .iter()
        .all(|data| data.verify_signature());

    if !all_signatures_valid {
        println!("Not all signatures are valid!");
        return 0.0;
    }

    thread_data_vec
        .iter()
        .map(|data| data.average)
        .sum::<f64>()
        / thread_data_vec.len() as f64
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
                    // println!("Response Body: {}", body);

                    let ticker: Ticker = serde_json::from_str(&body).unwrap();
                    // println!("Parsed Response: {:#?}", ticker);

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
