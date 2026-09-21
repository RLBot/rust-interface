<img src="https://raw.githubusercontent.com/RLBot/rust-interface/refs/heads/master/assets/RLBotRustLogoText.svg" alt="RLBot" width="400">

This is a Rust interface for the [RLBot] v5 socket api. RLBot is a framework
for creating offline Rocket League bots. This crate lets you write bots
using a simple, safe interface that should feel comfortable to Rust
developers.

[RLBot]: https://rlbot.org/

All of the types in the [`flat`] module are generated from the
[flatbuffers spec]. Documentation for these types are carried over from the
spec. For further documentation about how RLBot works, see the resources
linked on the [RLBot Wiki].

[flatbuffers spec]: https://github.com/RLBot/flatbuffers-schema
[RLBot Wiki]: https://wiki.rlbot.org/

The two main different ways of using this crate follows:
- The [`agents`] API - This is a **higher-level** interface. The
  [run_x_agent] functions initializes an agent for you. Relevant examples:
  [atba_agent, atba_hivemind, high_jump_script].
- [`RLBotConnection`] – This is a **lower-level** wrapper around the actual
  tcp connection to [core] (RLBotServer). It allows you to use
  [`send_packet`] and [`recv_packet`] to manually communicate with RLBot.
  For documentation on how to do this, refer to the [socket specification].
  Relevant examples: [start_match, stop_match, packet_logger and atba_raw]

[run_x_agent]: https://docs.rs/rlbot/latest/rlbot/agents/#functions
[atba_agent, atba_hivemind, high_jump_script]: https://github.com/RLBot/rust-interface/tree/master/rlbot/examples
[`send_packet`]: https://docs.rs/rlbot/latest/rlbot/struct.RLBotConnection.html#method.send_packet
[`recv_packet`]: https://docs.rs/rlbot/latest/rlbot/struct.RLBotConnection.html#method.recv_packet
[socket specification]: https://wiki.rlbot.org/v5/framework/sockets-specification/
[start_match, stop_match, packet_logger and atba_raw]: https://github.com/RLBot/rust-interface/tree/master/rlbot/examples
[core]: https://github.com/RLBot/core
