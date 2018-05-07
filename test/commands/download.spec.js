'use strict';

const download = require('../../lib/commands/download').api;
const tmpPath = createFilePath('download');

describe('download repo when it exists', () => {

  beforeEach(() => {
    return removeFolder(tmpPath);
  });

  describe('cli', () => {
    it('can download', (done) => {
      let buffer = '';

      const child = spawn('node', [
        './bin/lyrn', 'download', repoUrl, tmpPath,
      ], { cwd: __dirname + '/../..' });

      child.stdout.on('data', (b) => {
        buffer += b.toString();
      });

      child.on('close', () => {
        expect(`Repository ${repoUrl} successfully downloaded\n`).to.equal(buffer);
        done();
      });
    });
  });

  describe('api', () => {
    it('can download', async () => {
      const results = await download(repoUrl, tmpPath);
      for (let entry in results) {
        expect(repoUrl).to.equal(entry);
      }
    });
  });
});
