import { TextDecoder, TextEncoder } from "node:util";

import fetchMock from "jest-fetch-mock";

// jsdom omits these WHATWG globals that Node and browsers both provide.
Object.assign(globalThis, {
  TextEncoder: globalThis.TextEncoder ?? TextEncoder,
  TextDecoder: globalThis.TextDecoder ?? TextDecoder,
});

fetchMock.enableMocks();
