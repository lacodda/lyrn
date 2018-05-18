const download = require('../../lib/commands/download').api;
const exec = require('../../lib/util/exec');
const { access } = require('../../lib/util/fs');

describe('download repo when it exists', () => {
  const tmpPath = createFilePath('download');

  beforeEach(() => removeFolder(tmpPath));

  describe('api', () => {
    it('can download', async () => {
      try {
        const result = await download(repoUrl, tmpPath);
        expect(repoUrl).to.equal(result);
      } catch (error) {
        assert.isNotOk('download', error);
      }
    });

    it('can remove .git folder', async () => {
      try {
        await download(repoUrl, tmpPath);
        try {
          await access(tmpPath + '/.git');
          assert.isNotOk('download', '.git folder was exists');
        } catch (error) {
          expect(error.code).to.equal('ENOENT');
        }
      } catch (error) {
        assert.isNotOk('download', error);
      }
    });
  });

  describe('cli', () => {
    it('can download', async () => {
      try {
        const result = await exec(`./bin/lyrn download ${repoUrl} ${tmpPath}`);
        expect(`Repository ${repoUrl} successfully downloaded\n`).to.equal(result);
      } catch (error) {
        assert.isNotOk('download', error);
      }
    });
  });
});
