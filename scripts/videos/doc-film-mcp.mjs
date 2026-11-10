// [DOC-FILMS-LIVE] Print the result of an actual MCP exchange for the camera.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';

const child = spawn('deslop-mcp', ['--root', '.'], { stdio: ['pipe', 'pipe', 'ignore'] });
const lines = createInterface({ input: child.stdout })[Symbol.asyncIterator]();
let id = 0;

function send(message) {
  child.stdin.write(JSON.stringify({ jsonrpc: '2.0', ...message }) + '\n');
}

async function request(method, params) {
  send({ id: ++id, method, params });
  for (let line = await lines.next(); !line.done; line = await lines.next()) {
    const response = JSON.parse(line.value);
    if (response.id !== id) continue;
    assert.ok(response.result, JSON.stringify(response.error));
    return response.result;
  }
  throw new Error('MCP closed before ' + method);
}

try {
  const session = await request('initialize', { protocolVersion: '2024-11-05',
    capabilities: {}, clientInfo: { name: 'Deslop documentation', version: '1' } });
  send({ method: 'notifications/initialized' });
  console.log('Connected:', session.serverInfo.name, session.serverInfo.version);
  const catalogue = await request('tools/list', {});
  console.log('Tools:', catalogue.tools.map(({ name }) => name).join(', '));
  const name = process.argv[2] ?? 'find-similar';
  const args = name === 'find-similar'
    ? { language: 'csharp', snippet: fs.readFileSync('Eta.cs', 'utf8'), limit: 2 }
    : { detail: 'summary', limit: 2 };
  console.log('Query:', name, '(limit 2)');
  const result = await request('tools/call', { name, arguments: args });
  assert.ok(!result.isError, JSON.stringify(result.content));
  const data = result.structuredContent ?? JSON.parse(result.content[0].text);
  for (const field of ['total', 'total_clusters', 'totalClusters', 'clusters', 'existing']) {
    const value = data[field];
    if (value !== undefined) console.log(field + ':', Array.isArray(value) ? value.length : value);
  }
} finally {
  child.stdin.end();
}
