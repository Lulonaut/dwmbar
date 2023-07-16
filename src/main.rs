#![feature(default_free_fn)]

use serde::{Deserialize, Serialize};
use std::default::default;
use std::ffi::CString;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::{exit, Command, Output};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;
use x11::xlib::{XDefaultRootWindow, XFlush, XOpenDisplay, XStoreName};

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ActiveCommand {
    command: String,

    #[serde(skip_serializing, skip_deserializing)]
    output: Arc<RwLock<String>>,
    update_delay: Option<u64>,

    ignore_status_code: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Config {
    default_update_delay: u64,
    thread_polling_delay: u64,
    delimiter: String,
    commands: Vec<ActiveCommand>,
}

impl Default for ActiveCommand {
    fn default() -> Self {
        ActiveCommand {
            command: String::new(),
            output: Arc::new(RwLock::new(String::new())),
            update_delay: None,
            ignore_status_code: None,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            delimiter: " ".to_string(),
            default_update_delay: 990,
            thread_polling_delay: 500,
            commands: vec![
                ActiveCommand {
                    command: "date".to_string(),
                    ..default()
                },
                ActiveCommand {
                    command: "echo \"The bar is working\"".to_string(),
                    update_delay: Some(0),
                    ..default()
                },
            ],
        }
    }
}

fn run_command(command: &str) -> Output {
    Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .expect("Failed to execute command")
}

fn read_config() -> Result<Config, anyhow::Error> {
    let file = PathBuf::from("config.json");
    if !file.exists() {
        print!("Config file does not exist yet, creating...");
        let mut file = File::create(file)?;

        // Write the default config
        file.write_all(
            serde_json::to_string_pretty(&Config::default())
                .unwrap()
                .as_bytes(),
        )?;
        println!("OK");
    }

    let contents = std::fs::read_to_string("config.json")?;
    let config = serde_json::from_str(&contents)?;
    Ok(config)
}

fn main() -> Result<(), anyhow::Error> {
    let config = read_config()?;
    let collected_commands = config.commands;

    let (tx, rx) = channel();

    // Spawn initial commands
    for command in collected_commands.clone() {
        let tx = tx.clone();
        thread::spawn(move || {
            let output = run_command(command.command.as_str());

            command.output.write().unwrap().clear();
            command
                .output
                .write()
                .unwrap()
                .push_str(&String::from_utf8(output.stdout).unwrap());
            tx.send(command).unwrap();
        });
    }

    let commands = Arc::new(Mutex::new(collected_commands));

    let display = unsafe { XOpenDisplay(std::ptr::null()) };

    if display.is_null() {
        println!("Failed to open X display");
        exit(1)
    }

    loop {
        // Respawn all commands need to be respawned
        for command in rx.try_iter() {
            let tx = tx.clone();
            thread::spawn(move || {
                // Wait the configured delay before spawning the command again or quit when the delay is 0
                let sleep_delay = Duration::from_millis(
                    command.update_delay.unwrap_or(config.default_update_delay),
                );

                // No updates
                if sleep_delay.is_zero() {
                    return;
                }
                thread::sleep(sleep_delay);

                let output = run_command(command.command.as_str());
                if !output.status.success()
                    && command.ignore_status_code.is_some()
                    && !command.ignore_status_code.unwrap()
                {
                    eprintln!("Command {} was not successful!", command.command);
                    tx.send(command).unwrap();
                    return;
                }

                // Clear the currently saved output and replace it with the new one
                command.output.write().unwrap().clear();
                command
                    .output
                    .write()
                    .unwrap()
                    .push_str(&String::from_utf8(output.stdout).unwrap());

                // Send the command object in the channel to be received by the main loop again
                tx.send(command).unwrap();
            });
        }

        // Assemble the status line
        let mut current_output = String::new();
        for command in commands.lock().unwrap().iter() {
            let output_lock = command.output.read().unwrap();
            let mut output = (*output_lock).clone();

            // Only include the first line
            if output.contains("\n") {
                if let Some(split) = output.split_once("\n") {
                    output = split.0.to_string();
                }
            }

            current_output.push_str(&output);
            current_output.push_str(&config.delimiter);
        }

        let root_window_name = CString::new(current_output).unwrap();
        // set the new state
        unsafe {
            XStoreName(
                display,
                XDefaultRootWindow(display),
                root_window_name.as_ptr(),
            );
            XFlush(display);
        }

        thread::sleep(Duration::from_millis(config.thread_polling_delay))
    }
}
