'use strict';

const exec = require('../../lib/util/exec');

describe('test exec function', () => {
  it('exec', async () => {
    const { stdout } = await exec(`${lyrn} --version`);
    expect(stdout).to.equal(`${pkg.version}\n`);
  });
});
