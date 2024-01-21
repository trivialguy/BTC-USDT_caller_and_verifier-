use serde::Deserialize;
use tokio::time::{self, Duration};
use std::{ptr::null, vec::Vec};
use structopt::StructOpt;
use std::fs::File;
use std::io::Read;
use std::io::Write;

#[derive(StructOpt, Debug)]
#[structopt(name = "simple", about = "A simple client")]
struct Opt {
    #[structopt(short, long)]
    mode: String,

    #[structopt(short, long)]
    times: Option<u64>,
}

#[derive(Deserialize, Debug)]
struct Ticker {
    symbol: String,
    price: f32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let opt = Opt::from_args();

    match opt.mode.as_str() {
        "cache" => {
            let times = opt.times.unwrap_or(1);
            // println!("./simple --mode=cache --times={}", times);
            let url = "https://api.binance.com/api/v3/ticker/price?symbol=BTCUSDT";

    // Set a 10-second timeout
    let timeout_duration = Duration::from_secs(times);

    // Create an HTTP client
    let client = reqwest::Client::new();

    // Vector to collect responses
    let mut responses: Vec<Ticker> = Vec::new();

    // Use a loop to make requests within the 10-second window
    let start_time = std::time::Instant::now();
    loop {
        // Check if 10 seconds have passed
        if start_time.elapsed() >= timeout_duration {
            break;
        }

        // Use the timeout function to set a timeout for each request
        match time::timeout(
            timeout_duration - start_time.elapsed(),
            client.get(url).send(),
        )
        .await
        .map_err(|_| "Request failed: {}") {
            Ok(resp_result) => {
                // Extract the response from the Result
                let resp = resp_result.map_err(|e| reqwest::Error::from(e));

                // Process the response
                match resp {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            // Print the response body
                            let body = resp.text().await?;
                            println!("Response Body: {}", body);

                            // Parse the JSON response into a Ticker struct
                            let ticker: Ticker = serde_json::from_str(&body)?;
                            println!("Parsed Response: {:#?}", ticker);

                            // Collect the response in the vector
                            responses.push(ticker);
                        } else {
                            // Print an error message if the request was not successful
                            println!("Request failed with status: {}", resp.status());
                        }
                    }
                    Err(err) => {
                        // Handle timeout or other errors
                        println!("{}", err);
                    }
                }
            }
            Err(_) => {
                // Handle timeout error
                println!("Request timed out");
            }
        }
    }

    // Print all collected responses
    println!("All Responses: {:#?}", responses);
    let mut data_file = File::create("data.txt").expect("creation failed");

    // Write contents to the file
    let mut res:f64=0.0;
    let length=responses.len();
    for item in responses.iter(){
        let num:f64=item.price.parse().unwrap();
        res+=num;
        data_file.write("price: ".as_bytes()).expect("write failed");
        data_file.write(item.price.as_bytes()).expect("write failed");
        data_file.write("\n".as_bytes()).expect("write failed");
    }
    data_file.write("Average: ".as_bytes()).expect("write failed");
    res=res/length as f64;
    data_file.write((res.to_string()).as_bytes()).expect("write failed");
    println!("Cache complete. The average USD price of BTC is: {}",res);
            // Call your command here if needed
        }
        "read" => {
            println!("./simple --mode=read");
            let mut data_file = File::open("data.txt").unwrap();

    // Create an empty mutable string
            let mut file_content = String::new();

            // Copy contents of file to a mutable string
            data_file.read_to_string(&mut file_content).unwrap();

            println!(file_content);
            
        }
        _ => {
            eprintln!("Unknown mode: {}", opt.mode);
            std::process::exit(1);
        }
    }
    // Example URL for Binance symbol price ticker
    

    Ok(())
}

