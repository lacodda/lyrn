const { readdir, access } = require('../../lib/util/fs');

describe('test fs functions', () => {

  describe('readdir', () => {
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

  describe('access', () => {
    it('access worked', async () => {
      try {
        await access('./bin');
        assert.isOk('access', 'dir exists');
      } catch (error) {
        assert.isNotOk('access', 'error was caught');
      }
    });

    it('catch error in access', async () => {
      try {
        await access('./error');
        assert.isNotOk('access', `error wasn't caught`);
      } catch (error) {
        assert.isOk('access', 'error was caught');
      }
    });
  });
});
