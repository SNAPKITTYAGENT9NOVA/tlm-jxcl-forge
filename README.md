# tlm-jxcl-forge
TLM JXCL PURE RAW DENSE ISA FORGE - 3500-line implementation specification

## redis-implementation-js

A small Node.js/Express demo illustrating Redis-backed HTTP response caching, merged in from the separate `Redis-implementation-js` repository. See [`redis-implementation-js/`](./redis-implementation-js) for the code:

- `server.js` — plain Express server (port 3000) that fetches https://jsonplaceholder.typicode.com/photos on every request.
- `server-cached.js` — same idea (port 3001) but backed by Redis (`redis://127.0.0.1:6379`), caching the response for 1 hour to demonstrate cache-hit vs cache-miss latency.

Run with `node redis-implementation-js/server.js` or `node redis-implementation-js/server-cached.js` (the cached variant needs a local Redis instance).
