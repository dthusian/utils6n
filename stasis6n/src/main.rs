use std::collections::HashMap;
use std::env;
use std::env::{VarError, VarsOs};
use std::error::Error;
use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::process::Command;
use serde::Serialize;
use serde::Deserialize;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct Config {
    pub apps: HashMap<String, ConfigApp>
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct ConfigApp {
    pub patterns: Vec<String>
}

fn get_config_path() -> Result<String, impl Error> {
    match env::var("XDG_CONFIG_HOME") {
        Ok(v) => Result::Ok(v + "/stasis6n.yaml"),
        Err(_) => {
            match env::var("HOME") {
                Ok(v) => Result::Ok(v + "/.config/stasis6n.yaml"),
                Err(_) => Result::Err(std::io::Error::from(ErrorKind::NotFound))
            }
        }
    }
}

fn read_config_file() -> Result<Config, Box<dyn Error>> {
    let mut file = File::options().read(true).append(true).create(true).open(get_config_path()?)?;
    let mut s = String::new();
    file.read_to_string(&mut s)?;
    if s.is_empty() {
        Result::Ok(Config::default())
    } else {
        match serde_yaml::from_str::<Config>(&s) {
            Ok(v) => Result::Ok(v),
            Err(err) => Result::Err(Box::new(err))
        }
    }
}

fn write_config_file(cfg: Config) -> Result<(), Box<dyn Error>> {
    let mut file = File::options().create(true).write(true).truncate(true).open(get_config_path()?)?;
    let mut s = serde_yaml::to_string(&cfg)?;
    file.write_all(s.as_bytes())?;
    Result::Ok(())
}

fn invoke_pkill(signal: &str, pattern: &str) -> Result<(), Box<dyn Error>> {
    let mut proc = Command::new("pkill")
      .arg("-".to_owned() + signal)
      .arg("-f")
      .arg(pattern)
      .spawn()?;
    let exit = proc.wait()?;
    match exit.code().expect("pkill exited without an exit code (how did this happen)") {
        0 => Result::Ok(()),
        1 => Result::Err(Box::new(std::io::Error::from(ErrorKind::NotFound))),
        2 => panic!("Invalid pattern"),
        3 => panic!("pkill exited fatally"),
        _ => panic!("pkill gave unexpected exit code")
    }
}

fn subcommand_app(app_name: &str, patterns: &[String]) {
    let mut cfg = read_config_file().expect("Failed to read config file");
    if patterns.is_empty() {
        println!("Removing app {}", app_name);
        cfg.apps.remove(app_name);
    } else {
        println!("Updated filter for app {}", app_name);
        cfg.apps.insert(app_name.to_owned(), ConfigApp { patterns: patterns.to_vec() });
    }
    write_config_file(cfg).expect("Failed to write config file");
}

fn subcommand_freeze(app_name: &str) {
    println!("Freezing {}", app_name);
    let cfg = read_config_file().expect("Failed to read config file");
    let app = cfg.apps.get(app_name).expect("App not found");
    for pattern in &app.patterns {
        invoke_pkill("SIGSTOP", pattern.as_str()).expect("pkill failed")
    }
}

fn subcommand_thaw(app_name: &str) {
    println!("Unfreezing {}", app_name);
    let cfg = read_config_file().expect("Failed to read config file");
    let app = cfg.apps.get(app_name).expect("App not found");
    for pattern in &app.patterns {
        invoke_pkill("SIGCONT", pattern.as_str()).expect("pkill failed")
    }
}

fn main() {
    let args = env::args().collect::<Vec<String>>();
    if args.len() < 2 {
        panic!("Provide a command");
    }
    if args[1] == "app" {
        subcommand_app(&args[2], &args[3..]);
    } else if args[1] == "freeze" {
        subcommand_freeze(&args[2]);
    } else if args[1] == "thaw" {
        subcommand_thaw(&args[2]);
    } else {
        panic!("Unknown command");
    }
}
