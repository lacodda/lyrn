const clone = require('../../lib/util/clone');

describe('clone repo when it exists', () => {
  const tmpPath = createFilePath('clone');

  beforeEach(() => removeFolder(tmpPath));

  describe('test clone function', () => {
    it('clone', async () => {
      const results = await clone(repoUrl, tmpPath);
      expect(repoUrl).to.equal(results);
    });
  });
});
