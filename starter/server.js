// A small server for the starter: every page answers in HTML, Markdown or JSON, whichever the request asks for.
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join, normalize } from "node:path";
import index from "./dist/index.mjs";
import show from "./dist/coffee/show.mjs";
import { coffees } from "./data.js";

const types = { ".css": "text/css", ".svg": "image/svg+xml" };

function send(res, output, accept) {
  const { contentType, body } = output.negotiate(accept);
  res.writeHead(200, { "Content-Type": contentType, Vary: "Accept" });
  res.end(body);
}

createServer(async (req, res) => {
  const url = new URL(req.url, "http://localhost");
  const accept = req.headers.accept;

  if (req.method === "GET" && url.pathname === "/") return send(res, index.render({ coffees }), accept);

  const coffee = coffees.find((c) => url.pathname === `/coffee/${c.slug}`);
  if (req.method === "GET" && coffee) return send(res, show.render({ coffee }), accept);

  // forms and actions: a real shop saves them; the starter goes back to where they came from
  if (req.method === "POST") {
    res.writeHead(303, { Location: req.headers.referer ?? "/" });
    return res.end();
  }

  const file = normalize(join("public", url.pathname));
  if (file.startsWith(join("public", "/")) && types[extname(file)]) {
    try {
      const body = await readFile(file);
      res.writeHead(200, { "Content-Type": types[extname(file)] });
      return res.end(body);
    } catch {}
  }
  res.writeHead(404, { "Content-Type": "text/plain" });
  res.end("Not found");
}).listen(process.env.PORT ?? 3000, () => console.log(`http://localhost:${process.env.PORT ?? 3000}`));
