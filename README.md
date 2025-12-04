# lazy-sparql-result-reader

A fast SPARQL results parser for JavaScript and TypeScript, compiled from Rust via WebAssembly.  
It reads streamed SPARQL query results and calls a callback for each parsed batch of bindings.

---

## Features

- Processes streaming SPARQL results efficiently.
- Calls a JavaScript callback for each batch of parsed bindings.
- Written in Rust for speed and reliability.
- Fully compatible with TypeScript.

---

## Installation

Install via npm:

```bash
npm install lazy-sparql-result-reader
```

## Usage Example

```ts
import init, { read } from "sparql-stream-parser";

// Initialize the WASM module
await init();

// Suppose you have a ReadableStream of SPARQL results
const stream: ReadableStream = getSparqlStream();

// Process the stream with a callback
await read(stream, 100, (bindings) => {
    console.log("Received batch:", bindings);
});
```

## License

This project is licensed under the **MIT** License.

You are free to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the software, under the conditions of the MIT License.

For full details, see the [LICENSE](./LICENSE) file.
