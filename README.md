# pellx_monitor

Monitor and error-reporter of a PellX pellets burner.

Intended to be run on a Raspberry Pi connected via GPIO to terminal 1 and 2 on the controller board of a PellX burner. The connection between said terminals is closed when the machine is running normally and opens when it is in an error state (including power failures).

Notifications can be sent as [Slack messages](https://api.slack.com/apps?new_app=1) (via [webhook URLs](https://docs.slack.dev/messaging/sending-messages-using-incoming-webhooks)) and/or as short emails via [Batsign](https://batsign.me).

## tl;dr

```
Usage: pellx_monitor [OPTIONS]

Options:
  -c, --config-dir <path>  Specify an alternate configuration directory
      --show               Show the resolved configuration and exit
  -d, --debug              Print additional debug information
      --dry-run            Perform a dry run without sending any notifications
      --save               Write configuration to disk
  -V, --version            Display version information and exit
  -h, --help               Print help
```

## license

This project is licensed under the **Boost Software License 1.0** - see the [LICENSE](LICENSE) file for details.
