# Fluctlight

Fluctlight is a playground for testing random ideas for speed improvements on a
chat server with the [Matrix](https://matrix.org) protocol.

The goal is to implement a server with _extreme_ performance (i.e. the Nginx of
Matrix servers), using as many tricks as I can possibly think of, including:
* As few allocations/copying as possible
* Intern all state strings
* Group allocations in cache-friendly arenas
* Allow bits of JSON objects to be pre-serialized
* Keep all state in memory
* Allow room servers to be on separate hosts, with a router proxy in front

This server will only ever implement the [federation APIs] of the Matrix
protocol, with no support for clients, and will never become a real server.

[federation APIs]: https://spec.matrix.org/latest/server-server-api/

For a real server, use [Conduit] instead. Note that in the world of Matrix
servers, performance is less important than memory usage, which is not the goal
of Fluctlight, but is the goal of Conduit.

[Conduit]: https://gitlab.com/famedly/conduit

## Status

The server is currently not usable for anything.

Incoming APIs:
* Prototype /key/server and /key/query

Outgoing APIs:
* Prototype /make_join and /send_join

## Features

Currently implemented:
* All non-networking logic is bundled in a module, which is automatically
  reloaded at runtime whenever cargo builds a new library
* Requests use a per-request memory pool to store ephemeral strings and lists
* Deserialized requests and response structures use borrowed data wherever
  possible
* Requests use canned (pre-rendered) JSON snippets as part of the response
* Canonical JSON using a borrowing and sorting version of serde_json::Value
* Hashes and signatures computed without storing canonical JSONs, by piping via
  serde straight to a sha256 writer sink
* An admin HTML page to show information using compiled [Askama] templates

For planned features, see [DESIGN.md](./DESIGN.md).

[Askama]: https://lib.rs/crates/askama

## License

The code is dual-licensed under either Apache-2.0 or MIT.
