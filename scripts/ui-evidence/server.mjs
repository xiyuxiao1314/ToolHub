#!/usr/bin/env node
/**
 * Serve the production ToolHub desktop UI with a Tauri invoke shim
 * backed by a live toolhubd stdio process. Used only for nine-page UI evidence.
 */
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawn } from 'node:child_process';
import readline from 'node:readline';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(__dirname, '../..');
const dist = path.join(repo, 'apps/desktop/ui/dist');
const toolhubd = process.env.TOOLHUBD_BIN || path.join(repo, 'target/debug/toolhubd.exe');
const registry = process.env.TOOLHUB_REGISTRY || path.join(process.env.TEMP || '.', `toolhub-ui-evidence-${process.pid}.sqlite`);
const port = Number(process.env.UI_EVIDENCE_PORT || 18765);

if (!fs.existsSync(path.join(dist, 'index.html'))) {
  console.error('ui/dist missing; run npm run build in apps/desktop/ui first');
  process.exit(1);
}

const child = spawn(toolhubd, [], {
  env: { ...process.env, TOOLHUB_REGISTRY: registry },
  stdio: ['pipe', 'pipe', 'ignore'],
});
const rl = readline.createInterface({ input: child.stdout });
const pending = new Map();
rl.on('line', (line) => {
  try {
    const msg = JSON.parse(line);
    if (msg.id != null && pending.has(String(msg.id))) {
      const { resolve } = pending.get(String(msg.id));
      pending.delete(String(msg.id));
      resolve(msg);
    }
  } catch {
    /* ignore non-JSON */
  }
});

function rpc(method, params) {
  const id = String(Date.now() + Math.random());
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    child.stdin.write(JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n');
    setTimeout(() => {
      if (pending.has(id)) {
        pending.delete(id);
        reject(new Error(`rpc timeout: ${method}`));
      }
    }, 120000);
  });
}

const mime = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.woff2': 'font/woff2',
};

const server = http.createServer(async (req, res) => {
  const url = new URL(req.url, `http://127.0.0.1:${port}`);
  if (url.pathname === '/api/invoke') {
    let body = '';
    for await (const chunk of req) body += chunk;
    try {
      const { method, params } = JSON.parse(body || '{}');
      if (method === 'app_versions') {
        res.writeHead(200, { 'content-type': 'application/json' });
        res.end(JSON.stringify({ app: '0.2.0', core: '0.2.0', protocol: '1.0' }));
        return;
      }
      const result = await rpc(method, params ?? {});
      res.writeHead(200, { 'content-type': 'application/json' });
      res.end(JSON.stringify(result.result ?? result));
    } catch (error) {
      res.writeHead(500, { 'content-type': 'application/json' });
      res.end(JSON.stringify({ error: String(error) }));
    }
    return;
  }

  let file = url.pathname === '/' ? '/index.html' : url.pathname;
  const abs = path.join(dist, file);
  if (!abs.startsWith(dist) || !fs.existsSync(abs) || fs.statSync(abs).isDirectory()) {
    res.writeHead(404).end('not found');
    return;
  }
  const ext = path.extname(abs);
  res.writeHead(200, { 'content-type': mime[ext] || 'application/octet-stream' });
  res.end(fs.readFileSync(abs));
});

const shim = `
<script>
window.__TAURI_INTERNALS__ = {
  invoke: async (cmd, args) => {
    const method = cmd === 'rpc' ? (args && args.method) : cmd;
    const params = cmd === 'rpc' ? (args && args.params) : (args || {});
    const res = await fetch('/api/invoke', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ method, params }),
    });
    const data = await res.json();
    if (!res.ok || data?.error) {
      const message = typeof data?.error === 'string' ? data.error : JSON.stringify(data?.error || res.statusText);
      throw new Error(message);
    }
    return data;
  },
};
</script>
`;

// Wrap index.html with the shim once at boot via middleware replacement.
const indexHtml = fs.readFileSync(path.join(dist, 'index.html'), 'utf8').replace('</head>', `${shim}</head>`);
const serverWrap = http.createServer(async (req, res) => {
  const url = new URL(req.url, `http://127.0.0.1:${port}`);
  if (url.pathname === '/api/invoke') {
    return server.emit('request', req, res);
  }
  if (url.pathname === '/' || url.pathname === '/index.html') {
    res.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
    res.end(indexHtml);
    return;
  }
  return server.emit('request', req, res);
});

serverWrap.listen(port, '127.0.0.1', () => {
  console.log(`ui-evidence listening http://127.0.0.1:${port}`);
  console.log(`registry=${registry}`);
  console.log(`toolhubd=${toolhubd}`);
});

process.on('exit', () => child.kill());
