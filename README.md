# pellx_monitor

Monitor and error-reporter of a PellX pellets burner.

Intended to be run on a Raspberry Pi connected via GPIO to terminal 1 and 2 on the controller board of a PellX burner. The connection between said terminals is closed when the machine is running normally and opens when it is in an error state (including power failures).

Notifications are sent as short emails via [Batsign](https://batsign.me).

## tl;dr

```
Usage: pellx_monitor [OPTIONS]

Options:
  -p, --pin-number <pin>
          Raspberry Pi GPIO pin number to monitor
  -i, --poll-interval <duration>
          Poll interval between GPIO pin reads
  -H, --hold <duration>
          Duration the pin must be HIGH or LOW before qualifying as a valid change
  -t, --time-between-batsigns <duration>
          Minimum time between sending notifications
  -r, --time-between-batsigns-retry <duration>
          Time to wait before retrying to send a notification after a failure
  -u, --batsign-url <url>
          Batsign URL to send alerts to (REQUIRED)
      --dry-run
          Perform a dry run without sending any notifications
      --debug
          Print additional debug information
      --show
          Show the resolved configuration and exit
  -c, --config <path to file>
          Specify an alternate configuration file
      --save
          Write the resolved configuration to disk
```

## license

This project is licensed under the **Boost Software License 1.0** - see the [LICENSE](LICENSE) file for details.
