use std::{env, fs, path::Path, process::exit};

use regex::Regex;
use serde_json::Value;

fn main() {
    let args: Vec<String> = env::args().collect();
    let filters = parse_filters(args.as_slice());
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

        for file_entry in fs::read_dir(ip_port_path.clone()).unwrap() {
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
                if filters.latest.is_none() {
                    continue;
                }
            } else if filters.latest.is_some() && filters.latest.unwrap() {
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
                    check_description(
                        &json,
                        &filters,
                        &file_path.display().to_string(),
                        ip_port_path
                            .display()
                            .to_string()
                            .split(":")
                            .collect::<Vec<&str>>()[0]
                            .split("/")
                            .collect::<Vec<&str>>()
                            .last()
                            .unwrap(),
                    );
                }
                Err(_) => {
                    println!("Invalid JSON: {}", file_path.display());
                }
            }
        }
    }
}

#[derive(Debug)]
struct Version {
    name: Option<String>,
    protocol: Option<i32>,
}

#[derive(Debug)]
struct Players {
    max: i32,
    online: i32,
    sample: Option<Vec<Player>>,
}

#[derive(Debug)]
struct Player {
    name: Option<String>,
    id: Option<String>,
}

fn check_description(json: &Value, filters: &Filters, filename: &str, ip: &str) {
    let description = json.get("description");
    let description = if let Some(description) = description {
        parse_description(description)
    } else {
        "".to_string()
    };

    let version = json.get("version");
    let version = if let Some(version) = version {
        parse_version(version)
    } else {
        Version {
            name: None,
            protocol: None,
        }
    };

    let enforces_secure_chat = json.get("enforcesSecureChat");
    let enforces_secure_chat = if let Some(enforces_secure_chat) = enforces_secure_chat {
        enforces_secure_chat.as_bool().unwrap_or(false)
    } else {
        false
    };

    let favicon = json.get("favicon");
    let favicon = if let Some(favicon) = favicon {
        favicon.as_str().unwrap_or("")
    } else {
        ""
    };

    let players = json.get("players");
    let players = if let Some(players) = players {
        parse_players(players)
    } else {
        return;
    };

    if let Some(regex) = &filters.name_regex {
        if let Some(sample) = &players.sample {
            let matched = sample
                .iter()
                .any(|p| p.name.as_ref().is_some_and(|name| regex.is_match(name)));

            if !matched {
                return;
            }
        } else {
            return;
        }
    }

    if !(filters.latest.is_some() && filters.latest.unwrap()) {
        let temp = filename.split("/").collect::<Vec<&str>>();
        let temp = temp.last().unwrap();
        let temp = temp.split("_").collect::<Vec<&str>>();
        let temp = temp.first().unwrap().split("-").collect::<Vec<&str>>();
        let year = temp[0].parse::<i32>().unwrap();
        let month = temp[1].parse::<i32>().unwrap();
        let day = temp[2].parse::<i32>().unwrap();

        if let Some(date) = &filters.min_date {
            let temp = date.split("-").collect::<Vec<&str>>();
            let min_year = temp[0].parse::<i32>().unwrap_or(0);
            let min_month = temp[1].parse::<i32>().unwrap_or(0);
            let min_day = temp[2].parse::<i32>().unwrap_or(0);
            if year < min_year
                || (year == min_year && month < min_month)
                || (year == min_year && month == min_month && day < min_day)
            {
                return;
            }
        }

        if let Some(date) = &filters.max_date {
            let temp = date.split("-").collect::<Vec<&str>>();
            let max_year = temp[0].parse::<i32>().unwrap_or(0);
            let max_month = temp[1].parse::<i32>().unwrap_or(0);
            let max_day = temp[2].parse::<i32>().unwrap_or(0);
            if year > max_year
                || (year == max_year && month > max_month)
                || (year == max_year && month == max_month && day > max_day)
            {
                return;
            }
        }
    }

    if let Some(max) = filters.max_players {
        if players.online > max {
            return;
        }
    }

    if let Some(min) = filters.min_players {
        if players.online < min {
            return;
        }
    }

    if let Some(regex) = &filters.version_regex {
        if let Some(version_name) = &version.name {
            if !regex.is_match(version_name) {
                return;
            }
        }
    }

    if let Some(regex) = &filters.description_regex {
        if !regex.is_match(&description) {
            return;
        }
    }

    if let Some(min) = filters.min_version {
        if version.protocol.unwrap_or(0) < min {
            return;
        }
    }

    if let Some(max) = filters.max_version {
        if version.protocol.unwrap_or(0) > max {
            return;
        }
    }

    if let Some(max) = filters.max_max_players {
        if players.max > max {
            return;
        }
    }

    if let Some(min) = filters.min_max_players {
        if players.max < min {
            return;
        }
    }

    if let Some(enforces_secure_chat_bool) = filters.enforces_secure_chat {
        if players.online > 0 && enforces_secure_chat != enforces_secure_chat_bool {
            return;
        }
    }

    if let Some(regex) = &filters.id_regex {
        if let Some(sample) = &players.sample {
            let matched = sample
                .iter()
                .any(|p| p.id.as_ref().is_some_and(|id| regex.is_match(id)));

            if !matched {
                return;
            }
        }
    }

    if let Some(regex) = &filters.ip_regex {
        if !regex.is_match(ip) {
            return;
        }
    }

    println!("Server found:");
    println!("IP: {}", ip);
    println!("Description: {}", description);
    println!("Version: {:?}", version);
    println!("Enforces Secure Chat: {}", enforces_secure_chat);
    println!("Favicon: {}", favicon);
    println!("Players: {:?}", players);
    println!("------------------------------");
    println!();
}

#[derive(Debug, Default)]
struct Filters {
    name_regex: Option<regex::Regex>,
    version_regex: Option<regex::Regex>,
    min_version: Option<i32>,
    max_version: Option<i32>,
    description_regex: Option<regex::Regex>,
    min_players: Option<i32>,
    max_players: Option<i32>,
    max_max_players: Option<i32>,
    min_max_players: Option<i32>,
    enforces_secure_chat: Option<bool>,
    id_regex: Option<regex::Regex>,
    ip_regex: Option<regex::Regex>,
    min_date: Option<String>,
    max_date: Option<String>,
    latest: Option<bool>,
}

fn parse_filters(args: &[String]) -> Filters {
    let mut filters = Filters::default();
    let mut i = 2;

    while i < args.len() {
        match args[i].as_str() {
            "--name" if i + 1 < args.len() => {
                let regex_pattern = &args[i + 1];
                if let Ok(regex) = Regex::new(regex_pattern) {
                    filters.name_regex = Some(regex);
                } else {
                    eprintln!("Error: Invalid regex pattern '{}'", regex_pattern);
                    exit(1);
                }
                i += 2;
            }
            "--version" | "--ver" if i + 1 < args.len() => {
                let regex_pattern = &args[i + 1];
                if let Ok(regex) = Regex::new(regex_pattern) {
                    filters.version_regex = Some(regex);
                } else {
                    eprintln!("Error: Invalid regex pattern '{}'", regex_pattern);
                    exit(1);
                }
                i += 2;
            }
            "--description" | "--desc" if i + 1 < args.len() => {
                let regex_pattern = &args[i + 1];
                if let Ok(regex) = Regex::new(regex_pattern) {
                    filters.description_regex = Some(regex);
                } else {
                    eprintln!("Error: Invalid regex pattern '{}'", regex_pattern);
                    exit(1);
                }
                i += 2;
            }
            "--min-player" | "--min-players" if i + 1 < args.len() => {
                if let Ok(min) = args[i + 1].parse::<i32>() {
                    filters.min_players = Some(min);
                } else {
                    eprintln!("Error: Invalid value for --min-player");
                    exit(1);
                }
                i += 2;
            }
            "--max-player" | "--max-players" if i + 1 < args.len() => {
                if let Ok(max) = args[i + 1].parse::<i32>() {
                    filters.max_players = Some(max);
                } else {
                    eprintln!("Error: Invalid value for --max-player");
                    exit(1);
                }
                i += 2;
            }
            "--max-max-player" | "--max-max-players" if i + 1 < args.len() => {
                if let Ok(max) = args[i + 1].parse::<i32>() {
                    filters.max_max_players = Some(max);
                } else {
                    eprintln!("Error: Invalid value for --max-max-player");
                    exit(1);
                }
                i += 2;
            }
            "--min-max-player" | "--min-max-players" if i + 1 < args.len() => {
                if let Ok(min) = args[i + 1].parse::<i32>() {
                    filters.min_max_players = Some(min);
                } else {
                    eprintln!("Error: Invalid value for --min-max-player");
                    exit(1);
                }
                i += 2;
            }
            "--enforces-secure-chat" | "--secure-chat" if i + 1 < args.len() => {
                let value = &args[i + 1];
                if value == "true" {
                    filters.enforces_secure_chat = Some(true);
                } else if value == "false" {
                    filters.enforces_secure_chat = Some(false);
                } else {
                    eprintln!("Error: Invalid value for --enforces-secure-chat");
                    exit(1);
                }
                i += 2;
            }
            "--min-version" | "--min-ver" if i + 1 < args.len() => {
                if let Ok(min) = args[i + 1].parse::<i32>() {
                    filters.min_version = Some(min);
                } else {
                    eprintln!("Error: Invalid value for --min-version");
                    exit(1);
                }
                i += 2;
            }
            "--max-version" | "--max-ver" if i + 1 < args.len() => {
                if let Ok(max) = args[i + 1].parse::<i32>() {
                    filters.max_version = Some(max);
                } else {
                    eprintln!("Error: Invalid value for --max-version");
                    exit(1);
                }
                i += 2;
            }
            "--id" | "--player-id" | "--uuid" if i + 1 < args.len() => {
                let regex_pattern = &args[i + 1];
                if let Ok(regex) = Regex::new(regex_pattern) {
                    filters.id_regex = Some(regex);
                } else {
                    eprintln!("Error: Invalid regex pattern '{}'", regex_pattern);
                    exit(1);
                }
                i += 2;
            }
            "--ip" | "--addr" if i + 1 < args.len() => {
                let regex_pattern = &args[i + 1];
                if let Ok(regex) = Regex::new(regex_pattern) {
                    filters.ip_regex = Some(regex);
                } else {
                    eprintln!("Error: Invalid regex pattern '{}'", regex_pattern);
                    exit(1);
                }
                i += 2;
            }
            "--min-date" | "--min-date" if i + 1 < args.len() => {
                filters.min_date = Some(args[i + 1].clone());
                i += 2;
            }
            "--max-date" | "--max-date" if i + 1 < args.len() => {
                filters.max_date = Some(args[i + 1].clone());
                i += 2;
            }
            "--date" if i + 1 < args.len() => {
                filters.min_date = Some(args[i + 1].clone());
                filters.max_date = Some(args[i + 1].clone());
                i += 2;
            }
            "--latest" | "--latest-only" => {
                filters.latest = Some(true);
                i += 1;
            }
            "--help" | "-h" => {
                println!("Usage: <dir> <options>");
                println!("Options:");
                println!("  --name <regex>               Filter by player name regex");
                println!("  --version <regex>            Filter by version regex");
                println!("  --description <regex>        Filter by description regex");
                println!("  --min-player <number>        Minimum number of players");
                println!("  --max-player <number>        Maximum number of players");
                println!("  --max-max-player <number>    Maximum max player count");
                println!("  --min-max-player <number>    Minimum max player count");
                println!("  --enforces-secure-chat       Enforces secure chat");
                println!("  --min-version <number>       Minimum version protocol");
                println!("  --max-version <number>       Maximum version protocol");
                println!("  --id <regex>                Filter by player ID regex");
                println!("  --ip <regex>                Filter by IP regex");
                println!("  --min-date <date (YYYY-MM-DD)>           Minimum date");
                println!("  --max-date <date (YYYY-MM-DD)>           Maximum date");
                println!("  --latest                     Only parse latest scan");
                println!("  --date <date>               Filter by date");
                println!("  --help, -h                  Show this help message");
                println!("  --version, -v               Show version");
                exit(0);
            }
            "--version" | "-v" => {
                println!("Version: 1.0.0");
                exit(0);
            }
            _ => {
                eprintln!("Error: Unknown argument '{}'", args[i]);
                exit(1);
            }
        }
    }

    filters
}

fn parse_players(players: &Value) -> Players {
    let mut max = 0;
    let mut online = 0;
    let mut sample = None;

    if let Some(Value::Number(v)) = players.get("max") {
        max = v.as_i64().unwrap_or(0) as i32;
    }

    if let Some(Value::Number(v)) = players.get("online") {
        online = v.as_i64().unwrap_or(0) as i32;
    }

    if let Some(Value::Array(arr)) = players.get("sample") {
        let mut sample_vec = Vec::new();
        for item in arr {
            let mut final_name = None;
            let mut final_id = None;
            if let Some(Value::String(name)) = item.get("name") {
                final_name = Some(name.clone());
            }
            if let Some(Value::String(id)) = item.get("id") {
                final_id = Some(id.clone());
            }
            sample_vec.push(Player {
                name: final_name,
                id: final_id,
            });
        }
        sample = Some(sample_vec);
    }

    Players {
        max,
        online,
        sample,
    }
}

fn parse_version(version: &Value) -> Version {
    let mut name = None;
    let mut protocol = None;

    if let Some(Value::String(v)) = version.get("name") {
        name = Some(v.clone());
    }

    if let Some(Value::Number(v)) = version.get("protocol") {
        protocol = v.as_i64().map(|v| v as i32);
    }

    Version { name, protocol }
}

fn parse_description(desc: &Value) -> String {
    match desc {
        Value::String(s) => s.clone(),
        Value::Array(arr) => {
            let mut result = String::new();
            for item in arr {
                if let Some(Value::String(text)) = item.get("text") {
                    result.push_str(text);
                }
            }
            result
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

            result
        }
        _ => "".to_string(),
    }
}
