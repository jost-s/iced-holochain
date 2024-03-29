# HoloMessages

A simple messaging Holochain application based on [iced](https://github.com/iced-rs/iced) - a Rust GUI library - as an example of a Rust GUI hApp.

## Running the application

### Clone & run
After cloning the repository, enter the nix shell to make the `hc` command available:

```sh
nix develop --run $SHELL
```

Then the hApp can be built:

```sh
./build-happ.sh
```

Now the app can be started:

```sh
cargo run
```

A new conductor will be created under the path `./holochain/`. It will be reused for every subsequent application start as long as it can be found under that path.

### Known bug

`holochain_websocket` shuts down when a signal is emitted in a `post_commit` hook, as is the case in the "profiles" zome that is part of this hApp. Therefore after creating a new profile, the app will stop working. A restart is required and as there is an existing profile to restore now, the websocket will not shut down.

### Multiple conductors

To start the app with a different conductor, a folder name can be passed in as a parameter:

```sh
cargo run -- tmp1
```

This will create a conductor under `./holochain/tmp1/`.

As a way to run the app with multiple conductors, it serves to start up different instances with separate conductors and interact with each other from different application windows.

E. g. to send messages between two agents,

```sh
cargo run -- agent1
```

will create a conductor under `./holochain/agent1/`. From another terminal calling

```sh
cargo run -- agent2
```

creates a second conductor at `./holochain/agent2/`. Both conductors share the same network and the agents can see each others' messages.

### Delete conductors

If you no longer want to use an existing conductor, you can delete its folder under `./holochain`. To delete all existing conductors including the default one, delete `./holochain` entirely.

### Network seed

The network seed can be defined in `./happ/workdir/dna.yaml`. It determines if agents running this hApp will be on the same network and see the same data. When you want to run multiple conductors to exchange messages, they need to have the same DNA based on a matching network seed installed.

## Tests

Tests can be run with the command

```sh
cargo test --all
```