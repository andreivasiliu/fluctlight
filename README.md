# Fluctlight

Fluctlight is a playground for testing random ideas for speed improvements on a
chat server with the [Matrix](matrix.org) protocol.

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

## Features

Currently implemented:
* All non-networking logic is bundled in a module, which is automatically
  reloaded at runtime whenever cargo builds a new library
* Requests use a per-request memory pool to store ephemeral strings and lists
* Deserialized requests and response structures use borrowed data wherever
  possible

For planned features, see [DESIGN.md](./DESIGN.md).
