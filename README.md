# RemindMe

A simple bot for reminding a nostr user about events that happen. Interaction with the bot is done through replies on the event to receive the reminder. Simply add a reply to an event, including the bot in the message with a time. Below is an example:

```
nostr:npub16uy98al5gzdrhy4aztdyw385ddhkvhphdv66fglpg7dwxchennmsxcckxs in 5 minutes
```

Single timeframes supported by the [`humantime`](https://docs.rs/humantime/latest/humantime/) crate are supported by the bot.

## Development

The application is written in Rust. Therefore, [`rustup`](https://rustup.rs/) should be used to get Rust and friends installed on your machine.

Once installed, copy the `.env.example` file to `.env`. Also create the initial database file:

```shell
cp .env.example .env
touch remindme.db
```

Lastly, run the project using `cargo`, turning on debug logs for local development.

```shell
RUST_LOG=remindme=debug cargo run -p remindme
```

### Nix

There is a `flake.nix` file that can be used for an easy local development setup with the needed tools. Use `nix develop` if `direnv` isn't setup. If you are using `direnv`, run `direnv allow` to get the environment tools installed locally.

## Support

PRs are more than welcome! I don't know how much more needs to be added, but I'm open to ideas.

Feeling generous? Leave me a tip! ⚡️w3irdrobot@vlt.ge.

Think I'm an asshole but still want to tip? Please donate [to OpenSats](https://opensats.org/).

Want to tell me how you feel? Hit me up [on Nostr](https://njump.me/rob@w3ird.tech).

## License

Distributed under the AGPLv3 License. See LICENSE.txt for more information.
