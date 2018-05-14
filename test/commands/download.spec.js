const download = require('../../lib/commands/download').api;
const exec = require('../../lib/util/exec');
const tmpPath = createFilePath('download');

describe('download repo when it exists', () => {

  beforeEach(() => {
    return removeFolder(tmpPath);
  });

  describe('cli', () => {
    it('can download', async () => {
      const { stdout, stderr } = await exec(`./bin/lyrn download ${repoUrl} ${tmpPath}`);

      expect(`Repository ${repoUrl} successfully downloaded\n`).to.equal(stdout);
    });
  });

  describe('api', () => {
    it('can download', async () => {
      const results = await download(repoUrl, tmpPath);
      expect(repoUrl).to.equal(results);
    });
  });
});
