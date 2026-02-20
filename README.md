# pellx_monitor

Monitor and error-reporter of a **PellX pellets burner**.

Intended to be run on a **Raspberry Pi-equivalent** device connected via GPIO to terminals on the controller board of a PellX burner. Terminals **1** and **2** are electrically connected when the burner is operating normally, and the circuit is broken when it is in an error state (including on power failures).

A notification is sent when this is detected. They can be sent as [**Slack** messages](https://api.slack.com/apps?new_app=1) (via [webhook URLs](https://docs.slack.dev/messaging/sending-messages-using-incoming-webhooks)) and/or as short emails via [**Batsign**](https://batsign.me).

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

Use `--save` to create a directory with configuration and resource files. Edit the `config.toml` inside it to get started.

## cross-compilation

Depending on the type of device you intend to run it on, compilation memory required may be a limiting factor and cross-compilation on a more competent machine may be required. For instance, a **Raspberry Pi Zero 2W** has only 512 megabytes of RAM, which is insufficient to comfortably build this project.

```
cargo build --target=aarch64-unknown-linux-gnu
```

## todo

* external command as notification methods
* implement notification methods like `Box<dyn Notifier>`
* better documentation
* more unit tests
* review all textual output
* colored terminal output?

## license

This project is dual-licensed under the [**MIT License**](LICENSE-MIT) and the [**Apache License (Version 2.0)**](LICENSE-APACHE).
