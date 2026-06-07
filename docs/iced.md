# iced API (0.14.0)

A cross-platform GUI library inspired by Elm

## 1: Manifest

- Homepage: <https://iced.rs>
- Repository: <https://github.com/iced-rs/iced>
- Categories: gui
- License: MIT
- rust-version: `1.88`
- edition: `2024`

### 1.1: Features

- `advanced`
- `advanced-shaping`
- `basic-shaping`
- `canvas`
- `crisp`
- `debug`
- `default`
- `fira-sans`
- `highlighter`
- `hot`
- `image`
- `image-without-codecs`
- `lazy`
- `linux-theme-detection`
- `markdown`
- `qr_code`
- `selector`
- `sipper`
- `smol`
- `strict-assertions`
- `svg`
- `sysinfo`
- `tester`
- `thread-pool`
- `time-travel`
- `tiny-skia`
- `tokio`
- `unconditional-rendering`
- `wayland`
- `web-colors`
- `webgl`
- `wgpu`
- `wgpu-bare`
- `x11`


## 2: README

<div align="center">

<img src="docs/logo.svg" width="140px" />

### Iced



A cross-platform GUI library for Rust focused on simplicity and type-safety.
Inspired by [Elm].

<a href="https://github.com/squidowl/halloy">
  <img src="https://iced.rs/showcase/halloy.gif" width="460px">
</a>
<a href="https://github.com/hecrj/icebreaker">
  <img src="https://iced.rs/showcase/icebreaker.gif" width="360px">
</a>

</div>

#### Features

* Simple, easy-to-use, batteries-included API
* Type-safe, reactive programming model
* [Cross-platform support] (Windows, macOS, Linux, and the Web)
* Responsive layout
* Built-in widgets (including [text inputs], [scrollables], and more!)
* Custom widget support (create your own!)
* [Debug tooling with performance metrics and time traveling]
* First-class support for async actions (use futures!)
* Modular ecosystem split into reusable parts:
  * A [renderer-agnostic native runtime] enabling integration with existing systems
  * Two built-in renderers leveraging [`wgpu`] and [`tiny-skia`]
    * [`iced_wgpu`] supporting Vulkan, Metal and DX12
    * [`iced_tiny_skia`] offering a software alternative as a fallback
  * A [windowing shell]

**Iced is currently experimental software.** [Take a look at the roadmap] and
[check out the issues].

#### Overview

Inspired by [The Elm Architecture], Iced expects you to split user interfaces
into four different concepts:

* **State** — the state of your application
* **Messages** — user interactions or meaningful events that you care
  about
* **View logic** — a way to display your **state** as widgets that
  may produce **messages** on user interaction
* **Update logic** — a way to react to **messages** and update your
  **state**

We can build something to see how this works! Let's say we want a simple counter
that can be incremented and decremented using two buttons.

We start by modelling the **state** of our application:

````rust
#[derive(Default)]
struct Counter {
    value: i32,
}
````

Next, we need to define the possible user interactions of our counter:
the button presses. These interactions are our **messages**:

````rust
#[derive(Debug, Clone, Copy)]
pub enum Message {
    Increment,
    Decrement,
}
````

Now, let's show the actual counter by putting it all together in our
**view logic**:

````rust
use iced::widget::{button, column, text, Column};

impl Counter {
    pub fn view(&self) -> Column<Message> {
        // We use a column: a simple vertical layout
        column![
            // The increment button. We tell it to produce an
            // `Increment` message when pressed
            button("+").on_press(Message::Increment),

            // We show the value of the counter here
            text(self.value).size(50),

            // The decrement button. We tell it to produce a
            // `Decrement` message when pressed
            button("-").on_press(Message::Decrement),
        ]
    }
}
````

Finally, we need to be able to react to any produced **messages** and change our
**state** accordingly in our **update logic**:

````rust
impl Counter {
    // ...

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Increment => {
                self.value += 1;
            }
            Message::Decrement => {
                self.value -= 1;
            }
        }
    }
}
````

And that's everything! We just wrote a whole user interface. Let's run it:

````rust
fn main() -> iced::Result {
    iced::run(Counter::update, Counter::view)
}
````

Iced will automatically:

1. Take the result of our **view logic** and layout its widgets.
1. Process events from our system and produce **messages** for our
   **update logic**.
1. Draw the resulting user interface.

Read the [book], the [documentation], and the [examples] to learn more!

#### Implementation details

Iced was originally born as an attempt at bringing the simplicity of [Elm] and
[The Elm Architecture] into [Coffee], a 2D game library I am working on.

The core of the library was implemented during May 2019 in [this pull request].
[The first alpha version] was eventually released as
[a renderer-agnostic GUI library]. The library did not provide a renderer and
implemented the current [tour example] on top of [`ggez`], a game library.

Since then, the focus has shifted towards providing a batteries-included,
end-user-oriented GUI library, while keeping the ecosystem modular.

#### Contributing / Feedback

If you want to contribute, please read our [contributing guidelines] for more details.

Feedback is also welcome! You can create a new topic in [our Discourse forum] or
come chat to [our Discord server].

#### Sponsors

The development of Iced is sponsored by the [Cryptowatch] team at [Kraken.com]

[documentation]: https://docs.rs/iced/
[Elm]: https://elm-lang.org/
[Cross-platform support]: https://raw.githubusercontent.com/iced-rs/iced/master/docs/images/todos_desktop.jpg
[text inputs]: https://iced.rs/examples/text_input.mp4
[scrollables]: https://iced.rs/examples/scrollable.mp4
[Debug tooling with performance metrics and time traveling]: https://github.com/user-attachments/assets/2e49695c-0261-4b43-ac2e-8d7da5454c4b
[renderer-agnostic native runtime]: runtime/
[`wgpu`]: https://github.com/gfx-rs/wgpu
[`tiny-skia`]: https://github.com/RazrFalcon/tiny-skia
[`iced_wgpu`]: wgpu/
[`iced_tiny_skia`]: tiny_skia/
[windowing shell]: winit/
[Take a look at the roadmap]: ROADMAP.md
[check out the issues]: https://github.com/iced-rs/iced/issues
[The Elm Architecture]: https://guide.elm-lang.org/architecture/
[book]: https://book.iced.rs/
[examples]: https://github.com/iced-rs/iced/tree/0.14/examples#examples
[Coffee]: https://github.com/hecrj/coffee
[this pull request]: https://github.com/hecrj/coffee/pull/35
[The first alpha version]: https://github.com/iced-rs/iced/tree/0.1.0-alpha
[a renderer-agnostic GUI library]: https://www.reddit.com/r/rust/comments/czzjnv/iced_a_rendereragnostic_gui_library_focused_on/
[tour example]: examples/README.md#tour
[`ggez`]: https://github.com/ggez/ggez
[contributing guidelines]: https://github.com/iced-rs/iced/blob/master/CONTRIBUTING.md
[our Discourse forum]: https://discourse.iced.rs/
[our Discord server]: https://discord.gg/3xZJ65GAhd
[Cryptowatch]: https://cryptowat.ch/charts
[Kraken.com]: https://kraken.com/


## 3: Common Traits

The following traits are commonly implemented by types in this crate. Unless otherwise noted, you can assume these traits are implemented:

- `Display`
- `error::Error`

- `iced_program::Program`

    ```rust
    impl<P: iced_program::Program> iced_program::Program for iced::daemon::Daemon<P> {
        type State = <P as iced_program::Program>::State;
        type Message = <P as iced_program::Program>::Message;
        type Theme = <P as iced_program::Program>::Theme;
        type Renderer = <P as iced_program::Program>::Renderer;
        type Executor = <P as iced_program::Program>::Executor;
    }
    ```

- `!RefUnwindSafe`
- `!UnwindSafe`
- `Freeze`
- `Unpin`

- `ToString` (`where T: Display + ?Sized`)
- `any::Any` (`where T: 'static + ?Sized`)
- `borrow::Borrow<T>` (`where T: ?Sized`)
- `borrow::BorrowMut<T>` (`where T: ?Sized`)
- `convert::Into<U>` (`where U: convert::From<T>`)
- `convert::TryFrom<U>` (`where U: convert::Into<T>`)
- `convert::TryInto<U>` (`where U: convert::TryFrom<T>`)
- `iced::application::IntoBoot<State, Message>`
- `iced_futures::maybe::platform::MaybeSend` (`where T: Send`)
- `iced_futures::maybe::platform::MaybeSync` (`where T: Sync`)
- `iced_program::message::MaybeClone`
- `iced_program::message::MaybeDebug`
- `smol_str::ToSmolStr` (`where T: Display + ?Sized`)
- `tracing::instrument::Instrument`
- `tracing::instrument::WithSubscriber`
- `wgpu_types::send_sync::WasmNotSendSync`

    ```rust
    where
        T: wgpu_types::send_sync::WasmNotSend + wgpu_types::send_sync::WasmNotSync
    ```
- `wgpu_types::send_sync::WasmNotSend` (`where T: Send`)
- `wgpu_types::send_sync::WasmNotSync` (`where T: Sync`)


## 4: Module: `iced`

iced is a cross-platform GUI library focused on simplicity and type-safety.
Inspired by [Elm].

#### Disclaimer

iced is **experimental** software. If you expect the documentation to hold your hand
as you learn the ropes, you are in for a frustrating experience.

The library leverages Rust to its full extent: ownership, borrowing, lifetimes, futures,
streams, first-class functions, trait bounds, closures, and more. This documentation
is not meant to teach you any of these. Far from it, it will assume you have **mastered**
all of them.

Furthermore—just like Rust—iced is very unforgiving. It will not let you easily cut corners.
The type signatures alone can be used to learn how to use most of the library.
Everything is connected.

Therefore, iced is easy to learn for **advanced** Rust programmers; but plenty of patient
beginners have learned it and had a good time with it. Since it leverages a lot of what
Rust has to offer in a type-safe way, it can be a great way to discover Rust itself.

If you don't like the sound of that, you expect to be spoonfed, or you feel frustrated
and struggle to use the library; then I recommend you to wait patiently until [the book]
is finished.

#### The Pocket Guide

Start by calling \[`run`\]:

````no_run,standalone_crate
pub fn main() -> iced::Result {
    iced::run(update, view)
}
# fn update(state: &mut (), message: ()) {}
# fn view(state: &()) -> iced::Element<'_, ()> { iced::widget::text("").into() }
````

Define an `update` function to **change** your state:

````standalone_crate
fn update(counter: &mut u64, message: Message) {
    match message {
        Message::Increment => *counter += 1,
    }
}
# #[derive(Clone)]
# enum Message { Increment }
````

Define a `view` function to **display** your state:

````standalone_crate
use iced::widget::{button, text};
use iced::Element;

fn view(counter: &u64) -> Element<'_, Message> {
    button(text(counter)).on_press(Message::Increment).into()
}
# #[derive(Clone)]
# enum Message { Increment }
````

And create a `Message` enum to **connect** `view` and `update` together:

````standalone_crate
#[derive(Debug, Clone)]
enum Message {
    Increment,
}
````

##### Custom State

You can define your own struct for your state:

````standalone_crate
#[derive(Default)]
struct Counter {
    value: u64,
}
````

But you have to change `update` and `view` accordingly:

````standalone_crate
# struct Counter { value: u64 }
# #[derive(Clone)]
# enum Message { Increment }
# use iced::widget::{button, text};
# use iced::Element;
fn update(counter: &mut Counter, message: Message) {
    match message {
        Message::Increment => counter.value += 1,
    }
}

fn view(counter: &Counter) -> Element<'_, Message> {
    button(text(counter.value)).on_press(Message::Increment).into()
}
````

##### Widgets and Elements

The `view` function must return an \[`Element`\]. An \[`Element`\] is just a generic \[`widget`\].

The \[`widget`\] module contains a bunch of functions to help you build
and use widgets.

Widgets are configured using the builder pattern:

````standalone_crate
# struct Counter { value: u64 }
# #[derive(Clone)]
# enum Message { Increment }
use iced::widget::{button, column, text};
use iced::Element;

fn view(counter: &Counter) -> Element<'_, Message> {
    column![
        text(counter.value).size(20),
        button("Increment").on_press(Message::Increment),
    ]
    .spacing(10)
    .into()
}
````

A widget can be turned into an \[`Element`\] by calling `into`.

Widgets and elements are generic over the message type they produce. The
\[`Element`\] returned by `view` must have the same `Message` type as
your `update`.

##### Layout

There is no unified layout system in iced. Instead, each widget implements
its own layout strategy.

Building your layout will often consist in using a combination of
[rows], [columns], and [containers]:

````standalone_crate
# struct State;
# enum Message {}
use iced::widget::{column, container, row};
use iced::{Fill, Element};

fn view(state: &State) -> Element<'_, Message> {
    container(
        column![
            "Top",
            row!["Left", "Right"].spacing(10),
            "Bottom"
        ]
        .spacing(10)
    )
    .padding(10)
    .center_x(Fill)
    .center_y(Fill)
    .into()
}
````

Rows and columns lay out their children horizontally and vertically,
respectively. [Spacing] can be easily added between elements.

Containers position or align a single widget inside their bounds.

##### Sizing

The width and height of widgets can generally be defined using a \[`Length`\].

* \[`Fill`\] will make the widget take all the available space in a given axis.
* \[`Shrink`\] will make the widget use its intrinsic size.

Most widgets use a \[`Shrink`\] sizing strategy by default, but will inherit
a \[`Fill`\] strategy from their children.

A fixed numeric \[`Length`\] in \[`Pixels`\] can also be used:

````standalone_crate
# struct State;
# enum Message {}
use iced::widget::container;
use iced::Element;

fn view(state: &State) -> Element<'_, Message> {
    container("I am 300px tall!").height(300).into()
}
````

##### Theming

The default \[`Theme`\] of an application can be changed by defining a `theme`
function and leveraging the \[`Application`\] builder, instead of directly
calling \[`run`\]:

````no_run,standalone_crate
# struct State;
use iced::Theme;

pub fn main() -> iced::Result {
    iced::application(new, update, view)
        .theme(theme)
        .run()
}

fn new() -> State {
    // ...
    # State
}

fn theme(state: &State) -> Theme {
    Theme::TokyoNight
}
# fn update(state: &mut State, message: ()) {}
# fn view(state: &State) -> iced::Element<'_, ()> { iced::widget::text("").into() }
````

The `theme` function takes the current state of the application, allowing the
returned \[`Theme`\] to be completely dynamic—just like `view`.

There are a bunch of built-in \[`Theme`\] variants at your disposal, but you can
also [create your own](Theme::custom).

##### Styling

As with layout, iced does not have a unified styling system. However, all
of the built-in widgets follow the same styling approach.

The appearance of a widget can be changed by calling its `style` method:

````standalone_crate
# struct State;
# enum Message {}
use iced::widget::container;
use iced::Element;

fn view(state: &State) -> Element<'_, Message> {
    container("I am a rounded box!").style(container::rounded_box).into()
}
````

The `style` method of a widget takes a closure that, given the current active
\[`Theme`\], returns the widget style:

````standalone_crate
# struct State;
# #[derive(Clone)]
# enum Message {}
use iced::widget::button;
use iced::{Element, Theme};

fn view(state: &State) -> Element<'_, Message> {
    button("I am a styled button!").style(|theme: &Theme, status| {
        let palette = theme.extended_palette();

        match status {
            button::Status::Active => {
                button::Style::default()
                   .with_background(palette.success.strong.color)
            }
            _ => button::primary(theme, status),
        }
    })
    .into()
}
````

Widgets that can be in multiple different states will also provide the closure
with some [`Status`], allowing you to use a different style for each state.

You can extract the [`Palette`] colors of a \[`Theme`\] with the [`palette`] or
[`extended_palette`] methods.

Most widgets provide styling functions for your convenience in their respective modules;
like [`container::rounded_box`], [`button::primary`], or [`text::danger`].

##### Concurrent Tasks

The `update` function can *optionally* return a \[`Task`\].

A \[`Task`\] can be leveraged to perform asynchronous work, like running a
future or a stream:

````standalone_crate
# #[derive(Clone)]
# struct Weather;
use iced::Task;

struct State {
    weather: Option<Weather>,
}

enum Message {
   FetchWeather,
   WeatherFetched(Weather),
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::FetchWeather => Task::perform(
            fetch_weather(),
            Message::WeatherFetched,
        ),
        Message::WeatherFetched(weather) => {
            state.weather = Some(weather);

            Task::none()
       }
    }
}

async fn fetch_weather() -> Weather {
    // ...
    # unimplemented!()
}
````

Tasks can also be used to interact with the iced runtime. Some modules
expose functions that create tasks for different purposes—like [changing
window settings](window#functions), [focusing a widget](widget::operation::focus_next), or
[querying its visible bounds](widget::selector::find).

Like futures and streams, tasks expose [a monadic interface](Task::then)—but they can also be
[mapped](Task::map), [chained](Task::chain), [batched](Task::batch), [canceled](Task::abortable),
and more.

##### Passive Subscriptions

Applications can subscribe to passive sources of data—like time ticks or runtime events.

You will need to define a `subscription` function and use the \[`Application`\] builder:

````no_run,standalone_crate
# struct State;
use iced::window;
use iced::{Size, Subscription};

#[derive(Debug, Clone)]
enum Message {
    WindowResized(Size),
}

pub fn main() -> iced::Result {
    iced::application(new, update, view)
        .subscription(subscription)
        .run()
}

fn subscription(state: &State) -> Subscription<Message> {
    window::resize_events().map(|(_id, size)| Message::WindowResized(size))
}
# fn new() -> State { State }
# fn update(state: &mut State, message: Message) {}
# fn view(state: &State) -> iced::Element<'_, Message> { iced::widget::text("").into() }
````

A \[`Subscription`\] is [a *declarative* builder of streams](Subscription#the-lifetime-of-a-subscription)
that are not allowed to end on their own. Only the `subscription` function
dictates the active subscriptions—just like `view` fully dictates the
visible widgets of your user interface, at every moment.

As with tasks, some modules expose convenient functions that build a \[`Subscription`\] for you—like
\[`time::every`\] which can be used to listen to time, or \[`keyboard::listen`\] which will notify you
of any keyboard events. But you can also create your own with \[`Subscription::run`\] and [`run_with`].

##### Scaling Applications

The `update`, `view`, and `Message` triplet composes very nicely.

A common pattern is to leverage this composability to split an
application into different screens:

````standalone_crate
# mod contacts {
#     use iced::{Element, Task};
#     pub struct Contacts;
#     impl Contacts {
#         pub fn update(&mut self, message: Message) -> Action { unimplemented!() }
#         pub fn view(&self) -> Element<Message> { unimplemented!() }
#     }
#     #[derive(Debug, Clone)]
#     pub enum Message {}
#     pub enum Action { None, Run(Task<Message>), Chat(()) }
# }
# mod conversation {
#     use iced::{Element, Task};
#     pub struct Conversation;
#     impl Conversation {
#         pub fn new(contact: ()) -> (Self, Task<Message>) { unimplemented!() }
#         pub fn update(&mut self, message: Message) -> Task<Message> { unimplemented!() }
#         pub fn view(&self) -> Element<Message> { unimplemented!() }
#     }
#     #[derive(Debug, Clone)]
#     pub enum Message {}
# }
use contacts::Contacts;
use conversation::Conversation;

use iced::{Element, Task};

struct State {
    screen: Screen,
}

enum Screen {
    Contacts(Contacts),
    Conversation(Conversation),
}

enum Message {
   Contacts(contacts::Message),
   Conversation(conversation::Message)
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::Contacts(message) => {
            if let Screen::Contacts(contacts) = &mut state.screen {
                let action = contacts.update(message);

                match action {
                    contacts::Action::None => Task::none(),
                    contacts::Action::Run(task) => task.map(Message::Contacts),
                    contacts::Action::Chat(contact) => {
                        let (conversation, task) = Conversation::new(contact);

                        state.screen = Screen::Conversation(conversation);

                        task.map(Message::Conversation)
                    }
                 }
            } else {
                Task::none()    
            }
        }
        Message::Conversation(message) => {
            if let Screen::Conversation(conversation) = &mut state.screen {
                conversation.update(message).map(Message::Conversation)
            } else {
                Task::none()    
            }
        }
    }
}

fn view(state: &State) -> Element<'_, Message> {
    match &state.screen {
        Screen::Contacts(contacts) => contacts.view().map(Message::Contacts),
        Screen::Conversation(conversation) => conversation.view().map(Message::Conversation),
    }
}
````

The `update` method of a screen can return an `Action` enum that can be leveraged by the parent to
execute a task or transition to a completely different screen altogether. The variants of `Action` can
have associated data. For instance, in the example above, the `Conversation` screen is created when
`Contacts::update` returns an `Action::Chat` with the selected contact.

Effectively, this approach lets you "tell a story" to connect different screens together in a type safe
way.

Furthermore, functor methods like \[`Task::map`\], \[`Element::map`\], and \[`Subscription::map`\] make composition
seamless.

[Elm]: https://elm-lang.org/
[the book]: https://book.iced.rs
[rows]: widget::Row
[columns]: widget::Column
[containers]: widget::Container
[Spacing]: widget::Column::spacing
[`Status`]: widget::button::Status
[`Palette`]: Theme::palette
[`palette`]: Theme::palette
[`extended_palette`]: Theme::extended_palette
[`container::rounded_box`]: widget::container::rounded_box
[`button::primary`]: widget::button::primary
[`text::danger`]: widget::text::danger
[`run_with`]: Subscription::run_with


### 4.1: Structs

#### 4.1.1: `struct iced::Application<P: iced_program::Program>`

```rust
pub struct Application<P: iced_program::Program> {}
```

_[Private fields hidden]_

The underlying definition and configuration of an iced application.

You can use this API to create and run iced applications
step by step—without coupling your logic to a trait
or a specific type.

You can create an \[`Application`\] with the \[`application`\] helper.

##### 4.1.1.1: `impl<P: iced_program::Program> iced::application::Application<P>`

###### 4.1.1.2.1: `fn run(self: Self) -> iced::Result`

```rust
pub fn run where
    Self: 'static,
    <P as iced_program::Program>::Message: iced_program::message::MaybeDebug + iced_program::message::MaybeClone(self: Self) -> iced::Result { ... }
```

Runs the \[`Application`\].

###### 4.1.1.2.2: `fn settings(self: Self, settings: iced_settings::Settings) -> Self`

Sets the \[`Settings`\] that will be used to run the \[`Application`\].

###### 4.1.1.2.3: `fn antialiasing(self: Self, antialiasing: bool) -> Self`

Sets the \[`Settings::antialiasing`\] of the \[`Application`\].

###### 4.1.1.2.4: `fn default_font(self: Self, default_font: iced_font::Font) -> Self`

Sets the default \[`Font`\] of the \[`Application`\].

###### 4.1.1.2.5: `fn font<impl impl Into<Cow<'static, [u8]>>: convert::Into<Cow<'static, [u8]>>>(self: Self, font: impl convert::Into<Cow<'static, [u8]>>) -> Self`

Adds a font to the list of fonts that will be loaded at the start of the \[`Application`\].

###### 4.1.1.2.6: `fn window(self: Self, window: iced_window::settings::Settings) -> Self`

Sets the \[`window::Settings`\] of the \[`Application`\].

Overwrites any previous \[`window::Settings`\].

###### 4.1.1.2.7: `fn centered(self: Self) -> Self`

Sets the \[`window::Settings::position`\] to \[`window::Position::Centered`\] in the \[`Application`\].

###### 4.1.1.2.8: `fn exit_on_close_request(self: Self, exit_on_close_request: bool) -> Self`

Sets the \[`window::Settings::exit_on_close_request`\] of the \[`Application`\].

###### 4.1.1.2.9: `fn window_size<impl impl Into<Size>: convert::Into<iced_size::Size>>(self: Self, size: impl convert::Into<iced_size::Size>) -> Self`

Sets the \[`window::Settings::size`\] of the \[`Application`\].

###### 4.1.1.2.10: `fn transparent(self: Self, transparent: bool) -> Self`

Sets the \[`window::Settings::transparent`\] of the \[`Application`\].

###### 4.1.1.2.11: `fn resizable(self: Self, resizable: bool) -> Self`

Sets the \[`window::Settings::resizable`\] of the \[`Application`\].

###### 4.1.1.2.12: `fn decorations(self: Self, decorations: bool) -> Self`

Sets the \[`window::Settings::decorations`\] of the \[`Application`\].

###### 4.1.1.2.13: `fn position(self: Self, position: iced_window::position::Position) -> Self`

Sets the \[`window::Settings::position`\] of the \[`Application`\].

###### 4.1.1.2.14: `fn level(self: Self, level: iced_window::level::Level) -> Self`

Sets the \[`window::Settings::level`\] of the \[`Application`\].

###### 4.1.1.2.15: `fn title<impl impl TitleFn<P::State>: iced::application::TitleFn<<P as iced_program::Program>::State>>(self: Self, title: impl iced::application::TitleFn<<P as iced_program::Program>::State>) -> iced::application::Application<impl iced_program::Program<State = <P as iced_program::Program>::State, Message = <P as iced_program::Program>::Message, Theme = <P as iced_program::Program>::Theme>>`

Sets the title of the \[`Application`\].

###### 4.1.1.2.16: `fn subscription<impl impl Fn(&P::State) -> Subscription<P::Message>: function::Fn<(&<P as iced_program::Program>::State) -> iced_futures::subscription::Subscription<<P as iced_program::Program>::Message>>>(self: Self, f: impl function::Fn<(&<P as iced_program::Program>::State) -> iced_futures::subscription::Subscription<<P as iced_program::Program>::Message>>) -> iced::application::Application<impl iced_program::Program<State = <P as iced_program::Program>::State, Message = <P as iced_program::Program>::Message, Theme = <P as iced_program::Program>::Theme>>`

Sets the subscription logic of the \[`Application`\].

###### 4.1.1.2.17: `fn theme<impl impl ThemeFn<P::State, P::Theme>: iced::application::ThemeFn<<P as iced_program::Program>::State, <P as iced_program::Program>::Theme>>(self: Self, f: impl iced::application::ThemeFn<<P as iced_program::Program>::State, <P as iced_program::Program>::Theme>) -> iced::application::Application<impl iced_program::Program<State = <P as iced_program::Program>::State, Message = <P as iced_program::Program>::Message, Theme = <P as iced_program::Program>::Theme>>`

Sets the theme logic of the \[`Application`\].

###### 4.1.1.2.18: `fn style<impl impl Fn(&P::State, &P::Theme) -> theme::Style: function::Fn<(&<P as iced_program::Program>::State, &<P as iced_program::Program>::Theme) -> iced_theme::Style>>(self: Self, f: impl function::Fn<(&<P as iced_program::Program>::State, &<P as iced_program::Program>::Theme) -> iced_theme::Style>) -> iced::application::Application<impl iced_program::Program<State = <P as iced_program::Program>::State, Message = <P as iced_program::Program>::Message, Theme = <P as iced_program::Program>::Theme>>`

Sets the style logic of the \[`Application`\].

###### 4.1.1.2.19: `fn scale_factor<impl impl Fn(&P::State) -> f32: function::Fn<(&<P as iced_program::Program>::State) -> f32>>(self: Self, f: impl function::Fn<(&<P as iced_program::Program>::State) -> f32>) -> iced::application::Application<impl iced_program::Program<State = <P as iced_program::Program>::State, Message = <P as iced_program::Program>::Message, Theme = <P as iced_program::Program>::Theme>>`

Sets the scale factor of the \[`Application`\].

###### 4.1.1.2.20: `fn executor<E>(self: Self) -> iced::application::Application<impl iced_program::Program<State = <P as iced_program::Program>::State, Message = <P as iced_program::Program>::Message, Theme = <P as iced_program::Program>::Theme>>`

```rust
pub fn executor<E> where E: iced_futures::executor::Executor(self: Self) -> iced::application::Application<impl iced_program::Program<State = <P as iced_program::Program>::State, Message = <P as iced_program::Program>::Message, Theme = <P as iced_program::Program>::Theme>> { ... }
```

Sets the executor of the \[`Application`\].

###### 4.1.1.2.21: `fn presets<impl impl IntoIterator<Item = Preset<P::State, P::Message>>: iter::traits::collect::IntoIterator<Item = iced_program::preset::Preset<<P as iced_program::Program>::State, <P as iced_program::Program>::Message>>>(self: Self, presets: impl iter::traits::collect::IntoIterator<Item = iced_program::preset::Preset<<P as iced_program::Program>::State, <P as iced_program::Program>::Message>>) -> Self`

Sets the boot presets of the \[`Application`\].

Presets can be used to override the default booting strategy
of your application during testing to create reproducible
environments.

##### 4.1.1.2: Trait Implementations for `Application`

**(Note: Does not implement common trait(s): `Display`, `ToString`, `error::Error`, `iced_futures::maybe::platform::MaybeSend`, `iced_futures::maybe::platform::MaybeSync`, `smol_str::ToSmolStr`, `wgpu_types::send_sync::WasmNotSend`, `wgpu_types::send_sync::WasmNotSendSync`, `wgpu_types::send_sync::WasmNotSync`)**

- `Debug`

- `!Send`
- `!Sync`

- `convert::From<T>`

#### 4.1.2: `struct iced::Daemon<P: iced_program::Program>`

```rust
pub struct Daemon<P: iced_program::Program> {}
```

_[Private fields hidden]_

The underlying definition and configuration of an iced daemon.

You can use this API to create and run iced applications
step by step—without coupling your logic to a trait
or a specific type.

You can create a \[`Daemon`\] with the \[`daemon`\] helper.

##### 4.1.2.1: `impl<P: iced_program::Program> iced::daemon::Daemon<P>`

###### 4.1.2.2.1: `fn run(self: Self) -> iced::Result`

```rust
pub fn run where
    Self: 'static,
    <P as iced_program::Program>::Message: iced_program::message::MaybeDebug + iced_program::message::MaybeClone(self: Self) -> iced::Result { ... }
```

Runs the \[`Daemon`\].

###### 4.1.2.2.2: `fn settings(self: Self, settings: iced_settings::Settings) -> Self`

Sets the \[`Settings`\] that will be used to run the \[`Daemon`\].

###### 4.1.2.2.3: `fn antialiasing(self: Self, antialiasing: bool) -> Self`

Sets the \[`Settings::antialiasing`\] of the \[`Daemon`\].

###### 4.1.2.2.4: `fn default_font(self: Self, default_font: iced_font::Font) -> Self`

Sets the default \[`Font`\] of the \[`Daemon`\].

###### 4.1.2.2.5: `fn font<impl impl Into<Cow<'static, [u8]>>: convert::Into<Cow<'static, [u8]>>>(self: Self, font: impl convert::Into<Cow<'static, [u8]>>) -> Self`

Adds a font to the list of fonts that will be loaded at the start of the \[`Daemon`\].

###### 4.1.2.2.6: `fn title<impl impl TitleFn<P::State>: iced::daemon::TitleFn<<P as iced_program::Program>::State>>(self: Self, title: impl iced::daemon::TitleFn<<P as iced_program::Program>::State>) -> iced::daemon::Daemon<impl iced_program::Program<State = <P as iced_program::Program>::State, Message = <P as iced_program::Program>::Message, Theme = <P as iced_program::Program>::Theme>>`

Sets the title of the \[`Daemon`\].

###### 4.1.2.2.7: `fn subscription<impl impl Fn(&P::State) -> Subscription<P::Message>: function::Fn<(&<P as iced_program::Program>::State) -> iced_futures::subscription::Subscription<<P as iced_program::Program>::Message>>>(self: Self, f: impl function::Fn<(&<P as iced_program::Program>::State) -> iced_futures::subscription::Subscription<<P as iced_program::Program>::Message>>) -> iced::daemon::Daemon<impl iced_program::Program<State = <P as iced_program::Program>::State, Message = <P as iced_program::Program>::Message, Theme = <P as iced_program::Program>::Theme>>`

Sets the subscription logic of the \[`Daemon`\].

###### 4.1.2.2.8: `fn theme<impl impl ThemeFn<P::State, P::Theme>: iced::daemon::ThemeFn<<P as iced_program::Program>::State, <P as iced_program::Program>::Theme>>(self: Self, f: impl iced::daemon::ThemeFn<<P as iced_program::Program>::State, <P as iced_program::Program>::Theme>) -> iced::daemon::Daemon<impl iced_program::Program<State = <P as iced_program::Program>::State, Message = <P as iced_program::Program>::Message, Theme = <P as iced_program::Program>::Theme>>`

Sets the theme logic of the \[`Daemon`\].

###### 4.1.2.2.9: `fn style<impl impl Fn(&P::State, &P::Theme) -> theme::Style: function::Fn<(&<P as iced_program::Program>::State, &<P as iced_program::Program>::Theme) -> iced_theme::Style>>(self: Self, f: impl function::Fn<(&<P as iced_program::Program>::State, &<P as iced_program::Program>::Theme) -> iced_theme::Style>) -> iced::daemon::Daemon<impl iced_program::Program<State = <P as iced_program::Program>::State, Message = <P as iced_program::Program>::Message, Theme = <P as iced_program::Program>::Theme>>`

Sets the style logic of the \[`Daemon`\].

###### 4.1.2.2.10: `fn scale_factor<impl impl Fn(&P::State, window::Id) -> f32: function::Fn<(&<P as iced_program::Program>::State, iced_window::id::Id) -> f32>>(self: Self, f: impl function::Fn<(&<P as iced_program::Program>::State, iced_window::id::Id) -> f32>) -> iced::daemon::Daemon<impl iced_program::Program<State = <P as iced_program::Program>::State, Message = <P as iced_program::Program>::Message, Theme = <P as iced_program::Program>::Theme>>`

Sets the scale factor of the \[`Daemon`\].

###### 4.1.2.2.11: `fn executor<E>(self: Self) -> iced::daemon::Daemon<impl iced_program::Program<State = <P as iced_program::Program>::State, Message = <P as iced_program::Program>::Message, Theme = <P as iced_program::Program>::Theme>>`

```rust
pub fn executor<E> where E: iced_futures::executor::Executor(self: Self) -> iced::daemon::Daemon<impl iced_program::Program<State = <P as iced_program::Program>::State, Message = <P as iced_program::Program>::Message, Theme = <P as iced_program::Program>::Theme>> { ... }
```

Sets the executor of the \[`Daemon`\].

###### 4.1.2.2.12: `fn presets<impl impl IntoIterator<Item = Preset<P::State, P::Message>>: iter::traits::collect::IntoIterator<Item = iced_program::preset::Preset<<P as iced_program::Program>::State, <P as iced_program::Program>::Message>>>(self: Self, presets: impl iter::traits::collect::IntoIterator<Item = iced_program::preset::Preset<<P as iced_program::Program>::State, <P as iced_program::Program>::Message>>) -> Self`

Sets the boot presets of the \[`Daemon`\].

Presets can be used to override the default booting strategy
of your application during testing to create reproducible
environments.

##### 4.1.2.2: Trait Implementations for `Daemon`

**(Note: Does not implement common trait(s): `Display`, `ToString`, `error::Error`, `iced_futures::maybe::platform::MaybeSend`, `iced_futures::maybe::platform::MaybeSync`, `smol_str::ToSmolStr`, `wgpu_types::send_sync::WasmNotSend`, `wgpu_types::send_sync::WasmNotSendSync`, `wgpu_types::send_sync::WasmNotSync`)**

- `Debug`

- `!Send`
- `!Sync`

- `convert::From<T>`

### 4.2: Enums

#### 4.2.1: `enum iced::Error`

```rust
pub enum Error {
    #[error("the futures executor could not be created")] ExecutorCreationFailed(io::error::Error),
    #[error("the application window could not be created")] WindowCreationFailed(Box<dyn error::Error + Send + Sync>),
    #[error("the application graphics context could not be created")] GraphicsCreationFailed(iced_graphics::error::Error),
}
```

An error that occurred while running an application.

##### 4.2.1.1: Variants

###### 4.2.1.1.1: `ExecutorCreationFailed(io::error::Error)`

The futures executor could not be created.

###### 4.2.1.1.2: `WindowCreationFailed(Box<dyn error::Error + Send + Sync>)`

The application window could not be created.

###### 4.2.1.1.3: `GraphicsCreationFailed(iced_graphics::error::Error)`

The application graphics context could not be created.

##### 4.2.1.2: Trait Implementations for `Error`

**(Note: Does not implement common trait(s): `iced_program::Program`)**

- `Debug`

- `convert::From<iced_winit::error::Error>`

- `Send`
- `Sync`

- `convert::From<T>`

### 4.3: Functions

#### 4.3.1: `fn application<State, Message, Theme, Renderer, impl impl BootFn<State, Message>: iced::application::BootFn<State, Message>, impl impl UpdateFn<State, Message>: iced::application::UpdateFn<State, Message>, impl impl for<'a> ViewFn<'a, State, Message, Theme, Renderer>: for<'a> iced::application::ViewFn<'a, State, Message, Theme, Renderer>>(boot: impl iced::application::BootFn<State, Message>, update: impl iced::application::UpdateFn<State, Message>, view: impl for<'a> iced::application::ViewFn<'a, State, Message, Theme, Renderer>) -> iced::application::Application<impl iced_program::Program<State = State, Message = Message, Theme = Theme>>`

```rust
pub fn application<State, Message, Theme, Renderer, impl impl BootFn<State, Message>: iced::application::BootFn<State, Message>, impl impl UpdateFn<State, Message>: iced::application::UpdateFn<State, Message>, impl impl for<'a> ViewFn<'a, State, Message, Theme, Renderer>: for<'a> iced::application::ViewFn<'a, State, Message, Theme, Renderer>>
  where
    State: 'static,
    Message: Send + 'static,
    Theme: iced_theme::Base,
    Renderer: iced_program::Renderer(boot: impl iced::application::BootFn<State, Message>, update: impl iced::application::UpdateFn<State, Message>, view: impl for<'a> iced::application::ViewFn<'a, State, Message, Theme, Renderer>) -> iced::application::Application<impl iced_program::Program<State = State, Message = Message, Theme = Theme>> { ... }
```

Creates an iced \[`Application`\] given its boot, update, and view logic.

###### Example

````no_run,standalone_crate
use iced::widget::{button, column, text, Column};

pub fn main() -> iced::Result {
    iced::application(u64::default, update, view).run()
}

#[derive(Debug, Clone)]
enum Message {
    Increment,
}

fn update(value: &mut u64, message: Message) {
    match message {
        Message::Increment => *value += 1,
    }
}

fn view(value: &u64) -> Column<Message> {
    column![
        text(value),
        button("+").on_press(Message::Increment),
    ]
}
````


#### 4.3.2: `fn daemon<State, Message, Theme, Renderer, impl impl application::BootFn<State, Message>: iced::application::BootFn<State, Message>, impl impl application::UpdateFn<State, Message>: iced::application::UpdateFn<State, Message>, impl impl for<'a> ViewFn<'a, State, Message, Theme, Renderer>: for<'a> iced::daemon::ViewFn<'a, State, Message, Theme, Renderer>>(boot: impl iced::application::BootFn<State, Message>, update: impl iced::application::UpdateFn<State, Message>, view: impl for<'a> iced::daemon::ViewFn<'a, State, Message, Theme, Renderer>) -> iced::daemon::Daemon<impl iced_program::Program<State = State, Message = Message, Theme = Theme>>`

```rust
pub fn daemon<State, Message, Theme, Renderer, impl impl application::BootFn<State, Message>: iced::application::BootFn<State, Message>, impl impl application::UpdateFn<State, Message>: iced::application::UpdateFn<State, Message>, impl impl for<'a> ViewFn<'a, State, Message, Theme, Renderer>: for<'a> iced::daemon::ViewFn<'a, State, Message, Theme, Renderer>>
  where
    State: 'static,
    Message: Send + 'static,
    Theme: iced_theme::Base,
    Renderer: iced_program::Renderer(boot: impl iced::application::BootFn<State, Message>, update: impl iced::application::UpdateFn<State, Message>, view: impl for<'a> iced::daemon::ViewFn<'a, State, Message, Theme, Renderer>) -> iced::daemon::Daemon<impl iced_program::Program<State = State, Message = Message, Theme = Theme>> { ... }
```

Creates an iced \[`Daemon`\] given its boot, update, and view logic.

A \[`Daemon`\] will not open a window by default, but will run silently
instead until a \[`Task`\] from \[`window::open`\] is returned by its update logic.

Furthermore, a \[`Daemon`\] will not stop running when all its windows are closed.
In order to completely terminate a \[`Daemon`\], its process must be interrupted or
its update logic must produce a \[`Task`\] from [`exit`].

[`exit`]: crate::exit


#### 4.3.3: `fn run<State, Message, Theme, Renderer, impl impl application::UpdateFn<State, Message> + 'static: iced::application::UpdateFn<State, Message> + 'static, impl impl for<'a> application::ViewFn<'a, State, Message, Theme, Renderer> + 'static: for<'a> iced::application::ViewFn<'a, State, Message, Theme, Renderer> + 'static>(update: impl iced::application::UpdateFn<State, Message> + 'static, view: impl for<'a> iced::application::ViewFn<'a, State, Message, Theme, Renderer> + 'static) -> iced::Result`

```rust
pub fn run<State, Message, Theme, Renderer, impl impl application::UpdateFn<State, Message> + 'static: iced::application::UpdateFn<State, Message> + 'static, impl impl for<'a> application::ViewFn<'a, State, Message, Theme, Renderer> + 'static: for<'a> iced::application::ViewFn<'a, State, Message, Theme, Renderer> + 'static>
  where
    State: default::Default + 'static,
    Message: Send + iced_program::message::MaybeDebug + iced_program::message::MaybeClone + 'static,
    Theme: iced_theme::Base + 'static,
    Renderer: iced_program::Renderer + 'static(update: impl iced::application::UpdateFn<State, Message> + 'static, view: impl for<'a> iced::application::ViewFn<'a, State, Message, Theme, Renderer> + 'static) -> iced::Result { ... }
```

Runs a basic iced application with default \[`Settings`\] given its update
and view logic.

This is equivalent to chaining \[`application()`\] with \[`Application::run`\].

###### Example

````no_run,standalone_crate
use iced::widget::{button, column, text, Column};

pub fn main() -> iced::Result {
    iced::run(update, view)
}

#[derive(Debug, Clone)]
enum Message {
    Increment,
}

fn update(value: &mut u64, message: Message) {
    match message {
        Message::Increment => *value += 1,
    }
}

fn view(value: &u64) -> Column<Message> {
    column![
        text(value),
        button("+").on_press(Message::Increment),
    ]
}
````


### 4.4: Type Aliases

#### 4.4.1: `type Element<'a, Message, Theme = iced_theme::Theme, Renderer = iced_renderer::Renderer>`

A generic widget.

This is an alias of an `iced_native` element with a default `Renderer`.


#### 4.4.2: `type Result`

The result of running an iced program.


## 5: Module: `iced::application`

Create and run iced applications step by step.

#### Example

````no_run,standalone_crate
use iced::widget::{button, column, text, Column};
use iced::Theme;

pub fn main() -> iced::Result {
    iced::application(u64::default, update, view)
        .theme(Theme::Dark)
        .centered()
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    Increment,
}

fn update(value: &mut u64, message: Message) {
    match message {
        Message::Increment => *value += 1,
    }
}

fn view(value: &u64) -> Column<Message> {
    column![
        text(value),
        button("+").on_press(Message::Increment),
    ]
}
````


### 5.1: Traits

#### 5.1.1: `trait iced::application::BootFn<State, Message>`

```rust
pub trait BootFn<State, Message> {
    pub fn boot(self: &Self) -> (State, iced_runtime::task::Task<Message>);;
}
```

The logic to initialize the `State` of some \[`Application`\].

This trait is implemented for both `Fn() -> State` and
`Fn() -> (State, Task<Message>)`.

In practice, this means that \[`application`\] can both take
simple functions like `State::default` and more advanced ones
that return a \[`Task`\].

##### 5.1.1.1: Required Methods

###### 5.1.1.1.1: `fn boot(self: &Self) -> (State, iced_runtime::task::Task<Message>)`

Initializes the \[`Application`\] state.

##### 5.1.1.2: Implementors

###### 5.1.1.2.1: `impl<T, C, State, Message> iced::application::BootFn<State, Message> for T`

```rust
where
    T: function::Fn<() -> C>,
    C: iced::application::IntoBoot<State, Message>
```


#### 5.1.2: `trait iced::application::IntoBoot<State, Message>`

```rust
pub trait IntoBoot<State, Message> {
    pub fn into_boot(self: Self) -> (State, iced_runtime::task::Task<Message>);;
}
```

The initial state of some \[`Application`\].

##### 5.1.2.1: Required Methods

###### 5.1.2.1.1: `fn into_boot(self: Self) -> (State, iced_runtime::task::Task<Message>)`

Turns some type into the initial state of some \[`Application`\].

##### 5.1.2.2: Implementors

###### 5.1.2.2.1: `impl<State, Message> iced::application::IntoBoot<State, Message> for State`

###### 5.1.2.2.2: `impl<State, Message> iced::application::IntoBoot<State, Message> for (State, iced_runtime::task::Task<Message>)`


#### 5.1.3: `trait iced::application::ThemeFn<State, Theme>`

```rust
pub trait ThemeFn<State, Theme> {
    pub fn theme(self: &Self, state: &State) -> option::Option<Theme>;;
}
```

The theme logic of some \[`Application`\].

Any implementors of this trait can be provided as an argument to
\[`Application::theme`\].

`iced` provides two implementors:

* the built-in \[`Theme`\] itself
* and any `Fn(&State) -> impl Into<Option<Theme>>`.

##### 5.1.3.1: Required Methods

###### 5.1.3.1.1: `fn theme(self: &Self, state: &State) -> option::Option<Theme>`

Returns the theme of the \[`Application`\] for the current state.

If `None` is returned, `iced` will try to use a theme that
matches the system color scheme.

##### 5.1.3.2: Implementors

###### 5.1.3.2.1: `impl<State> iced::application::ThemeFn<State, iced_theme::Theme> for iced_theme::Theme`

###### 5.1.3.2.2: `impl<F, T, State, Theme> iced::application::ThemeFn<State, Theme> for F`

```rust
where
    F: function::Fn<(&State) -> T>,
    T: convert::Into<option::Option<Theme>>
```


#### 5.1.4: `trait iced::application::TitleFn<State>`

```rust
pub trait TitleFn<State> {
    pub fn title(self: &Self, state: &State) -> String;;
}
```

The title logic of some \[`Application`\].

This trait is implemented both for `&static str` and
any closure `Fn(&State) -> String`.

This trait allows the \[`application`\] builder to take any of them.

##### 5.1.4.1: Required Methods

###### 5.1.4.1.1: `fn title(self: &Self, state: &State) -> String`

Produces the title of the \[`Application`\].

##### 5.1.4.2: Implementors

###### 5.1.4.2.1: `impl<State> iced::application::TitleFn<State> for &'static str`

###### 5.1.4.2.2: `impl<T, State> iced::application::TitleFn<State> for T`

```rust
where T: function::Fn<(&State) -> String>
```


#### 5.1.5: `trait iced::application::UpdateFn<State, Message>`

```rust
pub trait UpdateFn<State, Message> {
    pub fn update(self: &Self, state: &mut State, message: Message) -> iced_runtime::task::Task<Message>;;
}
```

The update logic of some \[`Application`\].

This trait allows the \[`application`\] builder to take any closure that
returns any `Into<Task<Message>>`.

##### 5.1.5.1: Required Methods

###### 5.1.5.1.1: `fn update(self: &Self, state: &mut State, message: Message) -> iced_runtime::task::Task<Message>`

Processes the message and updates the state of the \[`Application`\].

##### 5.1.5.2: Implementors

###### 5.1.5.2.1: `impl<State> iced::application::UpdateFn<State, convert::Infallible> for ()`

###### 5.1.5.2.2: `impl<T, State, Message, C> iced::application::UpdateFn<State, Message> for T`

```rust
where
    T: function::Fn<(&mut State, Message) -> C>,
    C: convert::Into<iced_runtime::task::Task<Message>>
```


#### 5.1.6: `trait iced::application::ViewFn<'a, State, Message, Theme, Renderer>`

```rust
pub trait ViewFn<'a, State, Message, Theme, Renderer> {
    pub fn view(self: &Self, state: &'a State) -> iced::Element<'a, Message, Theme, Renderer>;;
}
```

The view logic of some \[`Application`\].

This trait allows the \[`application`\] builder to take any closure that
returns any `Into<Element<'_, Message>>`.

##### 5.1.6.1: Required Methods

###### 5.1.6.1.1: `fn view(self: &Self, state: &'a State) -> iced::Element<'a, Message, Theme, Renderer>`

Produces the widget of the \[`Application`\].

##### 5.1.6.2: Implementors

###### 5.1.6.2.1: `impl<'a, T, State, Message, Theme, Renderer, Widget> iced::application::ViewFn<'a, State, Message, Theme, Renderer> for T`

```rust
where
    T: function::Fn<(&'a State) -> Widget>,
    State: 'static,
    Widget: convert::Into<iced::Element<'a, Message, Theme, Renderer>>
```


### 5.2: Functions

#### 5.2.1: `fn timed<State, Message, Theme, Renderer, impl impl BootFn<State, Message>: iced::application::BootFn<State, Message>, impl impl UpdateFn<State, Message>: iced::application::timed::UpdateFn<State, Message>, impl impl Fn(&State) -> Subscription<Message>: function::Fn<(&State) -> iced_futures::subscription::Subscription<Message>>, impl impl for<'a> ViewFn<'a, State, Message, Theme, Renderer>: for<'a> iced::application::ViewFn<'a, State, Message, Theme, Renderer>>(boot: impl iced::application::BootFn<State, Message>, update: impl iced::application::timed::UpdateFn<State, Message>, subscription: impl function::Fn<(&State) -> iced_futures::subscription::Subscription<Message>>, view: impl for<'a> iced::application::ViewFn<'a, State, Message, Theme, Renderer>) -> iced::application::Application<impl iced_program::Program<State = State, Message = (Message, time::Instant), Theme = Theme>>`

```rust
pub fn timed<State, Message, Theme, Renderer, impl impl BootFn<State, Message>: iced::application::BootFn<State, Message>, impl impl UpdateFn<State, Message>: iced::application::timed::UpdateFn<State, Message>, impl impl Fn(&State) -> Subscription<Message>: function::Fn<(&State) -> iced_futures::subscription::Subscription<Message>>, impl impl for<'a> ViewFn<'a, State, Message, Theme, Renderer>: for<'a> iced::application::ViewFn<'a, State, Message, Theme, Renderer>>
  where
    State: 'static,
    Message: Send + 'static,
    Theme: iced_theme::Base + 'static,
    Renderer: iced_program::Renderer + 'static(boot: impl iced::application::BootFn<State, Message>, update: impl iced::application::timed::UpdateFn<State, Message>, subscription: impl function::Fn<(&State) -> iced_futures::subscription::Subscription<Message>>, view: impl for<'a> iced::application::ViewFn<'a, State, Message, Theme, Renderer>) -> iced::application::Application<impl iced_program::Program<State = State, Message = (Message, time::Instant), Theme = Theme>> { ... }
```

Creates an \[`Application`\] with an `update` function that also
takes the \[`Instant`\] of each `Message`.

This constructor is useful to create animated applications that
are *pure* (e.g. without relying on side-effect calls like \[`Instant::now`\]).

Purity is needed when you want your application to end up in the
same exact state given the same history of messages. This property
enables proper time traveling debugging with [`comet`].

[`comet`]: https://github.com/iced-rs/comet


### 5.3: Re-exports

- `fn application<State, Message, Theme, Renderer, impl impl BootFn<State, Message>: iced::application::BootFn<State, Message>, impl impl UpdateFn<State, Message>: iced::application::UpdateFn<State, Message>, impl impl for<'a> ViewFn<'a, State, Message, Theme, Renderer>: for<'a> iced::application::ViewFn<'a, State, Message, Theme, Renderer>>(boot: impl iced::application::BootFn<State, Message>, update: impl iced::application::UpdateFn<State, Message>, view: impl for<'a> iced::application::ViewFn<'a, State, Message, Theme, Renderer>) -> iced::application::Application<impl iced_program::Program<State = State, Message = Message, Theme = Theme>>` (See section 4.3.1: for details)
- `struct iced::application::Application<P: iced_program::Program>` (See section 4.1.1: for details)


## 6: Module: `iced::application::timed`

An \[`Application`\] that receives an \[`Instant`\] in update logic.


### 6.1: Traits

#### 6.1.1: `trait iced::application::timed::UpdateFn<State, Message>`

```rust
pub trait UpdateFn<State, Message> {
    pub fn update(self: &Self, state: &mut State, message: Message, now: time::Instant) -> impl convert::Into<iced_runtime::task::Task<Message>>;;
}
```

The update logic of some timed \[`Application`\].

This is like [`application::UpdateFn`](super::UpdateFn),
but it also takes an \[`Instant`\].

##### 6.1.1.1: Required Methods

###### 6.1.1.1.1: `fn update(self: &Self, state: &mut State, message: Message, now: time::Instant) -> impl convert::Into<iced_runtime::task::Task<Message>>`

Processes the message and updates the state of the \[`Application`\].

##### 6.1.1.2: Implementors

###### 6.1.1.2.1: `impl<State, Message> iced::application::timed::UpdateFn<State, Message> for ()`

###### 6.1.1.2.2: `impl<T, State, Message, C> iced::application::timed::UpdateFn<State, Message> for T`

```rust
where
    T: function::Fn<(&mut State, Message, time::Instant) -> C>,
    C: convert::Into<iced_runtime::task::Task<Message>>
```


### 6.2: Re-exports

- `fn timed<State, Message, Theme, Renderer, impl impl BootFn<State, Message>: iced::application::BootFn<State, Message>, impl impl UpdateFn<State, Message>: iced::application::timed::UpdateFn<State, Message>, impl impl Fn(&State) -> Subscription<Message>: function::Fn<(&State) -> iced_futures::subscription::Subscription<Message>>, impl impl for<'a> ViewFn<'a, State, Message, Theme, Renderer>: for<'a> iced::application::ViewFn<'a, State, Message, Theme, Renderer>>(boot: impl iced::application::BootFn<State, Message>, update: impl iced::application::timed::UpdateFn<State, Message>, subscription: impl function::Fn<(&State) -> iced_futures::subscription::Subscription<Message>>, view: impl for<'a> iced::application::ViewFn<'a, State, Message, Theme, Renderer>) -> iced::application::Application<impl iced_program::Program<State = State, Message = (Message, time::Instant), Theme = Theme>>` (See section 5.2.1: for details)


## 7: Module: `iced::clipboard`

Access the clipboard.


## 8: Module: `iced::daemon`

Create and run daemons that run in the background.


### 8.1: Traits

#### 8.1.1: `trait iced::daemon::ThemeFn<State, Theme>`

```rust
pub trait ThemeFn<State, Theme> {
    pub fn theme(self: &Self, state: &State, window: iced_window::id::Id) -> option::Option<Theme>;;
}
```

The theme logic of some \[`Daemon`\].

Any implementors of this trait can be provided as an argument to
\[`Daemon::theme`\].

`iced` provides two implementors:

* the built-in \[`Theme`\] itself
* and any `Fn(&State, window::Id) -> impl Into<Option<Theme>>`.

##### 8.1.1.1: Required Methods

###### 8.1.1.1.1: `fn theme(self: &Self, state: &State, window: iced_window::id::Id) -> option::Option<Theme>`

Returns the theme of the \[`Daemon`\] for the current state and window.

If `None` is returned, `iced` will try to use a theme that
matches the system color scheme.

##### 8.1.1.2: Implementors

###### 8.1.1.2.1: `impl<State> iced::daemon::ThemeFn<State, iced_theme::Theme> for iced_theme::Theme`

###### 8.1.1.2.2: `impl<F, T, State, Theme> iced::daemon::ThemeFn<State, Theme> for F`

```rust
where
    F: function::Fn<(&State, iced_window::id::Id) -> T>,
    T: convert::Into<option::Option<Theme>>
```


#### 8.1.2: `trait iced::daemon::TitleFn<State>`

```rust
pub trait TitleFn<State> {
    pub fn title(self: &Self, state: &State, window: iced_window::id::Id) -> String;;
}
```

The title logic of some \[`Daemon`\].

This trait is implemented both for `&static str` and
any closure `Fn(&State, window::Id) -> String`.

This trait allows the \[`daemon`\] builder to take any of them.

##### 8.1.2.1: Required Methods

###### 8.1.2.1.1: `fn title(self: &Self, state: &State, window: iced_window::id::Id) -> String`

Produces the title of the \[`Daemon`\].

##### 8.1.2.2: Implementors

###### 8.1.2.2.1: `impl<State> iced::daemon::TitleFn<State> for &'static str`

###### 8.1.2.2.2: `impl<T, State> iced::daemon::TitleFn<State> for T`

```rust
where T: function::Fn<(&State, iced_window::id::Id) -> String>
```


#### 8.1.3: `trait iced::daemon::ViewFn<'a, State, Message, Theme, Renderer>`

```rust
pub trait ViewFn<'a, State, Message, Theme, Renderer> {
    pub fn view(self: &Self, state: &'a State, window: iced_window::id::Id) -> iced::Element<'a, Message, Theme, Renderer>;;
}
```

The view logic of some \[`Daemon`\].

This trait allows the \[`daemon`\] builder to take any closure that
returns any `Into<Element<'_, Message>>`.

##### 8.1.3.1: Required Methods

###### 8.1.3.1.1: `fn view(self: &Self, state: &'a State, window: iced_window::id::Id) -> iced::Element<'a, Message, Theme, Renderer>`

Produces the widget of the \[`Daemon`\].

##### 8.1.3.2: Implementors

###### 8.1.3.2.1: `impl<'a, T, State, Message, Theme, Renderer, Widget> iced::daemon::ViewFn<'a, State, Message, Theme, Renderer> for T`

```rust
where
    T: function::Fn<(&'a State, iced_window::id::Id) -> Widget>,
    State: 'static,
    Widget: convert::Into<iced::Element<'a, Message, Theme, Renderer>>
```


### 8.2: Re-exports

- `fn daemon<State, Message, Theme, Renderer, impl impl application::BootFn<State, Message>: iced::application::BootFn<State, Message>, impl impl application::UpdateFn<State, Message>: iced::application::UpdateFn<State, Message>, impl impl for<'a> ViewFn<'a, State, Message, Theme, Renderer>: for<'a> iced::daemon::ViewFn<'a, State, Message, Theme, Renderer>>(boot: impl iced::application::BootFn<State, Message>, update: impl iced::application::UpdateFn<State, Message>, view: impl for<'a> iced::daemon::ViewFn<'a, State, Message, Theme, Renderer>) -> iced::daemon::Daemon<impl iced_program::Program<State = State, Message = Message, Theme = Theme>>` (See section 4.3.2: for details)
- `struct iced::daemon::Daemon<P: iced_program::Program>` (See section 4.1.2: for details)


## 9: Module: `iced::debug`

Debug your applications.


## 10: Module: `iced::event`

Handle events of a user interface.


## 11: Module: `iced::executor`

Choose your preferred executor to power your application.


## 12: Module: `iced::font`

Load and use fonts.


## 13: Module: `iced::keyboard`

Listen and react to keyboard events.


## 14: Module: `iced::mouse`

Listen and react to mouse events.


## 15: Module: `iced::overlay`

Display interactive elements on top of other widgets.


### 15.1: Type Aliases

#### 15.1.1: `type Element<'a, Message, Theme = iced_renderer::Renderer, Renderer = iced_renderer::Renderer>`

A generic overlay.

This is an alias of an [`overlay::Element`] with a default `Renderer`.

[`overlay::Element`]: crate::core::overlay::Element


## 16: Module: `iced::system`

Retrieve system information.


## 17: Module: `iced::task`

Create runtime tasks.


## 18: Module: `iced::time`

Listen and react to time.


### 18.1: Functions

#### 18.1.1: `fn now() -> iced_runtime::task::Task<time::Instant>`

Returns a \[`Task`\] that produces the current \[`Instant`\]
by calling \[`Instant::now`\].

While you can call \[`Instant::now`\] directly in your application;
that renders your application "impure" (i.e. no referential transparency).

You may care about purity if you want to leverage the `time-travel`
feature properly.


## 19: Module: `iced::touch`

Listen and react to touch events.


## 20: Module: `iced::widget`

Use the built-in widgets or create your own.


## 21: Module: `iced::window`

Configure the window of your application in native platforms.


## 22: Module: `iced::window::icon`

Attach an icon to the window of your application.


### 22.1: Enums

#### 22.1.1: `enum iced::window::icon::Error`

```rust
pub enum Error {
    #[error("The icon is invalid: {0}")] InvalidError(#[from] iced_window::icon::Error),
    #[error("The underlying OS failed to create the window icon: {0}")] OsError(#[from] io::error::Error),
}
```

An error produced when creating an \[`Icon`\].

##### 22.1.1.1: Variants

###### 22.1.1.1.1: `InvalidError(iced_window::icon::Error)`

The \[`Icon`\] is not valid.

###### 22.1.1.1.2: `OsError(io::error::Error)`

The underlying OS failed to create the icon.

##### 22.1.1.2: Trait Implementations for `Error`

**(Note: Does not implement common trait(s): `iced_program::Program`)**

- `Debug`

- `convert::From<iced_window::icon::Error>`
- `convert::From<io::error::Error>`

- `Send`
- `Sync`

- `convert::From<T>`

## 23: Examples Appendix

### Examples

**Iced moves fast and the `master` branch can contain breaking changes!** If you want to browse examples that are compatible with the latest release,
then [switch to the `latest` branch](https://github.com/iced-rs/iced/tree/latest/examples#examples).

#### [Tour](tour)

A simple UI tour that can run both on native platforms and the web! It showcases different widgets that can be built using Iced.

The **[`main`](tour/src/main.rs)** file contains all the code of the example! All the cross-platform GUI is defined in terms of **state**, **messages**, **update logic** and **view logic**.

<div align="center">
  <a href="https://iced.rs/examples/tour.mp4">
    <img src="https://iced.rs/examples/tour.gif">
  </a>
</div>

You can run the native version with `cargo run`:

````
cargo run --package tour
````

#### [Todos](todos)

A todos tracker inspired by [TodoMVC]. It showcases dynamic layout, text input, checkboxes, scrollables, icons, and async actions! It automatically saves your tasks in the background, even if you did not finish typing them.

The example code is located in the **[`main`](todos/src/main.rs)** file.

<div align="center">
  <a href="https://iced.rs/examples/todos.mp4">
    <img src="https://iced.rs/examples/todos.gif" height="400px">
  </a>
</div>

You can run the native version with `cargo run`:

````
cargo run --package todos
````

#### [Game of Life](game_of_life)

An interactive version of the [Game of Life], invented by [John Horton Conway].

It runs a simulation in a background thread while allowing interaction with a `Canvas` that displays an infinite grid with zooming, panning, and drawing support.

The relevant code is located in the **[`main`](game_of_life/src/main.rs)** file.

<div align="center">
  <img src="https://iced.rs/examples/game_of_life.gif">
</div>

You can run it with `cargo run`:

````
cargo run --package game_of_life
````

#### [Styling](styling)

An example showcasing custom styling with a light and dark theme.

The example code is located in the **[`main`](styling/src/main.rs)** file.

<div align="center">
  <img src="https://iced.rs/examples/styling.gif">
</div>

You can run it with `cargo run`:

````
cargo run --package styling
````

#### Extras

A bunch of simpler examples exist:

* [`bezier_tool`](bezier_tool), a Paint-like tool for drawing Bézier curves using the `Canvas` widget.
* [`clock`](clock), an application that uses the `Canvas` widget to draw a clock and its hands to display the current time.
* [`color_palette`](color_palette), a color palette generator based on a user-defined root color.
* [`counter`](counter), the classic counter example explained in the [`README`](../README.md).
* [`custom_widget`](custom_widget), a demonstration of how to build a custom widget that draws a circle.
* [`download_progress`](download_progress), a basic application that asynchronously downloads a dummy file of 100 MB and tracks the download progress.
* [`events`](events), a log of native events displayed using a conditional `Subscription`.
* [`geometry`](geometry), a custom widget showcasing how to draw geometry with the `Mesh2D` primitive in [`iced_wgpu`](../wgpu).
* [`integration`](integration), a demonstration of how to integrate Iced in an existing [`wgpu`] application.
* [`pane_grid`](pane_grid), a grid of panes that can be split, resized, and reorganized.
* [`pick_list`](pick_list), a dropdown list of selectable options.
* [`pokedex`](pokedex), an application that displays a random Pokédex entry (sprite included!) by using the [PokéAPI].
* [`progress_bar`](progress_bar), a simple progress bar that can be filled by using a slider.
* [`scrollable`](scrollable), a showcase of various scrollable content configurations.
* [`sierpinski_triangle`](sierpinski_triangle), a [sierpiński triangle](https://en.wikipedia.org/wiki/Sierpi%C5%84ski_triangle) Emulator, use `Canvas` and `Slider`.
* [`solar_system`](solar_system), an animated solar system drawn using the `Canvas` widget and showcasing how to compose different transforms.
* [`stopwatch`](stopwatch), a watch with start/stop and reset buttons showcasing how to listen to time.
* [`svg`](svg), an application that renders the [Ghostscript Tiger] by leveraging the `Svg` widget.

All of them are packaged in their own crate and, therefore, can be run using `cargo`:

````
cargo run --package <example>
````

#### [Coffee]

Since [Iced was born in May 2019], it has been powering the user interfaces in
[Coffee], an experimental 2D game engine.

<div align="center">
  <img src="https://iced.rs/examples/coffee.gif">
</div>


[TodoMVC]: http://todomvc.com/
[Game of Life]: https://en.wikipedia.org/wiki/Conway%27s_Game_of_Life
[John Horton Conway]: https://en.wikipedia.org/wiki/John_Horton_Conway
[`wgpu`]: https://github.com/gfx-rs/wgpu
[PokéAPI]: https://pokeapi.co/
[Ghostscript Tiger]: https://commons.wikimedia.org/wiki/File:Ghostscript_Tiger.svg
[Coffee]: https://github.com/hecrj/coffee
[Iced was born in May 2019]: https://github.com/hecrj/coffee/pull/35
