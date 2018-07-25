const { join, resolve } = require('path');
const { ensureDir, remove } = require('fs-extra');
const download = require('../../lib/commands/download').api;
const exec = require('../../lib/util/exec');
const { access } = require('../../lib/util/fs');

describe('download repo when it exists', () => {
  const targetPath = getTmpPath('download');

  beforeEach(async () => {
    await remove(tmpPath);
    await ensureDir(tmpPath);
  });

  describe('api', () => {
    it('can download', async () => {
      try {
        const result = await download(repoUrl, targetPath);
        expect(repoUrl).to.equal(result);
      } catch (error) {
        assert.isNotOk('download', error);
      }
    });

    it('can remove .git folder', async () => {
      try {
        await download(repoUrl, targetPath);
        try {
          await access(resolve(join(targetPath, '.git')));
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
        const result = await exec(`${bin} download ${repoUrl} ${targetPath}`);
        expect(`Repository ${repoUrl} successfully downloaded\n`).to.equal(
          result
        );
      } catch (error) {
        assert.isNotOk('download', error);
      }
    });
  });
});
