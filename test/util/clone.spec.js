'use strict';
const assert = require('assert');
const fse = require('fs-extra');

const path = require('path');

const clone = require('../../lib/util/clone');

const TMP_PATH = path.join(__dirname, '..', 'tmp');
const OUT_PATH = path.join(TMP_PATH, 'clone');
const REPO_URL = 'https://github.com/lacodda/test.git';

describe('clone repo when it exists', () => {

  beforeEach(() => {
    return fse.remove(OUT_PATH).catch(err => {
      console.log(err);

      throw err;
    });
  });

  describe('api', () => {
    it('can clone', async () => {
      const results = await clone(REPO_URL, OUT_PATH);
      for (let entry in results) {
        assert.equal(REPO_URL, entry);
      }
    });
  });
});
