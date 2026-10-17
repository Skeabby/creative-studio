import { describe, expect, it } from "vitest";
import { checkRequest } from "../src/guard.js";

const PORT = 5174;
const TOKEN = "abc123";
const opts = { port: PORT, token: TOKEN };

function req(headers: Record<string, string | undefined>, method = "GET") {
  return { headers, method } as any;
}

describe("request guard", () => {
  it("allows a same-host GET with a bearer token", () => {
    expect(
      checkRequest(
        req({ host: `127.0.0.1:${PORT}`, authorization: `Bearer ${TOKEN}` }),
        opts,
      ),
    ).toBeNull();
  });

  it("allows localhost host too", () => {
    expect(
      checkRequest(req({ host: `localhost:${PORT}`, authorization: `Bearer ${TOKEN}` }), opts),
    ).toBeNull();
  });

  it("rejects a bad host", () => {
    expect(checkRequest(req({ host: "evil.com", authorization: `Bearer ${TOKEN}` }), opts)).toEqual(
      { status: 403, error: "bad host" },
    );
  });

  it("rejects a cross-origin request even with the right token", () => {
    expect(
      checkRequest(
        req({ host: `127.0.0.1:${PORT}`, origin: "http://evil.com", authorization: `Bearer ${TOKEN}` }),
        opts,
      ),
    ).toEqual({ status: 403, error: "bad origin" });
  });

  it("rejects a non-JSON content-type on a write", () => {
    expect(
      checkRequest(
        req(
          {
            host: `127.0.0.1:${PORT}`,
            authorization: `Bearer ${TOKEN}`,
            "content-type": "text/plain",
          },
          "POST",
        ),
        opts,
      ),
    ).toEqual({ status: 415, error: "bad content-type" });
  });

  it("allows a JSON write with the token", () => {
    expect(
      checkRequest(
        req(
          {
            host: `127.0.0.1:${PORT}`,
            authorization: `Bearer ${TOKEN}`,
            "content-type": "application/json",
          },
          "POST",
        ),
        opts,
      ),
    ).toBeNull();
  });

  it("rejects a missing token", () => {
    expect(checkRequest(req({ host: `127.0.0.1:${PORT}` }), opts)).toEqual({
      status: 401,
      error: "missing token",
    });
  });

  it("rejects a wrong token", () => {
    expect(
      checkRequest(req({ host: `127.0.0.1:${PORT}`, authorization: "Bearer nope" }), opts),
    ).toEqual({ status: 401, error: "missing token" });
  });

  it("accepts the nt cookie in place of a bearer", () => {
    expect(
      checkRequest(req({ host: `127.0.0.1:${PORT}`, cookie: `nt=${TOKEN}` }), opts),
    ).toBeNull();
  });

  it("disables the token check when token is empty", () => {
    expect(checkRequest(req({ host: `127.0.0.1:${PORT}` }), { port: PORT, token: "" })).toBeNull();
  });
});
