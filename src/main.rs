use std::{env, fs, path::Path, process::exit};

use serde_json::Value;

fn main() {
    let args: Vec<String> = env::args().collect();
    let Some(dir) = args.get(1) else {
        eprintln!("Usage: {} <directory>", args[0]);
        exit(1);
    };

    let path = Path::new(dir);
    if !path.exists() {
        eprintln!("Error: Directory does not exist");
        exit(1);
    }

    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let ip_port_path = entry.path();

        if !ip_port_path.is_dir() {
            eprintln!("Error: {} is not a directory", ip_port_path.display());
            exit(1);
        }

        for file_entry in fs::read_dir(ip_port_path).unwrap() {
            let file_entry = file_entry.unwrap();
            let file_path = file_entry.path();

            if !file_path.is_file() {
                eprintln!("Error: {} is not a file", file_path.display());
                exit(1);
            }

            if file_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .contains("latest")
            {
                continue;
            }

            let contents = fs::read_to_string(&file_path);
            let contents = match contents {
                Ok(contents) => contents,
                Err(e) => {
                    eprintln!("Error reading file {}: {}", file_path.display(), e);
                    exit(1);
                }
            };

            match serde_json::from_str::<Value>(&contents) {
                Ok(json) => {
                    println!("Checking: {}", file_path.display());
                    check_description(&json, &args);
                }
                Err(_) => {
                    println!("Invalid JSON: {}", file_path.display());
                }
            }
        }
    }
}

fn check_description(json: &Value, args: &[String]) {
    let res = json.get("description").and_then(parse_description);
}

fn parse_description(desc: &Value) -> Option<String> {
    match desc {
        Value::String(s) => Some(s.clone()),
        Value::Array(arr) => {
            let mut result = String::new();
            for item in arr {
                if let Some(Value::String(text)) = item.get("text") {
                    result.push_str(text);
                }
            }
            Some(result)
        }
        Value::Object(map) => {
            let mut result = String::new();

            if let Some(Value::String(text)) = map.get("text") {
                result.push_str(text);
            }

            if let Some(Value::Array(extra)) = map.get("extra") {
                for item in extra {
                    if let Some(Value::String(text)) = item.get("text") {
                        result.push_str(text);
                    }
                }
            }

            Some(result)
        }
        _ => None,
    }
}
