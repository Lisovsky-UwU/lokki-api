//! Eyeball what an OpenAPI import would build, without writing anything:
//!
//! ```text
//! cargo run --example import_spec -- path/to/openapi.yaml
//! ```
//!
//! Prints the collection, its folders and environments, and the first
//! request of each folder in full - which is the fastest way to tell
//! whether a specification in the wild reads the way it should.

use lokki_api_lib::import::openapi;

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: cargo run --example import_spec -- <specification file>");
        std::process::exit(2);
    };
    let text = std::fs::read_to_string(&path).expect("read the specification");
    let plan = openapi::parse(&text).expect("parse the specification");

    println!("collection: {}", plan.collection_name);
    for environment in &plan.environments {
        println!("environment: {}", environment.name);
        for variable in &environment.variables {
            println!("    {} = {:?} (secret: {})", variable.key, variable.value, variable.secret);
        }
    }
    println!("requests: {}", plan.request_count());

    for folder in &plan.folders {
        println!("\nfolder {} ({} requests)", folder.name, folder.requests.len());
        for request in &folder.requests {
            let http = request.http.as_ref().unwrap();
            println!("    {:?} {} - {}", http.method, http.url, request.meta.name);
        }
        if let Some(first) = folder.requests.first() {
            println!("    first in full: {:#?}", first.http.as_ref().unwrap());
        }
    }
    for request in &plan.requests {
        let http = request.http.as_ref().unwrap();
        println!("\nat the root: {:?} {} - {}", http.method, http.url, request.meta.name);
    }

    for warning in &plan.warnings {
        println!("\nwarning: {warning}");
    }
}
