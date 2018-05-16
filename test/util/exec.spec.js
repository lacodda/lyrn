const exec = require('../../lib/util/exec');

describe('test exec function', () => {
  it('exec worked', async () => {
    try {
      const stdout = await exec(`${lyrn} --version`);
      expect(stdout).to.equal(`${pkg.version}\n`);
    } catch (error) {
      assert.isNotOk('exec', 'this will fail');
    }
  });

  it('catch error in exec', async () => {
    try {
      const stdout = await exec(`${lyrn} --not-a-param`);
      expect(stdout).to.equal(`${pkg.version}\n`);
    } catch (error) {
      assert.isOk('exec', 'error was caught');
    }
  });
});
