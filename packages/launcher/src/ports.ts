import { createServer } from "node:net";

/** Resolves a free TCP port, preferring `preferred` when it is available. */
export function findFreePort(preferred = 0): Promise<number> {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.unref();
    server.once("error", () => {
      if (preferred !== 0) {
        // Preferred port busy — fall back to an ephemeral one.
        findFreePort(0).then(resolve, reject);
        return;
      }
      reject(new Error("no free port available"));
    });
    server.listen({ port: preferred, host: "127.0.0.1" }, () => {
      const address = server.address();
      if (address === null || typeof address === "string") {
        server.close();
        reject(new Error("could not determine port"));
        return;
      }
      const { port } = address;
      server.close(() => resolve(port));
    });
  });
}
