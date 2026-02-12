# pellx_monitor

Monitor and error-reporter of PellX pellets burner.

Intended to be run on a Raspberry Pi connected via GPIO to terminal 1 and 2 on the controller board of a PellX burner. The connection between said terminals is closed when the machine is not in an error state, and opens on error and on power failure.

Notifications are sent as short emails via [Batsign](https://batsign.me).

## tl;dr

```
Usage: pellx_monitor [OPTIONS]

Options:
  -p, --pin-number <PIN_NUMBER>
          GPIO pin number to monitor
  -i, --poll-interval <POLL_INTERVAL>
          Poll interval for checking the GPIO pin
  -q, --qualify-high <QUALIFY_HIGH>
          Duration the pin must be HIGH before qualifying as an alarm
  -t, --time-between-mails <TIME_BETWEEN_MAILS>
          Minimum time between sending mails
  -r, --time-between-mails-retry <TIME_BETWEEN_MAILS_RETRY>
          Time to wait before retrying to send a mail after a failure
  -u, --batsign-url <BATSIGN_URL>
          Batsign URL to send the alert to (REQUIRED)
  -s, --batsign-subject <BATSIGN_SUBJECT>
          Subject line for the Batsign message (REQUIRED)
  -c, --config <CONFIG>
          Override path to configuration file
      --save
          Write the resolved configuration to disk
  -h, --help
          Print help
  -V, --version
          Print version
```

## license

This project is licensed under the **Boost Software License 1.0** - see the [LICENSE](LICENSE) file for details.
