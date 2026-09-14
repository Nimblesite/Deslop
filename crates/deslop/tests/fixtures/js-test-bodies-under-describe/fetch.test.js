import assert from 'assert';
import { startServer } from './support/server.js';
import fetchAdapter from '../lib/adapters/fetch.js';

describe('progress', () => {
  describe('upload', function () {
    it('should support upload progress capturing', async function () {
      this.timeout(15000);
      const server = await startServer({ port: 0 });
      const samples = [];
      const payload = Buffer.alloc(1024 * 1024, 'x');
      const response = await fetchAdapter.post(server.url, payload, {
        onUploadProgress(event) {
          samples.push(event.loaded);
        },
      });
      assert.strictEqual(response.status, 200);
      assert.ok(samples.length > 0, 'progress was sampled');
      assert.strictEqual(samples[samples.length - 1], payload.length);
      await server.close();
    });
  });

  describe('download', function () {
    it('should support download progress capturing', async function () {
      this.timeout(15000);
      const server = await startServer({ port: 0 });
      const samples = [];
      const payload = Buffer.alloc(1024 * 1024, 'x');
      const response = await fetchAdapter.get(server.url, {
        onDownloadProgress(event) {
          samples.push(event.loaded);
        },
      });
      assert.strictEqual(response.status, 200);
      assert.ok(samples.length > 0, 'progress was sampled');
      assert.strictEqual(samples[samples.length - 1], payload.length);
      await server.close();
    });
  });
});
