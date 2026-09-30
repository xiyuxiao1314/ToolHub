export declare class ToolhubClient {
  constructor(transport: (req: unknown) => Promise<any>);
  call(method: string, params?: unknown): Promise<any>;
  status(): Promise<any>;
  search(query: string): Promise<any>;
  resolveCapability(capability: string, opts?: Record<string, unknown>): Promise<any>;
  scan(mode?: string): Promise<any>;
}
export declare function nodeStdioTransport(child: any): (req: unknown) => Promise<any>;
