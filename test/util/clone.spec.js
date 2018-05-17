const clone = require('../../lib/util/clone');

describe('clone repo when it exists', () => {
  const tmpPath = createFilePath('clone');

  beforeEach(() => removeFolder(tmpPath));

  describe('test clone function', () => {
    it('can clone', async () => {
      try {
        const result = await clone(repoUrl, tmpPath);
        expect(repoUrl).to.equal(result);
      } catch (error) {
        assert.isNotOk('clone', 'clone failed');
      }
    });

    it('catch error in clone', async () => {
      try {
        const result = await clone(repoUrl + 'badExt', tmpPath);
        expect(repoUrl).to.equal(result);
      } catch (error) {
        assert.isOk('clone', 'error was caught');
      }
    });
  });
});
