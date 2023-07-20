# dwmbar
Simple status bar for [dwm](https://dwm.suckless.org/)

## Installation
```shell
cargo install dwmbar
```

## Configuration
The config file is placed at `$XDG_CONFIG_HOME/dwmbar/config.json` or simply `~/.config/dwmbar/config.json`
The default should give you an idea of how it is structured. This is a basic command:

```json
...
{
  "command": "pwd",
  "update_delay": 5000, // Update delay, in ms (optional)
  "ignore_status_code": true, // Take the output even if the status code indicates a failure. Default: false (optional)
},
...
```

Each command is seperated by the delimiter specified at the top of the file.  
`thread_polling_delay` refers to the delay after checking if any commands are done. `1/0.thread_polling_delay` is effectively the refresh rate at which the status bar is updated