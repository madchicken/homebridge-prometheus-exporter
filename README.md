<p style="display: flex; align-items: center; align-content: center">
  <a href="https://homebridge.io"><img src="https://raw.githubusercontent.com/homebridge/branding/master/logos/homebridge-color-round-stylized.png" height="140"></a>
  <a href="//prometheus.io" target="_blank"><img alt="Prometheus" src="https://raw.githubusercontent.com/prometheus/prometheus/main/documentation/images/prometheus-logo.svg"></a><br>
</p>

# Prometheus Exporter for Homebridge
A simple exporter for Prometheus that reads information about all your devices and exports all values as Prometheus metrics.

## Usage
`prometheus-exporter` is a command line program that takes few parameters:

```text
USAGE:
    homebridge-exporter [OPTIONS] --username <USERNAME> --password <PASSWORD>

OPTIONS:
    -h, --help                   Print help information
    -p, --password <PASSWORD>    Homebridge password
        --port <PORT>            Metrics webserver port (service /metrics for Prometheus scraper)
                                 [default: 8001]
        --prefix <PREFIX>        Registry metrics prefix [default: homebrige]
    -u, --username <USERNAME>    Homebridge username
        --uri <URI>              Homebridge UI uri [default: http://localhost:8581]
    -V, --version                Print version information
```

