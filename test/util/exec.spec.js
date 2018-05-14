'use strict';

const exec = require('../../lib/util/exec');
const execNew = require('../../lib/util/exec').new;

describe('test exec function', () => {
  it('exec', async () => {
    const { stdout } = await exec(`${lyrn} --version`);
    expect(stdout).to.equal(`${pkg.version}\n`);
  });
});

describe('test New exec function', () => {
  it('exec worked', async () => {
    try {
      const stdout = await execNew(`${lyrn} --version`);
      expect(stdout).to.equal(`${pkg.version}\n`);
    } catch (error) {
      assert.isNotOk('exec', 'this will fail');
    }
  });

  it('catch error in exec', async () => {
    try {
      const stdout = await execNew(`${lyrn} --not-a-param`);
      expect(stdout).to.equal(`${pkg.version}\n`);
    } catch (error) {
      assert.isOk('exec', 'error was caught');
    }
  });
});
