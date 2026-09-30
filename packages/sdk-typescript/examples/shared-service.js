// Run: node packages/sdk-typescript/examples/shared-service.js C:/path/to/toolhub.exe
import { ToolhubClient, nodeCliTransport } from '../src/index.js';
const client=new ToolhubClient(nodeCliTransport({executable:process.argv[2] || 'toolhub'}));
try { await client.negotiate();console.log(await client.status());console.log(await client.search('python')); } finally { client.close(); }
