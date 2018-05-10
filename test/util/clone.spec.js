'use strict';

const clone = require('../../lib/util/clone');
const tmpPath = createFilePath('clone');

describe('clone repo when it exists', () => {

  beforeEach(() => {
    return removeFolder(tmpPath);
  });

  describe('test clone function', () => {
    it('clone', async () => {
      const results = await clone(repoUrl, tmpPath);
      for (let entry in results) {
        expect(repoUrl).to.equal(entry);
      }
    });
  });
});
