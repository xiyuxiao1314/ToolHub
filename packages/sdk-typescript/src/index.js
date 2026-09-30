/**
 * Minimal ToolHub client — line JSON-RPC to local daemon (stdio or HTTP proxy).
 * @typedef {{jsonrpc:string,id:number|string,method:string,params?:unknown}} JsonRpcRequest
 */
export class ToolhubClient {
  /**
   * @param {(req: JsonRpcRequest) => Promise<any>} transport
   */
  constructor(transport) {
    this.transport = transport;
    this.nextId = 1;
  }

  /**
   * @param {string} method
   * @param {any} [params]
   */
  async call(method, params = {}) {
    const id = this.nextId++;
    const res = await this.transport({
      jsonrpc: '2.0',
      id,
      method,
      params,
    });
    if (res && res.error) {
      const code = res.error?.data?.error_code || 'internal_error';
      throw new Error(`${code}: ${res.error.message}`);
    }
    return res?.result ?? null;
  }

  status() {
    return this.call('status');
  }

  search(query) {
    return this.call('registry.search', { query });
  }

  resolveCapability(capability, opts = {}) {
    return this.call('resolve.capability', { capability, ...opts });
  }

  scan(mode = 'quick') {
    return this.call('scan.start', { mode });
  }
}

export function nodeStdioTransport(child) {
  /** @type {Map<number, (v:any)=>void>} */
  const pending = new Map();
  let buf = '';
  child.stdout.setEncoding('utf8');
  child.stdout.on('data', (chunk) => {
    buf += chunk;
    let idx;
    while ((idx = buf.indexOf('\n')) >= 0) {
      const line = buf.slice(0, idx);
      buf = buf.slice(idx + 1);
      if (!line.trim()) continue;
      try {
        const msg = JSON.parse(line);
        const resolve = pending.get(msg.id);
        if (resolve) {
          pending.delete(msg.id);
          resolve(msg);
        }
      } catch {
        /* ignore partial */
      }
    }
  });
  return (req) =>
    new Promise((resolve, reject) => {
      pending.set(req.id, resolve);
      child.stdin.write(JSON.stringify(req) + '\n', (err) => {
        if (err) reject(err);
      });
    });
}
