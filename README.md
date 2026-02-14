# pellx_monitor

Monitor and error-reporter of a PellX pellets burner.

Intended to be run on a Raspberry Pi connected via GPIO to terminal 1 and 2 on the controller board of a PellX burner. The connection between said terminals is closed when the machine is not in an error state, and opens on error and on power failure.

Notifications are sent as short emails via [Batsign](https://batsign.me).

## tl;dr

```
Usage: pellx_monitor [OPTIONS]

Options:
  -p, --pin-number <PIN_NUMBER>
          Raspberry Pi GPIO pin number to monitor
  -i, --poll-interval <POLL_INTERVAL>
          Poll interval between GPIO pin reads
  -H, --hold <HOLD>
          Duration the pin must be HIGH or LOW before qualifying as a valid change
  -t, --time-between-batsigns <TIME_BETWEEN_BATSIGNS>
          Minimum time between sending notification mails
  -r, --time-between-batsigns-retry <TIME_BETWEEN_BATSIGNS_RETRY>
          Time to wait before retrying to send a notification mail after a failure
  -u, --batsign-url <BATSIGN_URL>
          Batsign URL to send the alert to (REQUIRED)
      --dry-run
          Perform a dry run without sending any mails
      --debug
          Print additional debug information
  -c, --config <CONFIG>
          Specify an alternate configuration file
      --save
          Write the resolved configuration to disk
```

## license

This project is licensed under the **Boost Software License 1.0** - see the [LICENSE](LICENSE) file for details.
