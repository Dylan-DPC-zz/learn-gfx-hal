# INCOMPLETE

# TODO

# WORK IN PROGRESS

# Drawing A Triangle

This part will be the most difficult part out of all of them.

Unfortunately, for you, and I, and for everyone else, `gfx-hal` has a rather
intense upfront word cost. Adding more as you go isn't pricey, but that first
triangle covers a whole lot of ground at once. This is a land of maximum
control, and unfortunately we can't just say "gimme a good default setup and
I'll change the settings later". By the end of this lesson we'll have expanded
our code from ~40 lines to ~700 lines.

## Adding in `gfx-hal` and a backend

First, we have to add the `gfx-hal` crate to our `Cargo.toml` file. We also need
to pick a backend crate. Remember that the "hal" in `gfx-hal` is for "Hardware
Abstraction Layer". So `gfx-hal` just provides the general types and operations,
then each backend actually implements the details according to the hardware API
it's abstracting over.

Since we want it to be something you can pick per-compile, we're going to use a
big pile of features and optional dependencies:

```toml
[features]
default = []
metal = ["gfx-backend-metal"]
dx12 = ["gfx-backend-dx12"]
vulkan = ["gfx-backend-vulkan"]

[dependencies]
winit = "0.18"
gfx-hal = "0.1"

[dependencies.gfx-backend-vulkan]
version = "0.1"
optional = true

[target.'cfg(target_os = "macos")'.dependencies.gfx-backend-metal]
version = "0.1"
optional = true

[target.'cfg(windows)'.dependencies.gfx-backend-dx12]
version = "0.1"
optional = true
```

If you want RLS to play nice with the various optional features you can define
one of them as default. If you're using VS Code with the RLS plugin, instead of
messing up your `Cargo.toml` you can make a `.vscode/settings.json` file in your
project folder and then place a setting for the feature you want it to pick as
default when RLS runs:

```json
{
  "rust.features": [
    "dx12"
  ]
}
```

If you've got other editors using RLS then they can probably be configured
similarly, but I don't know the details there.

Over inside our main file we put some conditional stuff at the top:

```rust
#[cfg(feature = "dx12")]
extern crate gfx_backend_dx12 as back;
#[cfg(feature = "metal")]
extern crate gfx_backend_metal as back;
#[cfg(feature = "vulkan")]
extern crate gfx_backend_vulkan as back;
extern crate gfx_hal as hal;
```

Yes, in the 2018 edition it's not _strictly necessary_ to have `extern crate`
any more, but making the shorthand alias names is very nice, so we'll do it.

## Create an Instance
