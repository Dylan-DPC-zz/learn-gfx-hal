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

If you want RLS to play nice with the various optional features you must tell it
which one to use for its compilations. If you're using VS Code with the RLS
plugin, instead of messing up your `Cargo.toml` by specifying a default feature
you can instead make a `.vscode/settings.json` file in your project folder and
then place a setting for the feature you want it to use for RLS runs. Something
like this:

```json
{
  "rust.features": [
    "dx12"
  ]
}
```

If you're using RLS with some editor besides VS Code I'm afraid I don't know the
details of how you tell it to use a particular feature, but you probably can.
Consult your plugin docs, and such.

Over inside our main file we put some conditional stuff at the top:

```rust
#[cfg(feature = "dx12")]
extern crate gfx_backend_dx12 as back;
#[cfg(feature = "metal")]
extern crate gfx_backend_metal as back;
#[cfg(feature = "vulkan")]
extern crate gfx_backend_vulkan as back;
```

Yes, in the 2018 edition it's not _strictly necessary_ to have `extern crate`
any more, but this way we can alias whichever backend we pick to just be `back`.

Finally, before we go on, I'll mention that there _are_ other backend options
that we haven't considered:

* [gfx-backend-empty](https://crates.io/crates/gfx-backend-empty) does nothing
  but provide the required implementations as empty structs and do-nothing
  methods and so on. It's mostly used in the rustdoc examples for `gfx-hal`, and
  you might also use this with RLS or something, but you can't actually draw a
  picture or compute anything with it.
* [gfx-backend-gl](https://crates.io/crates/gfx-backend-gl) lets you target
  OpenGL 2.1+ and OpenGL ES2+. You'd probably use this if you wanted to run in a
  webpage, or perhaps on a Raspberry Pi (which has OpenGL ES2 drivers, but not
  Vulkan), or something like that where you couldn't pick one of the main
  options. Unfortunately, the GL backend is actually a little busted at the
  moment. The biggest snag is that webpages and desktop apps have rather
  different control flow, so it's hard to come up with a unified API. Work is
  being done, and hopefully soon I'll be able to recommend the GL backend.

## Allow For Logging

Since we're already mucking about with extra dependencies and stuff we'll also
take the time to add logging ability to our program.

In Rust you use the [log](https://docs.rs/log) crate as the generic logging
facade. It provides macros for each log level and you call them just like you'd
call `println!`. Then a particular logging backend (some other crate) picks up
those logging calls and does the actual logging into a file or over the network
or however. The simplest logging backend to use is probably
[env_logger](https://docs.rs/env_logger) since it just spits things to `stdout`
and `stderr` instead of needing to setup log files. That's fine for a tutorial,
so we'll do that. We just add a bit more to our `Cargo.toml`:

```toml
[dependencies]
log = "0.4.0"
env_logger = "0.5.12"
winit = "0.18"
gfx-hal = "0.1"
```

And then we turn on the `env_logger` in main before we do anything else:

```rust
fn main() {
  env_logger::init();
  // ...
```

And we'll see anything that someone wanted to log.

## Create an Instance

With our dependencies all set, the very first thing we do is create an
[Instance](https://docs.rs/gfx-hal/0.1.0/gfx_hal/trait.Instance.html). This does
whatever minimal things are required to activate your backend API. It's quite
simple. Every backend provides a type called `Instance` that implements the
`Instance` trait, and also they have a method called `create` which you pass a
`&str` (your instance name) and `u32` (your version). The `create` method
_isn't_ part of the Instance trait itself (because of that evil GL backend!),
though hopefully in a future version that can get squared away. We're working
with a 0.1 library after all.

```rust
let instance = back::Instance::create(WINDOW_NAME, 1);
```

Creating the instance does the _minimal_ setup to get the backend started, but
there's a whole lot more to go.

## Create a Surface

Once our Instance is started, we want to make a
[Surface](https://docs.rs/gfx-hal/0.1.0/gfx_hal/window/trait.Surface.html). This
is the part where `winit` and `gfx-hal` touch each other just enough for them to
communicate.

```rust
  let surface = instance.create_surface(&winit_state.window);
```

The `create_surface` call is another of those methods that's part of the
Instance _types_ that each backend just happens to agree to have, rather than
being on the Instance _trait_ itself. You just pass in a `&Window` and it does
the right thing.

## Create an Adapter

Next we need an
[Adapter](https://docs.rs/gfx-hal/0.1.0/gfx_hal/adapter/struct.Adapter.html),
which represents the graphics card you'll be using. A given Instance might have
more than one available, so we call
[enumerate_adapters](https://docs.rs/gfx-hal/0.1.0/gfx_hal/trait.Instance.html#tymethod.enumerate_adapters)
on our Instance to get the list of what's available. How do we decide what to
use? Well, you might come up with any criteria you like. The biggest thing you
probably care about is if the Adapter can do graphics work and/or computation
work. For now we just want one that can do graphics work.

Each Adapter has a `Vec<B::QueueFamily>`, and a
[QueueFamily](https://docs.rs/gfx-hal/0.1.0/gfx_hal/queue/family/trait.QueueFamily.html)
has methods to check if that QueueFamily supports graphics, compute, and/or
transfer. If a QueueFamily supports graphics or compute it will always also
support transfer (otherwise you wouldn't be able to send it things to draw and
compute), but some QueueFamily could theoretically support _just_ transfer and
nothing else. Also, each QueueFamily has a maximum number of queues that's
available, and we obviously need to have more than 0 queues available for it to
be acceptable. Finally, we obviously need to make sure that our Surface supports
the QueueFamily we're selecting.

So we have a `Vec<Adapter<Self::Backend>>` and each of those holds a
`Vec<B::QueueFamily>`, sounds like it's time for some Iterator magic.

```rust
  let adapter = instance
    .enumerate_adapters()
    .into_iter()
    .filter(|a| {
      a.queue_families
        .iter()
        .find(|qf| qf.supports_graphics() && qf.max_queues() > 0 && surface.supports_queue_family(qf))
        .is_some()
    })
    .next()
    .expect("Couldn't find a graphical Adapter!");
```

## Open up a Device

Okay so once we have an Adapter selected, we have to actually call
[open](https://docs.rs/gfx-hal/0.1.0/gfx_hal/adapter/trait.PhysicalDevice.html#tymethod.open)
on the associated PhysicalDevice to start using it. Think of this as the
difference between knowing the IP address you want to connect to and actually
opening the TCP socket that goes there.

Look, they even have a sample call to make. We need to specify a reference to a
slice of QueueFamily and QueuePriority tuple pairs. Well we know how to get a
QueueFamily we want, we just did that. A
[QueuePriority](https://docs.rs/gfx-hal/0.1.0/gfx_hal/adapter/type.QueuePriority.html)
is apparently just a 0.0 to 1.0 float for how high of priority we want. They use
1.0 in their example, so that seems fine to me.

Calling `open` gives us a Result, but we don't really know what to do if there's
a failure, so we'll just `expect` on that with a message like we have with other
things so far. This gives us a
[Gpu](https://docs.rs/gfx-hal/0.1.0/gfx_hal/struct.Gpu.html), which just bundles
up a Device and some Queues. The Queues value lets us call
[take](https://docs.rs/gfx-hal/0.1.0/gfx_hal/queue/family/struct.Queues.html#method.take)
to try and get out a particular QueueGroup by a specified id value. A QueueGroup
is just a vector of CommandQueue values with some metadata. We call `take` with
the id value of the QueueFamily we've been working with and hopefully get a
QueueGroup out. There's technically another Option layer we have to `expect`
away, but we're used to that by now I think. Once we have a QueueGroup, we can
get that vector of CommandQueue values and call it a day. Doesn't hurt much to
throw in a `debug_assert!` that we've really got at least one `CommandQueue`
available. We always _should_, because of the `filter` on the queue_families
that we did, but re-checking things you think are probably already true is the
whole point of a debug_assert after all.

```rust
  let (device, command_queues) = {
    let queue_family = adapter
      .queue_families
      .iter()
      .find(|qf| qf.supports_graphics() && qf.max_queues() > 0 && surface.supports_queue_family(qf))
      .expect("Couldn't find a QueueFamily with graphics!");
    let Gpu { device, mut queues } = unsafe {
      adapter
        .physical_device
        .open(&[(&queue_family, &[1.0; 1])])
        .expect("Couldn't open the PhysicalDevice!")
    };
    let queue_group = queues
      .take::<Graphics>(queue_family.id())
      .expect("Couldn't take ownership of the QueueGroup!");
    debug_assert!(queue_group.queues.len() > 0);
    (device, queue_group.queues)
  };
```

## Create a SwapChain

TODO!
