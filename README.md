# Prometheus Exporter for Homebridge
A simple exporter for Prometheus that reads information about all your devices and exports all values as Prometheus metrics.

## Usage
`prometheus-exporter` is a command line program that takes few parameters:

```text
USAGE:
    homebridge-exporter [OPTIONS] --username <USERNAME> --password <PASSWORD>

OPTIONS:
        --debug                  Debug mode
    -h, --help                   Print help information
    -p, --password <PASSWORD>    Homebridge password
        --port <PORT>            Metrics webserver port (service /metrics for Prometheus scraper)
                                 [default: 9123]
        --prefix <PREFIX>        Registry metrics prefix [default: homebrige]
    -u, --username <USERNAME>    Homebridge username
        --uri <URI>              Homebridge UI uri [default: http://localhost:8581]
    -V, --version                Print version information
```
This software scrapes all the accessories from homebridge APIs and creates prometheus metrics out of all services information. 
It also exposes an additional endpoint that you can use to restart the homebridge server in case you detect some problem with metric values (for example from Prometheus AlertManager).
The endpoint is `/restart` and must be used sending a POST request that contains an Authorization header containing a bearer token you should generate and put inside the file `authorization-keys.yaml`.
An example of the content fo this file is this:

```yaml
keys:
  - 1d3f962f-bdcf-4f08-85ce-3e109f4e8f62
```
You can then use this CURL to trigger your Homebridge instance to restart:

```shell
curl -X POST -H "Authorization: Bearer 1d3f962f-bdcf-4f08-85ce-3e109f4e8f62" http://YOUR_IP:9123/restart
```

## Build from source
This project is written in Rust and uses Cargo to compile and link the final executable.
To be able to build the final executable, you need first to install Rust, following the instructions [here](https://www.rust-lang.org/tools/install).

Once your environment is ready, you can build the project by running the command:

    cargo build

To build in release more, just add `-r` to the above command.
If you want to build only a specific target, specify it using the `--target` parameter:

    cargo build --target x86_64-apple-darwin

Supported targets are defined in the [cargo config file](blob/master/.cargo/config.toml).

