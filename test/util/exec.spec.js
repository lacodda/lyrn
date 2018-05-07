'use strict';

const exec = require('../../lib/util/exec');

describe('test exec function', () => {
  it('exec', async () => {
    const result = await exec(`${lyrn} --version`);
    expect(result).to.equal(`${pkg.version}\n`);
  });
});
