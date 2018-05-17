const { readdir } = require('../../lib/util/fs');

describe('test fs functions', () => {
  it('readdir worked', async () => {
    try {
      const result = await readdir(`./bin`);
      expect(result).to.deep.equal(['lyrn']);
    } catch (error) {
      assert.isNotOk('readdir', 'this will fail');
    }
  });

  it('catch error in readdir', async () => {
    try {
      const result = await readdir(`./bin`);
      expect(result).to.deep.equal(['error']);
    } catch (error) {
      assert.isOk('readdir', 'error was caught');
    }
  });
});
